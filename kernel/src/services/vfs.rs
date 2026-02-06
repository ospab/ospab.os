//! Virtual Filesystem Service - Unix-like hierarchy
//!
//! Implements a Unix/Linux-like filesystem structure:
//! / - root
//! /bin - system binaries (commands)
//! /etc - configuration files
//! /home - user home directories
//! /tmp - temporary files
//! /dev - device files
//! /usr - user programs
//! /var - variable data (logs, etc)

use alloc::string::{String, ToString};
use alloc::vec::Vec;
use alloc::format;
use alloc::collections::BTreeMap;
use core::ffi::CStr;
use crate::ipc::message::{FSRequest, FSResponse};
use crate::boot::limine;
use crate::fs::tar;
use crate::fs::vfs::{DeviceFileHandle, DeviceKind, FileHandle, FileSystem, FsError, MemFileHandle, OpenFlags};
use alloc::boxed::Box;

/// File type
#[derive(Clone, PartialEq)]
pub enum FileType {
    Regular,    // Regular file
    Directory,  // Directory
    Device,     // Device file
    Link,       // Symbolic link
}

/// Virtual file entry
#[derive(Clone)]
pub struct VNode {
    pub name: String,
    pub file_type: FileType,
    pub size: usize,
    pub data: Option<Vec<u8>>,  // For regular files
    pub children: Option<BTreeMap<String, VNode>>,  // For directories
    pub device_id: Option<usize>,  // For device files
}

impl VNode {
    /// Create new directory
    pub fn new_dir(name: &str) -> Self {
        Self {
            name: name.to_string(),
            file_type: FileType::Directory,
            size: 0,
            data: None,
            children: Some(BTreeMap::new()),
            device_id: None,
        }
    }
    
    /// Create new file
    pub fn new_file(name: &str, data: Vec<u8>) -> Self {
        let size = data.len();
        Self {
            name: name.to_string(),
            file_type: FileType::Regular,
            size,
            data: Some(data),
            children: None,
            device_id: None,
        }
    }
    
    /// Create new device file
    pub fn new_device(name: &str, device_id: usize) -> Self {
        Self {
            name: name.to_string(),
            file_type: FileType::Device,
            size: 0,
            data: None,
            children: None,
            device_id: Some(device_id),
        }
    }
}

/// Unix-like VFS Service
pub struct VFSService {
    root: spin::Mutex<VNode>,
    current_dir: spin::Mutex<String>,
}

impl VFSService {
    /// Create new VFS service with Unix-like structure
    pub const fn new() -> Self {
        Self {
            root: spin::Mutex::new(VNode {
                name: String::new(),
                file_type: FileType::Directory,
                size: 0,
                data: None,
                children: None,
                device_id: None,
            }),
            current_dir: spin::Mutex::new(String::new()),
        }
    }

    fn normalize_path(path: &str) -> String {
        let mut parts: Vec<&str> = Vec::new();
        for part in path.split('/') {
            if part.is_empty() || part == "." {
                continue;
            }
            if part == ".." {
                parts.pop();
            } else {
                parts.push(part);
            }
        }
        if parts.is_empty() {
            "/".to_string()
        } else {
            format!("/{}", parts.join("/"))
        }
    }

    fn insert_path(root: &mut VNode, path: &str, data: Option<Vec<u8>>, is_dir: bool) {
        let clean = path.trim_start_matches('/').trim_start_matches("./");
        if clean.is_empty() {
            return;
        }

        let components: Vec<&str> = clean.split('/').filter(|s| !s.is_empty()).collect();

        fn insert_components(node: &mut VNode, comps: &[&str], data: &Option<Vec<u8>>, is_dir: bool) {
            if comps.is_empty() {
                return;
            }

            let name = comps[0];
            let children = node.children.get_or_insert_with(BTreeMap::new);

            if comps.len() == 1 {
                if is_dir {
                    children.entry(name.to_string()).or_insert_with(|| VNode::new_dir(name));
                } else {
                    let file_data = data.clone().unwrap_or_default();
                    children.insert(name.to_string(), VNode::new_file(name, file_data));
                }
                return;
            }

            let child = children.entry(name.to_string()).or_insert_with(|| VNode::new_dir(name));
            insert_components(child, &comps[1..], data, is_dir);
        }

        insert_components(root, &components, &data, is_dir);
    }

