//! Global Descriptor Table (GDT) implementation for ospabOS
//! Production-ready implementation using spin::Lazy (no static mut)

use spin::Lazy;
use x86_64::structures::gdt::{Descriptor, GlobalDescriptorTable, SegmentSelector};
use x86_64::structures::tss::TaskStateSegment;
use x86_64::VirtAddr;

/// IST index for double fault handler - uses separate stack
pub const DOUBLE_FAULT_IST_INDEX: u16 = 0;

/// Stack size for IST stacks (20KB each)
const STACK_SIZE: usize = 4096 * 5;

/// Aligned stack structure
#[repr(C, align(4096))]
struct Stack {
    data: [u8; STACK_SIZE],
}

/// Dedicated stack for double fault handler
/// This ensures we can handle stack overflow and get proper error reports
static DOUBLE_FAULT_STACK: Stack = Stack { data: [0; STACK_SIZE] };

/// Lazy-initialized TSS with IST configured
static TSS: Lazy<TaskStateSegment> = Lazy::new(|| {
    let mut tss = TaskStateSegment::new();
    
    // Set up IST[0] for double fault - points to end of stack (grows down)
    let stack_start = VirtAddr::from_ptr(&DOUBLE_FAULT_STACK);
    let stack_end = stack_start + STACK_SIZE as u64;
    tss.interrupt_stack_table[DOUBLE_FAULT_IST_INDEX as usize] = stack_end;
    
    tss
});

/// GDT with selectors - lazy initialized
static GDT: Lazy<(GlobalDescriptorTable, Selectors)> = Lazy::new(|| {
    let mut gdt = GlobalDescriptorTable::new();
    
    // Add kernel code segment
    let code_selector = gdt.add_entry(Descriptor::kernel_code_segment());
    
    // Add kernel data segment (required for SS in x86_64)
    let data_selector = gdt.add_entry(Descriptor::kernel_data_segment());
    
    // Add TSS segment (requires reference to TSS)
    let tss_selector = gdt.add_entry(Descriptor::tss_segment(&TSS));
    
    (gdt, Selectors { code_selector, data_selector, tss_selector })
});

/// Segment selectors for kernel code, data and TSS
struct Selectors {
    code_selector: SegmentSelector,
    data_selector: SegmentSelector,
    tss_selector: SegmentSelector,
}

/// Initialize GDT and TSS
/// 
/// This function is safe to call multiple times - it will only
/// actually initialize once due to Lazy.
pub fn init() {
    use x86_64::instructions::segmentation::{CS, DS, ES, SS, Segment};
    use x86_64::instructions::tables::load_tss;

    // Force lazy initialization and load GDT
    GDT.0.load();
    
    unsafe {
        // Set code segment register
        CS::set_reg(GDT.1.code_selector);
        
        // Set data segment registers (all point to same data segment)
        SS::set_reg(GDT.1.data_selector);
        DS::set_reg(GDT.1.data_selector);
        ES::set_reg(GDT.1.data_selector);
        
        // Load TSS
        load_tss(GDT.1.tss_selector);
    }
}