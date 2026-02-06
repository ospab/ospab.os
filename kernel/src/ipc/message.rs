//! Message types for IPC communication

use alloc::string::String;
use alloc::vec::Vec;

/// Main message enum for inter-service communication
#[derive(Debug, Clone)]
pub enum Message {
    /// Filesystem requests
    FS(FSRequest),
    /// UI/Terminal requests
    UI(UIRequest),
    /// Package manager requests
    Pkg(PkgRequest),
    /// System control
    System(SystemRequest),
}

/// Filesystem operations
#[derive(Debug, Clone)]
pub enum FSRequest {
    /// List directory contents
    ListDir { path: String },
    /// Read file contents
    ReadFile { path: String },
    /// Write file contents
    WriteFile { path: String, data: Vec<u8> },
    /// Create directory
    CreateDir { path: String },
    /// Delete file/directory
    Delete { path: String },
    /// Change current directory
    ChangeDir { path: String },
    /// Get current working directory
    GetCwd,
}

/// Filesystem response
#[derive(Debug, Clone)]
pub enum FSResponse {
    /// List of entries
    DirListing(Vec<String>),
    /// File contents
    FileData(Vec<u8>),
    /// Success confirmation
    Success,
    /// Error message
    Error(String),
    /// Current working directory
    Cwd(String),
}

/// UI/Terminal operations
#[derive(Debug, Clone)]
pub enum UIRequest {
    /// Print text to terminal
    Print(String),
    /// Print line to terminal
    PrintLn(String),
    /// Clear screen
    Clear,
    /// Set cursor position
    SetCursor { x: usize, y: usize },
    /// Get input line
    ReadLine,
}

/// Package manager operations
#[derive(Debug, Clone)]
pub enum PkgRequest {
    /// Install package
    Install { name: String },
    /// Remove package
    Remove { name: String },
    /// Update package database
    Update,
    /// List installed packages
    List,
    /// Search for package
    Search { query: String },
}

/// Package manager response
#[derive(Debug, Clone)]
pub enum PkgResponse {
    /// Success message
    Success(String),
    /// Error message
    Error(String),
    /// List of packages
    PackageList(Vec<String>),
}

/// System control operations
#[derive(Debug, Clone)]
pub enum SystemRequest {
    /// Shutdown system
    Shutdown,
    /// Reboot system
    Reboot,
    /// Get system info
    GetInfo,
}
