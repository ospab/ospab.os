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

/// Kernel privilege stack (RSP0) for Ring 3 -> Ring 0 transitions
static KERNEL_PRIV_STACK: Stack = Stack { data: [0; STACK_SIZE] };

/// Lazy-initialized TSS with IST configured
static TSS: Lazy<TaskStateSegment> = Lazy::new(|| {
    let mut tss = TaskStateSegment::new();
    
    // Set up IST[0] for double fault - points to end of stack (grows down)
    let stack_start = VirtAddr::from_ptr(&DOUBLE_FAULT_STACK);
    let stack_end = stack_start + STACK_SIZE as u64;
    tss.interrupt_stack_table[DOUBLE_FAULT_IST_INDEX as usize] = stack_end;

    // Set up privilege stack 0 for user->kernel transitions
    let priv_stack_start = VirtAddr::from_ptr(&KERNEL_PRIV_STACK);
    let priv_stack_end = priv_stack_start + STACK_SIZE as u64;
    tss.privilege_stack_table[0] = priv_stack_end;
    
    tss
});

/// GDT with selectors - lazy initialized
static GDT: Lazy<(GlobalDescriptorTable, Selectors)> = Lazy::new(|| {
    let mut gdt = GlobalDescriptorTable::new();
    
    // Add kernel code/data segments
    let kernel_code = gdt.add_entry(Descriptor::kernel_code_segment());
    let kernel_data = gdt.add_entry(Descriptor::kernel_data_segment());

    // Add user code/data segments (Ring 3)
    let user_data = gdt.add_entry(Descriptor::user_data_segment());
    let user_code = gdt.add_entry(Descriptor::user_code_segment());
    
    // Add TSS segment (requires reference to TSS)
    let tss_selector = gdt.add_entry(Descriptor::tss_segment(&TSS));

    (
        gdt,
        Selectors {
            kernel_code,
            kernel_data,
            user_code,
            user_data,
            tss_selector,
        },
    )
});

/// Segment selectors for kernel code, data and TSS
#[derive(Clone, Copy)]
pub struct Selectors {
    pub kernel_code: SegmentSelector,
    pub kernel_data: SegmentSelector,
    pub user_code: SegmentSelector,
    pub user_data: SegmentSelector,
    pub tss_selector: SegmentSelector,
}

pub fn selectors() -> Selectors {
    GDT.1
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
        CS::set_reg(GDT.1.kernel_code);

        // Set data segment registers
        SS::set_reg(GDT.1.kernel_data);
        DS::set_reg(GDT.1.kernel_data);
        ES::set_reg(GDT.1.kernel_data);

        // Load TSS
        load_tss(GDT.1.tss_selector);
    }
}