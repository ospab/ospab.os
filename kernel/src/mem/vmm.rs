//! Virtual Memory Manager for ospabOS v0.1.5
//! Implements 4-level paging (PML4 -> PDPT -> PD -> PT) with user/kernel separation

use spin::Mutex;
use x86_64::{
    structures::paging::{
        FrameAllocator as X64FrameAllocator, Mapper, OffsetPageTable, Page, PageTable, PageTableFlags, PhysFrame,
        Size4KiB,
    },
    PhysAddr, VirtAddr,
};

use crate::mem::physical::FRAME_ALLOCATOR;
use crate::boot;

/// Wrapper to make FrameAllocator compatible with x86_64::structures::paging::FrameAllocator
struct FrameAllocatorWrapper;

unsafe impl X64FrameAllocator<Size4KiB> for FrameAllocatorWrapper {
    fn allocate_frame(&mut self) -> Option<PhysFrame<Size4KiB>> {
        let frame_addr = FRAME_ALLOCATOR.lock().allocate()?;
        Some(PhysFrame::containing_address(PhysAddr::new(frame_addr as u64)))
    }
}

impl FrameAllocatorWrapper {
    fn new() -> Self {
        FrameAllocatorWrapper
    }
}

/// Virtual address space boundaries
pub const USER_SPACE_START: u64 = 0x0000_0000_0000_0000;
pub const USER_SPACE_END: u64 = 0x0000_7FFF_FFFF_FFFF; // 128 TB user space
pub const KERNEL_SPACE_START: u64 = 0xFFFF_8000_0000_0000; // HHDM base
pub const KERNEL_HEAP_START: u64 = 0xFFFF_FFFF_8000_0000;
pub const KERNEL_HEAP_SIZE: u64 = 32 * 1024 * 1024; // 32 MB

/// Page Table Entry flags for user/kernel pages
pub const USER_PAGE_FLAGS: PageTableFlags = PageTableFlags::PRESENT
    .union(PageTableFlags::WRITABLE)
    .union(PageTableFlags::USER_ACCESSIBLE);

pub const KERNEL_PAGE_FLAGS: PageTableFlags = PageTableFlags::PRESENT
    .union(PageTableFlags::WRITABLE);

/// Address Space - represents a virtual address space with its own page table
pub struct AddressSpace {
    /// Physical address of the PML4 (root page table)
    pub cr3: PhysAddr,
    /// Cached mapper for this address space
    mapper: Option<OffsetPageTable<'static>>,
}

impl AddressSpace {
    /// Create a new address space with empty page tables
    pub fn new() -> Result<Self, &'static str> {
        // Get HHDM offset
        let hhdm = boot::hhdm_offset().ok_or("HHDM offset not available")?;
        
        // Allocate a frame for PML4
        let frame = {
            let mut allocator = FRAME_ALLOCATOR.lock();
            allocator.allocate().ok_or("Failed to allocate frame for PML4")?
        };

        let pml4_addr = PhysAddr::new(frame as u64);

        // Zero out the PML4
        unsafe {
            let pml4_ptr = (pml4_addr.as_u64() + hhdm) as *mut PageTable;
            (*pml4_ptr).zero();
        }

