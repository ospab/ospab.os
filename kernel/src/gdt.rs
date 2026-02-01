//! Global Descriptor Table (GDT) implementation for ospabOS
//! Uses a simple approach to avoid runtime initialization issues

#![allow(static_mut_refs)]

use x86_64::structures::gdt::{Descriptor, GlobalDescriptorTable, SegmentSelector};
use x86_64::structures::tss::TaskStateSegment;
use x86_64::VirtAddr;

pub const DOUBLE_FAULT_IST_INDEX: u16 = 0;

// Static stack for double fault handler - must be properly aligned
const STACK_SIZE: usize = 4096 * 5;

#[repr(C, align(4096))]
struct Stack {
    data: [u8; STACK_SIZE],
}

static DOUBLE_FAULT_STACK: Stack = Stack { data: [0; STACK_SIZE] };

// Use a simple static approach - initialize once and keep
static mut TSS: TaskStateSegment = TaskStateSegment::new();
static mut GDT: GlobalDescriptorTable = GlobalDescriptorTable::new();
static mut CODE_SELECTOR: SegmentSelector = SegmentSelector(0);
static mut TSS_SELECTOR: SegmentSelector = SegmentSelector(0);
static mut INITIALIZED: bool = false;

pub fn init() {
    use x86_64::instructions::segmentation::{CS, Segment};
    use x86_64::instructions::tables::load_tss;

    unsafe {
        if INITIALIZED {
            return;
        }
        
        // Setup TSS with double fault stack
        let stack_end = VirtAddr::new(
            (&DOUBLE_FAULT_STACK.data as *const _ as u64) + STACK_SIZE as u64
        );
        TSS.interrupt_stack_table[DOUBLE_FAULT_IST_INDEX as usize] = stack_end;
        
        // Setup GDT
        CODE_SELECTOR = GDT.add_entry(Descriptor::kernel_code_segment());
        TSS_SELECTOR = GDT.add_entry(Descriptor::tss_segment(&TSS));
        
        // Load GDT
        GDT.load();
        
        // Set segment registers
        CS::set_reg(CODE_SELECTOR);
        load_tss(TSS_SELECTOR);
        
        INITIALIZED = true;
    }
}