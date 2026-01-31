//! Global Descriptor Table (GDT) implementation for ospabOS

use x86_64::structures::gdt::{Descriptor, GlobalDescriptorTable, SegmentSelector};
use x86_64::VirtAddr;
use spin::Once;

pub const DOUBLE_FAULT_IST_INDEX: u16 = 0;

static GDT_ONCE: Once<(GlobalDescriptorTable, Selectors)> = Once::new();

fn get_gdt() -> &'static (GlobalDescriptorTable, Selectors) {
    GDT_ONCE.call_once(|| {
        let mut gdt = GlobalDescriptorTable::new();
        let code_selector = gdt.add_entry(Descriptor::kernel_code_segment());
        let tss_selector = gdt.add_entry(Descriptor::tss_segment(&get_tss()));
        (gdt, Selectors { code_selector, tss_selector })
    })
}

struct Selectors {
    code_selector: SegmentSelector,
    tss_selector: SegmentSelector,
}

pub fn init() {
    use x86_64::instructions::segmentation::{CS, Segment};
    use x86_64::instructions::tables::load_tss;

    get_gdt().0.load();
    unsafe {
        CS::set_reg(get_gdt().1.code_selector);
        load_tss(get_gdt().1.tss_selector);
    }
}

use x86_64::structures::tss::TaskStateSegment;

static TSS_ONCE: Once<TaskStateSegment> = Once::new();

fn init_tss() -> TaskStateSegment {
    let mut tss = TaskStateSegment::new();
    tss.interrupt_stack_table[DOUBLE_FAULT_IST_INDEX as usize] = {
        const STACK_SIZE: usize = 4096 * 5;
        static mut STACK: [u8; STACK_SIZE] = [0; STACK_SIZE];
        let stack_start = VirtAddr::from_ptr(core::ptr::addr_of!(STACK) as *const u8);
        let stack_end = stack_start + STACK_SIZE;
        stack_end
    };
    tss
}

fn get_tss() -> &'static TaskStateSegment {
    TSS_ONCE.call_once(init_tss)
}