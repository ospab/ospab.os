//! Minimal coreutils implemented against the VFS service.

use alloc::string::String;
use alloc::vec::Vec;
use crate::ipc::message::{FSRequest, FSResponse};
use crate::services::vfs;

pub fn ls(path: &str) -> Result<Vec<String>, String> {
    let response = vfs::process_request(FSRequest::ListDir { path: path.to_string() });
    match response {
        FSResponse::DirListing(entries) => Ok(entries),
        FSResponse::Error(msg) => Err(msg),
        _ => Err("Unexpected response".to_string()),
    }
}

pub fn cat(path: &str) -> Result<Vec<u8>, String> {
    let response = vfs::process_request(FSRequest::ReadFile { path: path.to_string() });
    match response {
        FSResponse::FileData(data) => Ok(data),
        FSResponse::Error(msg) => Err(msg),
        _ => Err("Unexpected response".to_string()),
    }
}

pub fn mkdir(path: &str) -> Result<(), String> {
    let response = vfs::process_request(FSRequest::CreateDir { path: path.to_string() });
    match response {
        FSResponse::Success => Ok(()),
        FSResponse::Error(msg) => Err(msg),
        _ => Err("Unexpected response".to_string()),
    }
}

pub fn cp(src: &str, dst: &str) -> Result<(), String> {
    let data = cat(src)?;
    let response = vfs::process_request(FSRequest::WriteFile {
        path: dst.to_string(),
        data,
    });
    match response {
        FSResponse::Success => Ok(()),
        FSResponse::Error(msg) => Err(msg),
        _ => Err("Unexpected response".to_string()),
    }
}

pub fn mv(src: &str, dst: &str) -> Result<(), String> {
    cp(src, dst)?;
    let response = vfs::process_request(FSRequest::Delete { path: src.to_string() });
    match response {
        FSResponse::Success => Ok(()),
        FSResponse::Error(msg) => Err(msg),
        _ => Err("Unexpected response".to_string()),
    }
}
