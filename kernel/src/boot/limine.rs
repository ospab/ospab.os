//! Limine boot protocol definitions and requests
//!
//! This module provides the structures and requests needed for the Limine boot protocol.
//! The bootloader scans the kernel binary for these request structures and responds
//! by filling in the response pointers.

use core::ffi::c_char;
use core::ptr;

/// Magic numbers used by Limine to identify requests
const LIMINE_COMMON_MAGIC: [u64; 2] = [0xc7b1dd30df4c8b88, 0x0a82e883a194f07b];

/// Request markers - bootloader scans between these
#[used]
#[link_section = ".limine_requests_start"]
static LIMINE_REQUESTS_START: [u64; 4] = [
    0xf6b8f4b39de7d1ae,
    0xfab91a6940fcb9cf,
    0x785c6ed015d3e316,
    0x181e920a7852b9d9,
];

#[used]
#[link_section = ".limine_requests_end"]
static LIMINE_REQUESTS_END: [u64; 2] = [0xadc0e0531bb10d03, 0x9572709f31764c62];

/// Base revision tag - bootloader sets third value to 0 if supported
/// Format: [magic1, magic2, revision]
#[used]
#[link_section = ".limine_requests"]
static mut BASE_REVISION: [u64; 3] = [
    0xf9562b2d5c95a6c8,
    0x6a7b384944536bdc,
    2,  // Request revision 2 - will be set to 0 by bootloader if supported
];

/// Check if the base revision was accepted by the bootloader
pub fn base_revision_supported() -> bool {
    unsafe { BASE_REVISION[2] == 0 }
}

/// Get the raw base revision value for debugging
pub fn get_base_revision_raw() -> u64 {
    unsafe { BASE_REVISION[2] }
}

// ============================================================================
// Bootloader Info Request
// ============================================================================

#[repr(C)]
pub struct BootloaderInfoResponse {
    pub revision: u64,
    pub name: *const c_char,
    pub version: *const c_char,
}

#[repr(C)]
pub struct BootloaderInfoRequest {
    pub id: [u64; 4],
    pub revision: u64,
    pub response: *mut BootloaderInfoResponse,
}

unsafe impl Sync for BootloaderInfoRequest {}

#[used]
#[link_section = ".limine_requests"]
static mut BOOTLOADER_INFO_REQUEST: BootloaderInfoRequest = BootloaderInfoRequest {
    id: [
        LIMINE_COMMON_MAGIC[0],
        LIMINE_COMMON_MAGIC[1],
        0xf55038d8e2a1202f,
        0x279426fcf5f59740,
    ],
    revision: 0,
    response: ptr::null_mut(),
};

pub fn bootloader_info() -> Option<&'static BootloaderInfoResponse> {
    unsafe {
        if BOOTLOADER_INFO_REQUEST.response.is_null() {
            None
        } else {
            Some(&*BOOTLOADER_INFO_REQUEST.response)
        }
    }
}

// ============================================================================
// HHDM (Higher Half Direct Map) Request
// ============================================================================

#[repr(C)]
pub struct HhdmResponse {
    pub revision: u64,
    pub offset: u64,
}

#[repr(C)]
pub struct HhdmRequest {
    pub id: [u64; 4],
    pub revision: u64,
    pub response: *mut HhdmResponse,
}

unsafe impl Sync for HhdmRequest {}

#[used]
#[link_section = ".limine_requests"]
static mut HHDM_REQUEST: HhdmRequest = HhdmRequest {
    id: [
        LIMINE_COMMON_MAGIC[0],
        LIMINE_COMMON_MAGIC[1],
        0x48dcf1cb8ad2b852,
        0x63984e959a98244b,
    ],
    revision: 0,
    response: ptr::null_mut(),
};

/// Get the HHDM offset - the base of the higher half direct map
pub fn hhdm_offset() -> Option<u64> {
    unsafe {
        if HHDM_REQUEST.response.is_null() {
            None
        } else {
            Some((*HHDM_REQUEST.response).offset)
        }
    }
}

// ============================================================================
// Framebuffer Request
// ============================================================================

#[repr(C)]
pub struct Framebuffer {
    pub address: *mut u8,
    pub width: u64,
    pub height: u64,
    pub pitch: u64,
    pub bpp: u16,
    pub memory_model: u8,
    pub red_mask_size: u8,
    pub red_mask_shift: u8,
    pub green_mask_size: u8,
    pub green_mask_shift: u8,
    pub blue_mask_size: u8,
    pub blue_mask_shift: u8,
    pub unused: [u8; 7],
    pub edid_size: u64,
    pub edid: *mut u8,
    pub mode_count: u64,
    pub modes: *mut *mut VideoMode,
}

