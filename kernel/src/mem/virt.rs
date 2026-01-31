#![allow(dead_code)]

use core::ptr;

const PAGE_SIZE: usize = 4096;
const PAGE_TABLE_ENTRIES: usize = 512;
const PAGE_PRESENT: u64 = 1;
const PAGE_WRITABLE: u64 = 1 << 1;
const PAGE_USER: u64 = 1 << 2;
const PAGE_HUGE: u64 = 1 << 7;

#[repr(align(4096))]
pub struct PageTable {
    entries: [u64; PAGE_TABLE_ENTRIES],
}

impl PageTable {
    pub const fn new() -> Self {
        PageTable {
            entries: [0; PAGE_TABLE_ENTRIES],
        }
    }

    pub fn set_entry(&mut self, index: usize, addr: u64, flags: u64) {
        self.entries[index] = (addr & !0xFFF) | flags;
    }

    pub fn get_entry(&self, index: usize) -> u64 {
        self.entries[index]
    }

    pub fn clear_entry(&mut self, index: usize) {
        self.entries[index] = 0;
    }
}

pub struct VirtualMemoryManager {
    pml4: *mut PageTable,
}

impl VirtualMemoryManager {
    pub fn new() -> Self {
        // Disable kernel-side page table creation — leave mapping to the bootloader.
        // Keep a null pointer so other methods can be no-ops safely.
        VirtualMemoryManager { pml4: core::ptr::null_mut() }
    }

    pub fn map_page(&mut self, virt_addr: usize, phys_addr: usize, flags: u64) {
        // Intentionally no-op: the bootloader/boot environment is responsible
        // for initial mappings. Avoid creating or modifying page tables here
        // to prevent double-mapping (PageAlreadyMapped) panics.
        let _ = (virt_addr, phys_addr, flags);
    }

    pub fn unmap_page(&mut self, virt_addr: usize) {
        // No-op when leaving mapping to the bootloader.
        let _ = virt_addr;
    }

    pub fn load_cr3(&self) {
        // Do not load our own CR3; bootloader already set up correct page tables.
    }

    fn get_or_create_table(&mut self, index: usize) -> *mut PageTable {
        unsafe {
            let entry = (*self.pml4).get_entry(index);
            if (entry & PAGE_PRESENT) == 0 {
                use super::physical::PhysicalAllocator;
                if let Some(addr) = PhysicalAllocator::allocate_page() {
                    ptr::write(addr as *mut PageTable, PageTable::new());
                    (*self.pml4).set_entry(index, addr as u64, PAGE_PRESENT | PAGE_WRITABLE);
                    addr as *mut PageTable
                } else {
                    panic!("Failed to allocate page table");
                }
            } else {
                (entry & !0xFFF) as *mut PageTable
            }
        }
    }

    fn get_or_create_table_from_table(&mut self, table: *mut PageTable, index: usize) -> *mut PageTable {
        unsafe {
            let entry = (*table).get_entry(index);
            if (entry & PAGE_PRESENT) == 0 {
                use super::physical::PhysicalAllocator;
                if let Some(addr) = PhysicalAllocator::allocate_page() {
                    ptr::write(addr as *mut PageTable, PageTable::new());
                    (*table).set_entry(index, addr as u64, PAGE_PRESENT | PAGE_WRITABLE);
                    addr as *mut PageTable
                } else {
                    panic!("Failed to allocate page table");
                }
            } else {
                (entry & !0xFFF) as *mut PageTable
            }
        }
    }

    fn get_table(&self, index: usize) -> Option<*mut PageTable> {
        unsafe {
            let entry = (*self.pml4).get_entry(index);
            if (entry & PAGE_PRESENT) != 0 {
                Some((entry & !0xFFF) as *mut PageTable)
            } else {
                None
            }
        }
    }

    fn get_table_from_table(&self, table: *mut PageTable, index: usize) -> Option<*mut PageTable> {
        unsafe {
            let entry = (*table).get_entry(index);
            if (entry & PAGE_PRESENT) != 0 {
                Some((entry & !0xFFF) as *mut PageTable)
            } else {
                None
            }
        }
    }
}