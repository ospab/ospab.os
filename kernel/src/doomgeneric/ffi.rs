use core::ffi::CStr;
use alloc::vec::Vec;
use alloc::string::ToString;
use crate::drivers::{framebuffer, keyboard};
use crate::ipc::message::{FSRequest, FSResponse};
use crate::services::vfs;

static mut WAD_DATA: Option<Vec<u8>> = None;
static mut WAD_POS: usize = 0;

#[no_mangle]
pub extern "C" fn DG_Sys_Framebuffer(fb: *const u32, w: i32, h: i32) {
    if fb.is_null() || w <= 0 || h <= 0 { return; }
    let width = w as usize;
    let height = h as usize;
    unsafe {
        for y in 0..height {
            for x in 0..width {
                let idx = y * width + x;
                let color = core::ptr::read(fb.add(idx));
                framebuffer::set_pixel(x, y, color);
            }
        }
    }
}

#[no_mangle]
pub extern "C" fn DG_Sys_ReadKey() -> i32 {
    if let Some(ch) = keyboard::try_read_key() {
        return ch as i32;
    }
    0
}

#[no_mangle]
pub extern "C" fn DG_Sys_OpenWAD(path: *const i8) -> i32 {
    if path.is_null() { return -1; }
    let cstr = unsafe { CStr::from_ptr(path) };
    let path_str = match cstr.to_str() {
        Ok(s) => s,
        Err(_) => return -1,
    };

    match vfs::process_request(FSRequest::ReadFile { path: path_str.to_string() }) {
        FSResponse::FileData(data) => {
            unsafe {
                WAD_DATA = Some(data);
                WAD_POS = 0;
            }
            0
        }
        _ => -1,
    }
}

#[no_mangle]
pub extern "C" fn DG_Sys_ReadWAD(_fd: i32, buf: *mut u8, len: i32) -> i32 {
    if buf.is_null() || len <= 0 { return 0; }
    unsafe {
        if let Some(ref data) = WAD_DATA {
            let remaining = data.len().saturating_sub(WAD_POS);
            if remaining == 0 { return 0; }
            let to_copy = core::cmp::min(remaining, len as usize);
            let dst = core::slice::from_raw_parts_mut(buf, to_copy);
            dst.copy_from_slice(&data[WAD_POS..WAD_POS + to_copy]);
            WAD_POS += to_copy;
            to_copy as i32
        } else {
            0
        }
    }
}
