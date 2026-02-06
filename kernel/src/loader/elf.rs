//! Minimal ELF64 loader for user-space executables.

use crate::mem::vmm::VMM;
use x86_64::structures::paging::PageTableFlags;
use x86_64::VirtAddr;

const ELF_MAGIC: [u8; 4] = [0x7F, b'E', b'L', b'F'];
const ELF_CLASS_64: u8 = 2;
const ELF_DATA_LITTLE: u8 = 1;
const ELF_MACHINE_X86_64: u16 = 0x3E;
const PT_LOAD: u32 = 1;

const USER_STACK_SIZE: usize = 4096 * 4;
const USER_STACK_TOP: u64 = 0x0000_7FFF_FFFF_F000;

#[repr(C)]
#[derive(Clone, Copy)]
struct Elf64Header {
    e_ident: [u8; 16],
    e_type: u16,
    e_machine: u16,
    e_version: u32,
    e_entry: u64,
    e_phoff: u64,
    e_shoff: u64,
    e_flags: u32,
    e_ehsize: u16,
    e_phentsize: u16,
    e_phnum: u16,
    e_shentsize: u16,
    e_shnum: u16,
    e_shstrndx: u16,
}

#[repr(C)]
#[derive(Clone, Copy)]
struct Elf64ProgramHeader {
    p_type: u32,
    p_flags: u32,
    p_offset: u64,
    p_vaddr: u64,
    p_paddr: u64,
    p_filesz: u64,
    p_memsz: u64,
    p_align: u64,
}

pub struct ElfLoadResult {
    pub entry: u64,
    pub user_stack: u64,
    pub address_space: crate::mem::vmm::AddressSpace,
}

pub fn load_user_elf(data: &[u8]) -> Result<ElfLoadResult, &'static str> {
    if data.len() < core::mem::size_of::<Elf64Header>() {
        return Err("ELF header too small");
    }

    let header = unsafe { (data.as_ptr() as *const Elf64Header).read_unaligned() };

    if header.e_ident[0..4] != ELF_MAGIC {
        return Err("Invalid ELF magic");
    }
    if header.e_ident[4] != ELF_CLASS_64 {
        return Err("Not ELF64");
    }
    if header.e_ident[5] != ELF_DATA_LITTLE {
        return Err("Not little endian ELF");
    }
    if header.e_machine != ELF_MACHINE_X86_64 {
        return Err("Unsupported ELF machine");
    }

    let phoff = header.e_phoff as usize;
    let phentsize = header.e_phentsize as usize;
    let phnum = header.e_phnum as usize;

    if phoff + phentsize * phnum > data.len() {
        return Err("ELF program headers out of range");
    }

    let mut vmm = VMM.lock();
    let vmm = vmm.as_mut().ok_or("VMM not initialized")?;
    let mut addr_space = vmm.create_user_address_space()?;

    for idx in 0..phnum {
        let off = phoff + idx * phentsize;
        if off + core::mem::size_of::<Elf64ProgramHeader>() > data.len() {
            return Err("ELF program header truncated");
        }

        let ph = unsafe { (data.as_ptr().add(off) as *const Elf64ProgramHeader).read_unaligned() };
        if ph.p_type != PT_LOAD {
            continue;
        }

        if (ph.p_offset + ph.p_filesz) as usize > data.len() {
            return Err("ELF segment out of range");
        }

        let seg_start = ph.p_vaddr & !0xFFF;
        let seg_end = (ph.p_vaddr + ph.p_memsz + 0xFFF) & !0xFFF;
        let pages = ((seg_end - seg_start) / 4096) as usize;

        // Map writable during load to allow segment initialization in kernel.
        let flags = PageTableFlags::PRESENT | PageTableFlags::USER_ACCESSIBLE | PageTableFlags::WRITABLE;

        addr_space.allocate_pages(VirtAddr::new(seg_start), pages, flags)?;

        let (old_cr3, old_flags) = x86_64::registers::control::Cr3::read();
        unsafe { addr_space.switch_to(); }

        unsafe {
            let dst = core::slice::from_raw_parts_mut(ph.p_vaddr as *mut u8, ph.p_memsz as usize);
            for b in dst.iter_mut() {
                *b = 0;
            }
            let src = &data[ph.p_offset as usize..(ph.p_offset + ph.p_filesz) as usize];
            dst[..src.len()].copy_from_slice(src);
        }

        unsafe { x86_64::registers::control::Cr3::write(old_cr3, old_flags); }
    }

    let stack_start = USER_STACK_TOP - USER_STACK_SIZE as u64;
    let stack_pages = USER_STACK_SIZE / 4096;
    addr_space.allocate_pages(VirtAddr::new(stack_start), stack_pages, PageTableFlags::PRESENT | PageTableFlags::USER_ACCESSIBLE | PageTableFlags::WRITABLE)?;

    let (old_cr3, old_flags) = x86_64::registers::control::Cr3::read();
    unsafe { addr_space.switch_to(); }
    unsafe {
        let dst = core::slice::from_raw_parts_mut(stack_start as *mut u8, USER_STACK_SIZE);
        for b in dst.iter_mut() {
            *b = 0;
        }
    }
    unsafe { x86_64::registers::control::Cr3::write(old_cr3, old_flags); }

    Ok(ElfLoadResult {
        entry: header.e_entry,
        user_stack: USER_STACK_TOP - 16,
        address_space: addr_space,
    })
}
