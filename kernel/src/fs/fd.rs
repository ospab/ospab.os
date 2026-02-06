//! Per-process file descriptor table.

use alloc::boxed::Box;
use alloc::vec::Vec;

use super::vfs::{DeviceFileHandle, DeviceKind, FileHandle, FsError};

pub struct FdTable {
    entries: Vec<Option<Box<dyn FileHandle>>>,
}

impl FdTable {
    pub fn new() -> Self {
        Self { entries: Vec::new() }
    }

    pub fn with_stdio() -> Self {
        let mut table = Self::new();
        table.entries.resize_with(3, || None);
        table.entries[0] = Some(Box::new(DeviceFileHandle::new(DeviceKind::Keyboard)));
        table.entries[1] = Some(Box::new(DeviceFileHandle::new(DeviceKind::Framebuffer)));
        table.entries[2] = Some(Box::new(DeviceFileHandle::new(DeviceKind::Serial)));
        table
    }

    pub fn insert(&mut self, handle: Box<dyn FileHandle>) -> u32 {
        for (idx, entry) in self.entries.iter_mut().enumerate() {
            if entry.is_none() {
                *entry = Some(handle);
                return idx as u32;
            }
        }
        let fd = self.entries.len() as u32;
        self.entries.push(Some(handle));
        fd
    }

    pub fn get_mut(&mut self, fd: u32) -> Result<&mut Box<dyn FileHandle>, FsError> {
        let idx = fd as usize;
        if idx >= self.entries.len() {
            return Err(FsError::Invalid);
        }
        self.entries[idx].as_mut().ok_or(FsError::Invalid)
    }

    pub fn close(&mut self, fd: u32) -> Result<(), FsError> {
        let idx = fd as usize;
        if idx >= self.entries.len() {
            return Err(FsError::Invalid);
        }
        self.entries[idx] = None;
        Ok(())
    }
}