        Ok(Self {
            cr3: pml4_addr,
            mapper: None,
        })
    }

    /// Get a mapper for this address space
    pub fn mapper(&mut self) -> &mut OffsetPageTable<'static> {
        if self.mapper.is_none() {
            let hhdm = boot::hhdm_offset().unwrap_or(0);
            unsafe {
                let pml4_ptr =
                    (self.cr3.as_u64() + hhdm) as *mut PageTable;
                let pml4 = &mut *pml4_ptr;
                self.mapper = Some(OffsetPageTable::new(
                    pml4,
                    VirtAddr::new(hhdm),
                ));
            }
        }
        self.mapper.as_mut().unwrap()
    }

    /// Map a virtual page to a physical frame with given flags
    pub fn map_page(
        &mut self,
        page: Page<Size4KiB>,
        frame: PhysFrame<Size4KiB>,
        flags: PageTableFlags,
    ) -> Result<(), &'static str> {
        let mapper = self.mapper();
        let mut frame_allocator = FrameAllocatorWrapper::new();

        unsafe {
            mapper
                .map_to(page, frame, flags, &mut frame_allocator)
                .map_err(|_| "Failed to map page")?
                .flush();
        }

        Ok(())
    }

    /// Allocate and map a range of virtual pages
    pub fn allocate_pages(
        &mut self,
        start: VirtAddr,
        count: usize,
        flags: PageTableFlags,
    ) -> Result<(), &'static str> {
        for i in 0..count {
            let virt_addr = start + (i as u64 * 4096);
            let page = Page::<Size4KiB>::containing_address(virt_addr);

            // Allocate a physical frame
            let frame_addr = FRAME_ALLOCATOR
                .lock()
                .allocate()
                .ok_or("Out of physical memory")?;
            let frame = PhysFrame::containing_address(PhysAddr::new(frame_addr as u64));

            // Map it
            let mapper = self.mapper();
            let mut frame_allocator = FrameAllocatorWrapper::new();
            unsafe {
                mapper
                    .map_to(page, frame, flags, &mut frame_allocator)
                    .map_err(|_| "Failed to map page")?
                    .flush();
            }
        }

        Ok(())
    }

    /// Unmap a virtual page
    pub fn unmap_page(&mut self, page: Page<Size4KiB>) -> Result<(), &'static str> {
        let mapper = self.mapper();

        mapper
            .unmap(page)
            .map_err(|_| "Failed to unmap page")?
            .1
            .flush();

        Ok(())
    }

    /// Switch to this address space (load CR3)
    pub unsafe fn switch_to(&self) {
        x86_64::registers::control::Cr3::write(
            PhysFrame::containing_address(self.cr3),
            x86_64::registers::control::Cr3Flags::empty(),
        );
    }

    /// Clone kernel mappings into this address space
    pub fn clone_kernel_mappings(&mut self) -> Result<(), &'static str> {
        // Get current PML4 (kernel's)
        let (kernel_pml4_frame, _) = x86_64::registers::control::Cr3::read();
        let kernel_pml4_addr = kernel_pml4_frame.start_address();
        let hhdm = boot::hhdm_offset().ok_or("HHDM offset not available")?;

        unsafe {
            let kernel_pml4_ptr =
                (kernel_pml4_addr.as_u64() + hhdm) as *const PageTable;
            let kernel_pml4 = &*kernel_pml4_ptr;

            let new_pml4_ptr =
                (self.cr3.as_u64() + hhdm) as *mut PageTable;
            let new_pml4 = &mut *new_pml4_ptr;

            // Copy upper half (kernel space) entries from kernel PML4
            // Entries 256-511 are kernel space (0xFFFF_8000_0000_0000 and above)
            for i in 256..512 {
                // Copy the raw entry value (physical address + flags)
                let entry_value = kernel_pml4[i].addr().as_u64() | kernel_pml4[i].flags().bits();
                new_pml4[i].set_addr(
                    PhysAddr::new(entry_value & 0x000F_FFFF_FFFF_F000),
                    kernel_pml4[i].flags()
                );
            }
        }

        Ok(())
    }
}

/// Global VMM instance
pub static VMM: Mutex<Option<VirtualMemoryManager>> = Mutex::new(None);

/// Virtual Memory Manager - manages address spaces and allocations
pub struct VirtualMemoryManager {
    /// Kernel address space
    kernel_space: AddressSpace,
    /// Next user heap address for sys_malloc
    next_user_heap: VirtAddr,
}

impl VirtualMemoryManager {
    /// Initialize the VMM with the current kernel page table
    pub fn init() -> Result<(), &'static str> {
        // Get current PML4 from CR3
        let (pml4_frame, _) = x86_64::registers::control::Cr3::read();
        let pml4_addr = pml4_frame.start_address();

        let kernel_space = AddressSpace {
            cr3: pml4_addr,
            mapper: None,
        };

        let vmm = VirtualMemoryManager {
            kernel_space,
            next_user_heap: VirtAddr::new(0x0000_4000_0000_0000), // Start at 64 TB
        };

        *VMM.lock() = Some(vmm);
        Ok(())
    }

    /// Allocate user memory (for sys_malloc)
    pub fn allocate_user_memory(
        &mut self,
        size: usize,
        address_space: &mut AddressSpace,
    ) -> Result<VirtAddr, &'static str> {
        if size == 0 {
            return Err("Cannot allocate 0 bytes");
        }

        // Round up to page size
        let pages = (size + 4095) / 4096;

        let start_addr = self.next_user_heap;
        self.next_user_heap += pages as u64 * 4096;

        // Allocate pages in the given address space
        address_space.allocate_pages(start_addr, pages, USER_PAGE_FLAGS)?;

        Ok(start_addr)
    }

    /// Create a new user address space with kernel mappings
    pub fn create_user_address_space(&self) -> Result<AddressSpace, &'static str> {
        let mut space = AddressSpace::new()?;
        space.clone_kernel_mappings()?;
        Ok(space)
    }

    /// Get reference to kernel address space
    pub fn kernel_space(&mut self) -> &mut AddressSpace {
        &mut self.kernel_space
    }
}

/// Serial debug output
fn serial_print(msg: &[u8]) {
    for &byte in msg {
        unsafe {
            while x86_64::instructions::port::Port::<u8>::new(0x3F8 + 5)
                .read()
                & 0x20
                == 0
            {}
            x86_64::instructions::port::Port::<u8>::new(0x3F8).write(byte);
        }
    }
}

/// Initialize the Virtual Memory Manager
pub fn init() -> Result<(), &'static str> {
    serial_print(b"[VMM] Initializing Virtual Memory Manager...\r\n");
    VirtualMemoryManager::init()?;
    serial_print(b"[VMM] VMM initialized successfully\r\n");
    Ok(())
}
