//! Page table implementation for ospabOS

use x86_64::structures::paging::{PageTable, PhysFrame, Mapper, Size4KiB, FrameAllocator};
use x86_64::{VirtAddr, PhysAddr};
use x86_64::structures::paging::mapper::MapToError;

pub struct PageTableManager {
    mapper: Mapper<Size4KiB>,
    frame_allocator: FrameAllocator,
}

impl PageTableManager {
    pub fn new(mapper: Mapper<Size4KiB>, frame_allocator: FrameAllocator) -> Self {
        PageTableManager { mapper, frame_allocator }
    }

    pub fn map_page(&mut self, virt: VirtAddr, phys: PhysAddr) -> Result<(), MapToError<Size4KiB>> {
        let frame = PhysFrame::containing_address(phys);
        let page = Page::containing_address(virt);

        if let Some(mapped_frame) = self.mapper.translate_page(page) {
            if mapped_frame == frame {
                return Ok(()); // Already mapped to the same frame
            } else {
                return Err(MapToError::FrameAlreadyMapped);
            }
        }

        self.mapper.map_to(page, frame, PageTableFlags::PRESENT | PageTableFlags::WRITABLE, &mut self.frame_allocator)?.flush();
        Ok(())
    }
}