#[repr(C)]
pub struct VideoMode {
    pub pitch: u64,
    pub width: u64,
    pub height: u64,
    pub bpp: u16,
    pub memory_model: u8,
    pub red_mask_size: u8,
    pub red_mask_shift: u8,
    pub green_mask_size: u8,
    pub green_mask_shift: u8,
    pub blue_mask_size: u8,
    pub blue_mask_shift: u8,
}

#[repr(C)]
pub struct FramebufferResponse {
    pub revision: u64,
    pub framebuffer_count: u64,
    pub framebuffers: *mut *mut Framebuffer,
}

#[repr(C)]
pub struct FramebufferRequest {
    pub id: [u64; 4],
    pub revision: u64,
    pub response: *mut FramebufferResponse,
}

unsafe impl Sync for FramebufferRequest {}

#[used]
#[link_section = ".limine_requests"]
static mut FRAMEBUFFER_REQUEST: FramebufferRequest = FramebufferRequest {
    id: [
        LIMINE_COMMON_MAGIC[0],
        LIMINE_COMMON_MAGIC[1],
        0x9d5827dcd881dd75,
        0xa3148604f6fab11b,
    ],
    revision: 0,
    response: ptr::null_mut(),
};

pub fn framebuffer() -> Option<&'static Framebuffer> {
    unsafe {
        if FRAMEBUFFER_REQUEST.response.is_null() {
            return None;
        }
        let resp = &*FRAMEBUFFER_REQUEST.response;
        if resp.framebuffer_count == 0 || resp.framebuffers.is_null() {
            return None;
        }
        let fb_ptr = *resp.framebuffers;
        if fb_ptr.is_null() {
            None
        } else {
            Some(&*fb_ptr)
        }
    }
}

// ============================================================================
// Memory Map Request
// ============================================================================

pub const MEMMAP_USABLE: u64 = 0;
pub const MEMMAP_RESERVED: u64 = 1;
pub const MEMMAP_ACPI_RECLAIMABLE: u64 = 2;
pub const MEMMAP_ACPI_NVS: u64 = 3;
pub const MEMMAP_BAD_MEMORY: u64 = 4;
pub const MEMMAP_BOOTLOADER_RECLAIMABLE: u64 = 5;
pub const MEMMAP_KERNEL_AND_MODULES: u64 = 6;
pub const MEMMAP_FRAMEBUFFER: u64 = 7;

#[repr(C)]
pub struct MemmapEntry {
    pub base: u64,
    pub length: u64,
    pub typ: u64,
}

#[repr(C)]
pub struct MemmapResponse {
    pub revision: u64,
    pub entry_count: u64,
    pub entries: *mut *mut MemmapEntry,
}

#[repr(C)]
pub struct MemmapRequest {
    pub id: [u64; 4],
    pub revision: u64,
    pub response: *mut MemmapResponse,
}

unsafe impl Sync for MemmapRequest {}

#[used]
#[link_section = ".limine_requests"]
static mut MEMMAP_REQUEST: MemmapRequest = MemmapRequest {
    id: [
        LIMINE_COMMON_MAGIC[0],
        LIMINE_COMMON_MAGIC[1],
        0x67cf3d9d378a806f,
        0xe304acdfc50c3c62,
    ],
    revision: 0,
    response: ptr::null_mut(),
};

pub struct MemmapIterator {
    entries: *mut *mut MemmapEntry,
    count: usize,
    index: usize,
}

impl Iterator for MemmapIterator {
    type Item = &'static MemmapEntry;
    
    fn next(&mut self) -> Option<Self::Item> {
        if self.index >= self.count {
            return None;
        }
        unsafe {
            let entry_ptr = *self.entries.add(self.index);
            self.index += 1;
            if entry_ptr.is_null() {
                None
            } else {
                Some(&*entry_ptr)
            }
        }
    }
}

pub fn memory_map() -> Option<MemmapIterator> {
    unsafe {
        if MEMMAP_REQUEST.response.is_null() {
            return None;
        }
        let resp = &*MEMMAP_REQUEST.response;
        if resp.entry_count == 0 || resp.entries.is_null() {
            return None;
        }
        Some(MemmapIterator {
            entries: resp.entries,
            count: resp.entry_count as usize,
            index: 0,
        })
    }
}

// ============================================================================
// Entry Point Request (optional - lets kernel specify custom entry)
// ============================================================================

#[repr(C)]
pub struct EntryPointResponse {
    pub revision: u64,
}

#[repr(C)]
pub struct EntryPointRequest {
    pub id: [u64; 4],
    pub revision: u64,
    pub response: *mut EntryPointResponse,
    pub entry: extern "C" fn() -> !,
}

unsafe impl Sync for EntryPointRequest {}

// Note: We use _start as our entry point, which is the standard way.
// The entry point request is optional and mainly used when you want
// a different entry than what's specified in the ELF header.