    fn resolve_path_mut<'a>(node: &'a mut VNode, components: &[&str]) -> Option<&'a mut VNode> {
        let mut current = node;
        for comp in components {
            let children = current.children.as_mut()?;
            current = children.get_mut(*comp)?;
        }
        Some(current)
    }

    /// Initialize VFS with Unix-like directory tree
    pub fn init(&self) {
        // Create root directory structure
        let mut root = VNode::new_dir("/");
        let mut children = BTreeMap::new();
        
        // /bin - system binaries
        let mut bin = VNode::new_dir("bin");
        let mut bin_children = BTreeMap::new();
        bin_children.insert("ls".to_string(), 
            VNode::new_file("ls", b"List directory contents".to_vec()));
        bin_children.insert("cat".to_string(),
            VNode::new_file("cat", b"Concatenate files".to_vec()));
        bin_children.insert("grape".to_string(),
            VNode::new_file("grape", b"Grape text editor".to_vec()));
        bin.children = Some(bin_children);
        children.insert("bin".to_string(), bin);
        
        // /etc - configuration
        let mut etc = VNode::new_dir("etc");
        let mut etc_children = BTreeMap::new();
        etc_children.insert("hostname".to_string(),
            VNode::new_file("hostname", b"ospabOS\n".to_vec()));
        etc_children.insert("os-release".to_string(),
            VNode::new_file("os-release", 
                b"NAME=\"ospabOS\"\nVERSION=\"0.1.0\"\nID=ospab\nPRETTY_NAME=\"ospabOS 0.1.0 Foundation\"\n".to_vec()));
        etc.children = Some(etc_children);
        children.insert("etc".to_string(), etc);
        
        // /home - user directories
        let mut home = VNode::new_dir("home");
        let mut home_children = BTreeMap::new();
        let mut user = VNode::new_dir("user");
        user.children = Some(BTreeMap::new());
        home_children.insert("user".to_string(), user);
        home.children = Some(home_children);
        children.insert("home".to_string(), home);
        
        // /tmp - temporary files
        let mut tmp = VNode::new_dir("tmp");
        tmp.children = Some(BTreeMap::new());
        children.insert("tmp".to_string(), tmp);
        
        // /dev - device files
        let mut dev = VNode::new_dir("dev");
        let mut dev_children = BTreeMap::new();
        dev_children.insert("null".to_string(), VNode::new_device("null", 0));
        dev_children.insert("zero".to_string(), VNode::new_device("zero", 1));
        dev_children.insert("keyboard".to_string(), VNode::new_device("keyboard", 2));
        dev_children.insert("framebuffer".to_string(), VNode::new_device("framebuffer", 3));
        dev_children.insert("serial".to_string(), VNode::new_device("serial", 4));
        dev.children = Some(dev_children);
        children.insert("dev".to_string(), dev);
        
        // /usr - user programs
        let mut usr = VNode::new_dir("usr");
        let mut usr_children = BTreeMap::new();
        let mut usr_bin = VNode::new_dir("bin");
        usr_bin.children = Some(BTreeMap::new());
        usr_children.insert("bin".to_string(), usr_bin);
        usr.children = Some(usr_children);
        children.insert("usr".to_string(), usr);
        
        // /var - variable data
        let mut var = VNode::new_dir("var");
        let mut var_children = BTreeMap::new();
        let mut var_log = VNode::new_dir("log");
        var_log.children = Some(BTreeMap::new());
        var_children.insert("log".to_string(), var_log);
        var.children = Some(var_children);
        children.insert("var".to_string(), var);
        
        root.children = Some(children);
        
        // Load files from Limine modules into root
        if let Some(modules) = limine::modules() {
            for module in modules {
                if module.path.is_null() {
                    continue;
                }

                let path = unsafe {
                    if let Ok(cstr) = CStr::from_ptr(module.path as *const i8).to_str() {
                        cstr
                    } else {
                        continue;
                    }
                };

                let filename = if let Some(pos) = path.rfind('/') {
                    &path[pos + 1..]
                } else {
                    path
                };

                let data = unsafe {
                    core::slice::from_raw_parts(module.address as *const u8, module.size as usize)
                };

                if filename.ends_with(".tar") {
                    let entries = tar::parse_tar(data);
                    for entry in entries {
                        Self::insert_path(&mut root, &entry.path, Some(entry.data), entry.is_dir);
                    }
                    continue;
                }

                // Copy file data for plain modules
                let file_data = data.to_vec();
                Self::insert_path(&mut root, filename, Some(file_data), false);
            }
        }
        
        *self.root.lock() = root;
        *self.current_dir.lock() = "/".to_string();
    }
    
    /// Resolve path to VNode
    fn resolve_path(&self, path: &str) -> Option<VNode> {
        let root = self.root.lock();
        
        if path == "/" {
            return Some(root.clone());
        }
        
        let path = path.trim_start_matches('/');
        let components: Vec<&str> = path.split('/').filter(|s| !s.is_empty()).collect();
        
        let mut current = root.clone();
        
        for component in components {
            if let Some(ref children) = current.children {
                if let Some(child) = children.get(component) {
                    current = child.clone();
                } else {
                    return None;
                }
            } else {
                return None;
            }
        }
        
        Some(current)
    }

    pub fn open_handle(&self, path: &str, flags: OpenFlags) -> Result<Box<dyn FileHandle>, FsError> {
        let resolve_path = if path.starts_with('/') {
            path.to_string()
        } else {
            let cwd = self.current_dir.lock().clone();
            if cwd == "/" {
                format!("/{}", path)
            } else {
                format!("{}/{}", cwd, path)
            }
        };
        let resolve_path = Self::normalize_path(&resolve_path);

        let node = self.resolve_path(&resolve_path).ok_or(FsError::NotFound)?;

        match node.file_type {
            FileType::Regular => {
                if matches!(flags, OpenFlags::WriteOnly | OpenFlags::ReadWrite) {
                    return Err(FsError::Permission);
                }
                let data = node.data.unwrap_or_default();
                Ok(Box::new(MemFileHandle::new(data)))
            }
            FileType::Device => {
                let dev = match node.device_id.unwrap_or(0) {
                    0 => DeviceKind::Null,
                    1 => DeviceKind::Zero,
                    2 => DeviceKind::Keyboard,
                    3 => DeviceKind::Framebuffer,
                    4 => DeviceKind::Serial,
                    _ => return Err(FsError::Invalid),
                };
                Ok(Box::new(DeviceFileHandle::new(dev)))
            }
            FileType::Directory => Err(FsError::NotFile),
            FileType::Link => Err(FsError::Invalid),
        }
    }

    /// Process filesystem request
    pub fn process(&self, request: FSRequest) -> FSResponse {
        match request {
            FSRequest::ListDir { path } => {
                let resolve_path = if path == "." || path.is_empty() {
                    self.current_dir.lock().clone()
                } else if path.starts_with('/') {
                    path.clone()
                } else {
                    let cwd = self.current_dir.lock().clone();
                    if cwd == "/" {
                        format!("/{}", path)
                    } else {
                        format!("{}/{}", cwd, path)
                    }
                };
                let resolve_path = Self::normalize_path(&resolve_path);
                
                if let Some(node) = self.resolve_path(&resolve_path) {
                    if node.file_type == FileType::Directory {
                        if let Some(ref children) = node.children {
                            let mut names: Vec<String> = children.keys().cloned().collect();
                            names.sort();
                            FSResponse::DirListing(names)
                        } else {
                            FSResponse::DirListing(Vec::new())
                        }
                    } else {
                        FSResponse::Error("Not a directory".to_string())
                    }
                } else {
                    FSResponse::Error("Directory not found".to_string())
                }
            }
            FSRequest::ReadFile { path } => {
                let resolve_path = if path.starts_with('/') {
                    path.clone()
                } else {
                    let cwd = self.current_dir.lock().clone();
                    if cwd == "/" {
                        format!("/{}", path)
                    } else {
                        format!("{}/{}", cwd, path)
                    }
                };
                let resolve_path = Self::normalize_path(&resolve_path);
                
                if let Some(node) = self.resolve_path(&resolve_path) {
                    match node.file_type {
                        FileType::Regular => {
                            if let Some(data) = node.data {
                                FSResponse::FileData(data)
                            } else {
                                FSResponse::FileData(Vec::new())
                            }
                        }
                        FileType::Device => {
                            FSResponse::FileData(b"<device file>".to_vec())
                        }
                        _ => FSResponse::Error("Cannot read this file type".to_string())
                    }
                } else {
                    FSResponse::Error(format!("File not found: {}", path))
                }
            }
            FSRequest::WriteFile { path, data } => {
                let resolve_path = if path.starts_with('/') {
                    path.clone()
                } else {
                    let cwd = self.current_dir.lock().clone();
                    if cwd == "/" {
                        format!("/{}", path)
                    } else {
                        format!("{}/{}", cwd, path)
                    }
                };
                let resolve_path = Self::normalize_path(&resolve_path);
                let clean = resolve_path.trim_start_matches('/');
                if clean.is_empty() {
                    return FSResponse::Error("Invalid path".to_string());
                }
                let components: Vec<&str> = clean.split('/').filter(|s| !s.is_empty()).collect();
                if components.is_empty() {
                    return FSResponse::Error("Invalid path".to_string());
                }
                let (parent_parts, name) = components.split_at(components.len() - 1);
                let mut root = self.root.lock();
                let parent = if parent_parts.is_empty() {
                    &mut *root
                } else {
                    match Self::resolve_path_mut(&mut root, parent_parts) {
                        Some(node) => node,
                        None => return FSResponse::Error("Directory not found".to_string()),
                    }
                };
                if parent.file_type != FileType::Directory {
                    return FSResponse::Error("Not a directory".to_string());
                }
                let children = parent.children.get_or_insert_with(BTreeMap::new);
                children.insert(name[0].to_string(), VNode::new_file(name[0], data));
                FSResponse::Success
            }
            FSRequest::CreateDir { path } => {
                let resolve_path = if path.starts_with('/') {
                    path.clone()
                } else {
                    let cwd = self.current_dir.lock().clone();
                    if cwd == "/" {
                        format!("/{}", path)
                    } else {
                        format!("{}/{}", cwd, path)
                    }
                };
                let resolve_path = Self::normalize_path(&resolve_path);
                let clean = resolve_path.trim_start_matches('/');
                if clean.is_empty() {
                    return FSResponse::Success;
                }
                let components: Vec<&str> = clean.split('/').filter(|s| !s.is_empty()).collect();
                let (parent_parts, name) = components.split_at(components.len() - 1);
                let mut root = self.root.lock();
                let parent = if parent_parts.is_empty() {
                    &mut *root
                } else {
                    match Self::resolve_path_mut(&mut root, parent_parts) {
                        Some(node) => node,
                        None => return FSResponse::Error("Directory not found".to_string()),
                    }
                };
                if parent.file_type != FileType::Directory {
                    return FSResponse::Error("Not a directory".to_string());
                }
                let children = parent.children.get_or_insert_with(BTreeMap::new);
                children.entry(name[0].to_string()).or_insert_with(|| VNode::new_dir(name[0]));
                FSResponse::Success
            }
            FSRequest::Delete { path } => {
                let resolve_path = if path.starts_with('/') {
                    path.clone()
                } else {
                    let cwd = self.current_dir.lock().clone();
                    if cwd == "/" {
                        format!("/{}", path)
                    } else {
                        format!("{}/{}", cwd, path)
                    }
                };
                let resolve_path = Self::normalize_path(&resolve_path);
                let clean = resolve_path.trim_start_matches('/');
                if clean.is_empty() {
                    return FSResponse::Error("Invalid path".to_string());
                }
                let components: Vec<&str> = clean.split('/').filter(|s| !s.is_empty()).collect();
                let (parent_parts, name) = components.split_at(components.len() - 1);
                let mut root = self.root.lock();
                let parent = if parent_parts.is_empty() {
                    &mut *root
                } else {
                    match Self::resolve_path_mut(&mut root, parent_parts) {
                        Some(node) => node,
                        None => return FSResponse::Error("Directory not found".to_string()),
                    }
                };
                if parent.file_type != FileType::Directory {
                    return FSResponse::Error("Not a directory".to_string());
                }
                if let Some(children) = parent.children.as_mut() {
                    if children.remove(name[0]).is_some() {
                        return FSResponse::Success;
                    }
                }
                FSResponse::Error("Not found".to_string())
            }
            FSRequest::ChangeDir { path } => {
                let resolve_path = if path.starts_with('/') {
                    path.clone()
                } else if path == ".." {
                    let cwd = self.current_dir.lock().clone();
                    if cwd == "/" {
                        "/".to_string()
                    } else {
                        let mut parts: Vec<&str> = cwd.split('/').filter(|s| !s.is_empty()).collect();
                        parts.pop();
                        if parts.is_empty() {
                            "/".to_string()
                        } else {
                            format!("/{}", parts.join("/"))
                        }
                    }
                } else if path == "." {
                    self.current_dir.lock().clone()
                } else {
                    let cwd = self.current_dir.lock().clone();
                    if cwd == "/" {
                        format!("/{}", path)
                    } else {
                        format!("{}/{}", cwd, path)
                    }
                };
                let resolve_path = Self::normalize_path(&resolve_path);
                
                if let Some(node) = self.resolve_path(&resolve_path) {
                    if node.file_type == FileType::Directory {
                        *self.current_dir.lock() = resolve_path;
                        FSResponse::Success
                    } else {
                        FSResponse::Error("Not a directory".to_string())
                    }
                } else {
                    FSResponse::Error("Directory not found".to_string())
                }
            }
            FSRequest::GetCwd => {
                let cwd = self.current_dir.lock();
                FSResponse::Cwd(cwd.clone())
            }
        }
    }
}

impl FileSystem for VFSService {
    fn open(&self, path: &str, flags: OpenFlags) -> Result<Box<dyn FileHandle>, FsError> {
        self.open_handle(path, flags)
    }
}

/// Global VFS instance
static VFS: spin::Mutex<Option<VFSService>> = spin::Mutex::new(None);

/// Initialize VFS service
pub fn init() {
    let mut vfs = VFS.lock();
    let service = VFSService::new();
    service.init();
    *vfs = Some(service);
}

/// Process VFS request
pub fn process_request(request: FSRequest) -> FSResponse {
    if let Some(ref vfs) = *VFS.lock() {
        vfs.process(request)
    } else {
        FSResponse::Error("VFS not initialized".to_string())
    }
}

pub fn open(path: &str, flags: u64) -> Result<Box<dyn FileHandle>, FsError> {
    let vfs = VFS.lock();
    let service = vfs.as_ref().ok_or(FsError::Invalid)?;
    service.open(path, OpenFlags::from_bits(flags))
}
