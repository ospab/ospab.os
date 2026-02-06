//! Shell - Command interpreter that dispatches messages to services

pub mod task; // v0.1.0: Shell as background task

use alloc::string::ToString;
use alloc::vec::Vec;
use alloc::format;
use crate::ipc::message::FSRequest;
use crate::services::vfs;
use crate::drivers::framebuffer;
use crate::apps::coreutils;

/// Get formatted prompt string with current directory
pub fn get_prompt() -> alloc::string::String {
    use alloc::format;
    
    let response = vfs::process_request(FSRequest::GetCwd);
    let cwd = match response {
        crate::ipc::message::FSResponse::Cwd(path) => path,
        _ => "/".to_string(),
    };
    
    // Format directory for prompt
    let dir_display = format_directory(&cwd);
    
    format!("[ospab:{}]$ ", dir_display)
}

/// Format directory path for prompt display
fn format_directory(path: &str) -> alloc::string::String {
    
    // Home directory (/home/user) shows as ~
    if path == "/home/user" {
        return "~".to_string();
    }
    
    // Root directory shows as /
    if path == "/" {
        return "/".to_string();
    }
    
    // For paths starting with /home/user/, replace with ~
    if path.starts_with("/home/user/") {
        return path.replacen("/home/user", "~", 1);
    }
    
    // For long paths, show only last 2-3 components
    let components: Vec<&str> = path.split('/').filter(|s| !s.is_empty()).collect();
    
    if components.len() <= 2 {
        // Short path - show as is
        path.to_string()
    } else {
        // Long path - show as .../parent/current
        use alloc::format;
        let len = components.len();
        format!(".../{}/{}", components[len-2], components[len-1])
    }
}

fn resolve_command_path(cmd: &str) -> alloc::string::String {
    if cmd.contains('/') {
        cmd.to_string()
    } else {
        format!("/bin/{}", cmd)
    }
}

pub fn exec_path(path: &str) -> Result<(), &'static str> {
    let response = vfs::process_request(FSRequest::ReadFile { path: path.to_string() });
    let data = match response {
        crate::ipc::message::FSResponse::FileData(data) => data,
        crate::ipc::message::FSResponse::Error(_) => return Err("file not found"),
        _ => return Err("unexpected response"),
    };

    if data.starts_with(b"#!") {
        if let Ok(text) = core::str::from_utf8(&data) {
            run_script(text);
            return Ok(());
        }
        return Err("invalid script encoding");
    }

    if data.starts_with(b"\x7FELF") {
        framebuffer::print("ELF exec not implemented yet\n");
        return Err("elf not supported");
    }

    if let Ok(text) = core::str::from_utf8(&data) {
        run_script(text);
        return Ok(());
    }

    Err("unknown file format")
}

fn run_script(content: &str) {
    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        execute_command(trimmed);
    }
}

