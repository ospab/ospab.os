pub mod idt;
pub mod exception;

pub fn init() {
    idt::init_idt();
    exception::init_exceptions();
}