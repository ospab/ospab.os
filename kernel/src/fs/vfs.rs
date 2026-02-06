//! VFS traits and common file handle helpers.

use alloc::boxed::Box;
use alloc::vec::Vec;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FsError {
    NotFound,
    NotFile,
    NotDir,
    Permission,
    Invalid,
    Io,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OpenFlags {
    ReadOnly,
    WriteOnly,
    ReadWrite,
}

impl OpenFlags {
    pub fn from_bits(flags: u64) -> Self {
        match flags & 0b11 {
            0b01 => OpenFlags::WriteOnly,
            0b10 => OpenFlags::ReadWrite,
            _ => OpenFlags::ReadOnly,
        }
    }
}

pub trait FileHandle: Send {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize, FsError>;
    fn write(&mut self, buf: &[u8]) -> Result<usize, FsError>;
}

pub trait FileSystem: Send + Sync {
    fn open(&self, path: &str, flags: OpenFlags) -> Result<Box<dyn FileHandle>, FsError>;
}

pub struct MemFileHandle {
    data: Vec<u8>,
    offset: usize,
}

impl MemFileHandle {
    pub fn new(data: Vec<u8>) -> Self {
        Self { data, offset: 0 }
    }
}

impl FileHandle for MemFileHandle {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize, FsError> {
        if self.offset >= self.data.len() {
            return Ok(0);
        }
        let remaining = self.data.len() - self.offset;
        let to_copy = core::cmp::min(remaining, buf.len());
        buf[..to_copy].copy_from_slice(&self.data[self.offset..self.offset + to_copy]);
        self.offset += to_copy;
        Ok(to_copy)
    }

    fn write(&mut self, _buf: &[u8]) -> Result<usize, FsError> {
        Err(FsError::Permission)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeviceKind {
    Null,
    Zero,
    Keyboard,
    Framebuffer,
    Serial,
}

pub struct DeviceFileHandle {
    kind: DeviceKind,
}

impl DeviceFileHandle {
    pub fn new(kind: DeviceKind) -> Self {
        Self { kind }
    }
}

impl FileHandle for DeviceFileHandle {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize, FsError> {
        match self.kind {
            DeviceKind::Null => Ok(0),
            DeviceKind::Zero => {
                for b in buf.iter_mut() {
                    *b = 0;
                }
                Ok(buf.len())
            }
            DeviceKind::Keyboard => {
                if buf.is_empty() {
                    return Ok(0);
                }
                if let Some(ch) = crate::drivers::keyboard::try_read_key() {
                    buf[0] = ch as u8;
                    Ok(1)
                } else {
                    Ok(0)
                }
            }
            DeviceKind::Framebuffer | DeviceKind::Serial => Ok(0),
        }
    }

    fn write(&mut self, buf: &[u8]) -> Result<usize, FsError> {
        match self.kind {
            DeviceKind::Null | DeviceKind::Zero | DeviceKind::Keyboard => Ok(buf.len()),
            DeviceKind::Framebuffer => {
                for &b in buf {
                    let ch = if b < 0x80 { b as char } else { '?' };
                    crate::drivers::framebuffer::print_char(ch);
                }
                Ok(buf.len())
            }
            DeviceKind::Serial => {
                if let Ok(s) = core::str::from_utf8(buf) {
                    crate::drivers::serial::write(s);
                } else {
                    for &b in buf {
                        let ch = if b < 0x80 { b as char } else { '?' };
                        let s = [ch as u8];
                        crate::drivers::serial::write(core::str::from_utf8(&s).unwrap_or("?"));
                    }
                }
                Ok(buf.len())
            }
        }
    }
}
