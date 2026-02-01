//! Page table helpers for ospabOS

use x86_64::structures::paging::mapper::MapToError;
use x86_64::structures::paging::{Mapper, Size4KiB, Page, PhysFrame, FrameAllocator, PageTableFlags};
use x86_64::{VirtAddr, PhysAddr};

/// Map `virt` -> `phys` without panicking if the page is already mapped.
/// Special-case: if `virt == 0x0` and already mapped by the bootloader, don't panic; skip mapping.
/// If mapped to the same frame, return Ok(()).
/// If mapped to a different frame, attempt to update by unmapping and remapping.
pub fn map_page_nonpanic<M, F>(
    mapper: &mut M,
    frame_allocator: &mut F,
    virt: VirtAddr,
    phys: PhysAddr,
) -> Result<(), MapToError<Size4KiB>>
where
    M: Mapper<Size4KiB>,
    F: FrameAllocator<Size4KiB>,
{
    let page: Page<Size4KiB> = Page::containing_address(virt);
    let frame: PhysFrame<Size4KiB> = PhysFrame::containing_address(phys);

    if let Some(existing) = mapper.translate_page(page) {
        // If already mapped to the same frame, nothing to do.
        if existing == frame {
            return Ok(());
        }

        // If this is the null page (0x0) and the bootloader mapped it, skip remapping to avoid conflicts.
        if virt.as_u64() == 0 {
            return Ok(());
        }

        // Try to unmap and remap to desired frame. If unmap fails, return the error
        // as MapToError by mapping a dummy page and returning FrameAlreadyMapped.
        unsafe {
            // Attempt to unmap; ignore the returned frame value but check for errors.
            if let Err(_) = mapper.unmap(page) {
                return Err(MapToError::FrameAlreadyMapped);
            }
            let flags = PageTableFlags::PRESENT | PageTableFlags::WRITABLE;
            mapper.map_to(page, frame, flags, frame_allocator)?.flush();
        }

        return Ok(());
    }

    // Not mapped yet: map normally.
    let flags = PageTableFlags::PRESENT | PageTableFlags::WRITABLE;
    unsafe {
        mapper.map_to(page, frame, flags, frame_allocator)?.flush();
    }
    Ok(())
}