/// Execute shell command
pub fn execute_command(cmd: &str) {
    let parts: Vec<&str> = cmd.split_whitespace().collect();
    if parts.is_empty() {
        return;
    }

    match parts[0] {
        "help" => {
            framebuffer::print("ospabOS v0.1.0 \"Foundation\" - Available commands:\n");
            framebuffer::print("  help     - Show this help\n");
            framebuffer::print("  clear    - Clear screen\n");
            framebuffer::print("  echo     - Echo text\n");
            framebuffer::print("  uptime   - Show system uptime\n");
            framebuffer::print("  version  - Show kernel version\n");
            framebuffer::print("  history  - Show command history\n");
            framebuffer::print("  ls       - List directory (initrd)\n");
            framebuffer::print("  cat      - Display file contents\n");
            framebuffer::print("  grape    - Text editor (^G=help)\n");
            framebuffer::print("  cd       - Change directory (VFS)\n");
            framebuffer::print("  pwd      - Print working directory\n");
            framebuffer::print("  tomato   - Package manager\n");
            framebuffer::print("  doom     - Run DOOM\n");
            framebuffer::print("  shutdown - Shutdown system\n");
            framebuffer::print("  reboot   - Reboot system\n");
        }
        "clear" => {
            framebuffer::clear();
        }
        "echo" => {
            if parts.len() > 1 {
                let text = parts[1..].join(" ");
                framebuffer::print(&text);
                framebuffer::print_char('\n');
            }
        }
        "uptime" => {
            use crate::drivers::timer;
            let uptime_ms = timer::get_uptime_ms();
            let uptime_s = uptime_ms / 1000;
            framebuffer::print("Uptime: ");
            print_num(uptime_s);
            framebuffer::print(" seconds\n");
        }
        "version" => {
            framebuffer::print("ospabOS v0.1.0 \"Foundation\"\n");
            framebuffer::print("Preemptive multitasking + Syscall interface + VMM\n");
            framebuffer::print("Message-passing architecture with IPC\n");
        }
        "history" => {
            use crate::drivers::keyboard;
            keyboard::print_history();
        }
        "ls" => {
            let path = if parts.len() > 1 { parts[1] } else { "." };
            match coreutils::ls(path) {
                Ok(entries) => {
                    if entries.is_empty() {
                        framebuffer::print("(empty directory)\n");
                    } else {
                        for entry in entries {
                            framebuffer::print(&entry);
                            framebuffer::print_char('\n');
                        }
                    }
                }
                Err(msg) => {
                    framebuffer::print("Error: ");
                    framebuffer::print(&msg);
                    framebuffer::print_char('\n');
                }
            }
        }
        "cd" => {
            if parts.len() > 1 {
                let mut path = parts[1].to_string();
                if path == "~" {
                    path = "/home/user".to_string();
                } else if let Some(rest) = path.strip_prefix("~/") {
                    path = format!("/home/user/{}", rest);
                }
                let response = vfs::process_request(FSRequest::ChangeDir { path });
                match response {
                    crate::ipc::message::FSResponse::Success => {}
                    crate::ipc::message::FSResponse::Error(msg) => {
                        framebuffer::print("Error: ");
                        framebuffer::print(&msg);
                        framebuffer::print_char('\n');
                    }
                    _ => {}
                }
            } else {
                framebuffer::print("Usage: cd <directory>\n");
            }
        }
        "pwd" => {
            let response = vfs::process_request(FSRequest::GetCwd);
            match response {
                crate::ipc::message::FSResponse::Cwd(path) => {
                    framebuffer::print(&path);
                    framebuffer::print_char('\n');
                }
                _ => {}
            }
        }
        "cat" => {
            if parts.len() < 2 {
                framebuffer::print("Usage: cat <filename>\n");
                return;
            }
            let filename = parts[1];
            match coreutils::cat(filename) {
                Ok(data) => {
                    if let Ok(text) = core::str::from_utf8(&data) {
                        framebuffer::print(text);
                        if !text.ends_with('\n') {
                            framebuffer::print_char('\n');
                        }
                    } else {
                        framebuffer::print("(binary file, ");
                        print_num(data.len() as u64);
                        framebuffer::print(" bytes)\n");
                    }
                }
                Err(msg) => {
                    framebuffer::print("Error: ");
                    framebuffer::print(&msg);
                    framebuffer::print_char('\n');
                }
            }
        }
        "mkdir" => {
            if parts.len() < 2 {
                framebuffer::print("Usage: mkdir <dir>\n");
                return;
            }
            match coreutils::mkdir(parts[1]) {
                Ok(_) => {}
                Err(msg) => {
                    framebuffer::print("Error: ");
                    framebuffer::print(&msg);
                    framebuffer::print_char('\n');
                }
            }
        }
        "cp" => {
            if parts.len() < 3 {
                framebuffer::print("Usage: cp <src> <dst>\n");
                return;
            }
            match coreutils::cp(parts[1], parts[2]) {
                Ok(_) => {}
                Err(msg) => {
                    framebuffer::print("Error: ");
                    framebuffer::print(&msg);
                    framebuffer::print_char('\n');
                }
            }
        }
        "mv" => {
            if parts.len() < 3 {
                framebuffer::print("Usage: mv <src> <dst>\n");
                return;
            }
            match coreutils::mv(parts[1], parts[2]) {
                Ok(_) => {}
                Err(msg) => {
                    framebuffer::print("Error: ");
                    framebuffer::print(&msg);
                    framebuffer::print_char('\n');
                }
            }
        }
        "grape" => {
            if parts.len() < 2 {
                framebuffer::print("Usage: grape <filename>\n");
                framebuffer::print("Commands:\n");
                framebuffer::print("  ^G (Ctrl+G) - Help\n");
                framebuffer::print("  ^X (Ctrl+X) - Save\n");
                framebuffer::print("  ^C (Ctrl+C) - Exit\n");
                framebuffer::print("  ^W (Ctrl+W) - Search\n");
                framebuffer::print("  ^K (Ctrl+K) - Cut\n");
                framebuffer::print("  ^U (Ctrl+U) - Paste\n");
                return;
            }
            let filename = parts[1];
            match crate::grape::open(filename) {
                Ok(_) => {}
                Err(e) => {
                    framebuffer::print("Error opening file: ");
                    framebuffer::print(&e);
                    framebuffer::print_char('\n');
                }
            }
        }
        "tomato" => {
            if parts.len() < 2 {
                framebuffer::print("Usage: tomato <install|remove|update|list|search> [package]\n");
                return;
            }
            match parts[1] {
                "list" => {
                    framebuffer::print("Installed packages:\n");
                    framebuffer::print("  (none - package manager not yet implemented)\n");
                }
                "install" | "remove" | "update" | "search" => {
                    framebuffer::print("Package manager not yet implemented\n");
                }
                _ => {
                    framebuffer::print("Unknown tomato command\n");
                }
            }
        }
        "doom" => {
            framebuffer::print("Starting DOOM...\n");
            framebuffer::print("(Ctrl+C to exit)\n\n");
            // Small delay to show message
            for _ in 0..5000000 {
                core::hint::spin_loop();
            }
            crate::doom::run_demo();
        }
        "shutdown" => {
            crate::power::shutdown();
        }
        "reboot" => {
            crate::power::reboot();
        }
        _ => {
            let path = resolve_command_path(parts[0]);
            if exec_path(&path).is_err() {
                framebuffer::print("Unknown command: ");
                framebuffer::print(parts[0]);
                framebuffer::print("\n");
            }
        }
    }
}

// Helper to print numbers
fn print_num(n: u64) {
    if n == 0 {
        framebuffer::print_char('0');
        return;
    }
    
    let mut buf = [0u8; 20];
    let mut i = 0;
    let mut num = n;
    
    while num > 0 {
        buf[i] = b'0' + (num % 10) as u8;
        num /= 10;
        i += 1;
    }
    
    for j in (0..i).rev() {
        framebuffer::print_char(buf[j] as char);
    }
}
