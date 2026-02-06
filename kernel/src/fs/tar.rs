//! Minimal ustar tar parser for initrd loading.
//!
//! This parser is intentionally small and read-only.

use alloc::format;
use alloc::string::{String, ToString};
use alloc::vec::Vec;

const TAR_BLOCK_SIZE: usize = 512;

pub struct TarEntry {
    pub path: String,
    pub data: Vec<u8>,
    pub is_dir: bool,
}

pub fn parse_tar(buf: &[u8]) -> Vec<TarEntry> {
    let mut entries = Vec::new();
    let mut offset = 0usize;

    while offset + TAR_BLOCK_SIZE <= buf.len() {
        let header = &buf[offset..offset + TAR_BLOCK_SIZE];
        if is_zero_block(header) {
            break;
        }

        let name = read_string(&header[0..100]);
        let prefix = read_string(&header[345..500]);
        let path = if !prefix.is_empty() {
            format!("{}/{}", prefix, name)
        } else {
            name
        };

        let size = read_octal(&header[124..136]);
        let typeflag = header[156];
        let is_dir = typeflag == b'5' || path.ends_with('/');

        let data_start = offset + TAR_BLOCK_SIZE;
        let data_end = data_start.saturating_add(size);

        let data = if !is_dir && data_end <= buf.len() {
            buf[data_start..data_end].to_vec()
        } else {
            Vec::new()
        };

        if !path.is_empty() {
            entries.push(TarEntry { path, data, is_dir });
        }

        let data_blocks = (size + TAR_BLOCK_SIZE - 1) / TAR_BLOCK_SIZE;
        offset = data_start + data_blocks * TAR_BLOCK_SIZE;
    }

    entries
}

fn is_zero_block(block: &[u8]) -> bool {
    block.iter().all(|b| *b == 0)
}

fn read_string(field: &[u8]) -> String {
    let end = field.iter().position(|b| *b == 0).unwrap_or(field.len());
    let bytes = &field[..end];
    String::from_utf8_lossy(bytes).trim().to_string()
}

fn read_octal(field: &[u8]) -> usize {
    let mut value = 0usize;
    for &b in field {
        if b < b'0' || b > b'7' {
            continue;
        }
        value = (value << 3) + (b - b'0') as usize;
    }
    value
}
