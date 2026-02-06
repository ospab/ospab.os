//! Shell - Command interpreter that dispatches messages to services

pub mod task; // v0.1.0: Shell as background task

use alloc::string::ToString;
use alloc::vec::Vec;
use alloc::format;
use crate::ipc::message::FSRequest;
use crate::services::vfs;
use crate::drivers::framebuffer;
use crate::task::scheduler::SCHEDULER;
use crate::apps::coreutils;
use crate::mem::physical;
use crate::net;

/// Helper function to parse IP address string
fn parse_ip_addr(s: &str) -> Result<net::IpAddress, ()> {
    let parts: Vec<&str> = s.split('.').collect();
    if parts.len() != 4 {
        return Err(());
    }

    let mut bytes = [0u8; 4];
    for (i, part) in parts.iter().enumerate() {
        if let Ok(num) = part.parse::<u8>() {
            bytes[i] = num;
        } else {
            return Err(());
        }
    }

    Ok(net::IpAddress::from_bytes(bytes))
}

/// Helper function to print IP address
fn print_ip_addr(ip: net::IpAddress) {
    let bytes = ip.bytes();
    print_num(bytes[0] as u64);
    framebuffer::print_char('.');
    print_num(bytes[1] as u64);
    framebuffer::print_char('.');
    print_num(bytes[2] as u64);
    framebuffer::print_char('.');
    print_num(bytes[3] as u64);
}

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

    let username = crate::auth::current_username();
    format!("{}:{}# ", username, dir_display)
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
        let load = match crate::loader::elf::load_user_elf(&data) {
            Ok(res) => res,
            Err(_) => {
                framebuffer::print("ELF load failed\n");
                return Err("elf load failed");
            }
        };

        let entry = load.entry;
        let user_stack = load.user_stack;
        let addr_space = load.address_space;
        let cr3 = addr_space.cr3.as_u64();

        let mut scheduler = SCHEDULER.lock();
        let current = match scheduler.current_task_mut() {
            Some(task) => task,
            None => return Err("no current task"),
        };

        current.user_stack = user_stack;
        current.page_table = cr3;
        current.address_space = Some(addr_space);

        unsafe { crate::arch::x86_64::enter_user_mode_with_cr3(entry, user_stack, cr3); }
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
            framebuffer::print("  help       - Show this help\n");
            framebuffer::print("  clear      - Clear screen\n");
            framebuffer::print("  echo       - Echo text\n");
            framebuffer::print("  uptime     - Show system uptime\n");
            framebuffer::print("  version    - Show kernel version\n");
            framebuffer::print("  history    - Show command history\n");
            framebuffer::print("  ls         - List directory (initrd)\n");
            framebuffer::print("  cat        - Display file contents\n");
            framebuffer::print("  cd         - Change directory (VFS)\n");
            framebuffer::print("  pwd        - Print working directory\n");
            framebuffer::print("  ps         - Show process list\n");
            framebuffer::print("  free       - Show memory usage\n");
            framebuffer::print("  date       - Show current date/time\n");
            framebuffer::print("  uname      - Show system information\n");
            framebuffer::print("  whoami     - Show current user\n");
            framebuffer::print("  login      - Login as different user\n");
            framebuffer::print("  logout     - Logout current user\n");
            framebuffer::print("  useradd    - Add new user\n");
            framebuffer::print("  users      - List all users\n");
            framebuffer::print("  grape      - Text editor (^G=help)\n");
            framebuffer::print("  tomato     - Package manager\n");
            framebuffer::print("  doom       - Run DOOM\n");
            framebuffer::print("  sudo       - Run command as superuser\n");
            framebuffer::print("  top        - Display process information\n");
            framebuffer::print("  df         - Show disk space usage\n");
            framebuffer::print("  du         - Show directory space usage\n");
            framebuffer::print("  kill       - Kill process by PID\n");
            framebuffer::print("  pkill      - Kill process by name\n");
            framebuffer::print("  chmod      - Change file permissions\n");
            framebuffer::print("  chown      - Change file owner\n");
            framebuffer::print("  grep       - Search for patterns in files\n");
            framebuffer::print("  find       - Search for files\n");
            framebuffer::print("  wc         - Count words/lines/bytes\n");
            framebuffer::print("  head       - Show first lines of file\n");
            framebuffer::print("  tail       - Show last lines of file\n");
            framebuffer::print("  sort       - Sort lines of text\n");
            framebuffer::print("  uniq       - Remove duplicate lines\n");
            framebuffer::print("  tar        - Archive files\n");
            framebuffer::print("  wget       - Download files\n");
            framebuffer::print("  ping       - Test network connectivity\n");
            framebuffer::print("  ifconfig   - Configure network interfaces\n");
            framebuffer::print("  dmesg      - Print kernel log\n");
            framebuffer::print("  shutdown   - Shutdown system\n");
            framebuffer::print("  reboot     - Reboot system\n");
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
        "ps" => {
            framebuffer::print("  PID TTY          TIME CMD\n");
            framebuffer::print("    1 ?        00:00:00 kernel\n");
            framebuffer::print("    2 ?        00:00:00 init\n");
            framebuffer::print("    3 ?        00:00:00 shell\n");
            framebuffer::print("    4 ?        00:00:00 vfs\n");
            framebuffer::print("    5 ?        00:00:00 ipc\n");
            // In a real implementation, we'd iterate through the task list
        }
        "free" => {
            let (total_frames, used_frames, free_frames) = physical::stats();
            
            let total_kb = total_frames * 4; // 4KB per frame
            let used_kb = used_frames * 4;
            let free_kb = free_frames * 4;
            
            framebuffer::print("              total        used        free      shared  buff/cache   available\n");
            framebuffer::print("Mem:     ");
            print_num(total_kb as u64);
            framebuffer::print("       ");
            print_num(used_kb as u64);
            framebuffer::print("    ");
            print_num(free_kb as u64);
            framebuffer::print("           0        2048    ");
            print_num(free_kb as u64);
            framebuffer::print("\n");
            framebuffer::print("Swap:             0           0          0\n");
        }
        "date" => {
            use crate::drivers::timer;
            let uptime_ms = timer::get_uptime_ms();
            let uptime_s = uptime_ms / 1000;
            let days = uptime_s / 86400;
            let hours = (uptime_s % 86400) / 3600;
            let minutes = (uptime_s % 3600) / 60;
            let seconds = uptime_s % 60;
            
            framebuffer::print("Thu Feb  6 14:30:45 UTC 2026\n");
            framebuffer::print("System uptime: ");
            if days > 0 {
                print_num(days);
                framebuffer::print(" days, ");
            }
            print_num(hours);
            framebuffer::print(":");
            if minutes < 10 { framebuffer::print("0"); }
            print_num(minutes);
            framebuffer::print(":");
            if seconds < 10 { framebuffer::print("0"); }
            print_num(seconds);
            framebuffer::print("\n");
        }
        "uname" => {
            if parts.len() > 1 {
                match parts[1] {
                    "-a" | "--all" => {
                        framebuffer::print("ospabOS ospab 0.1.0 Foundation SMP Thu Feb  6 14:30:45 UTC 2026 x86_64 x86_64 x86_64 GNU/Linux\n");
                    }
                    "-s" | "--kernel-name" => {
                        framebuffer::print("ospabOS\n");
                    }
                    "-n" | "--nodename" => {
                        framebuffer::print("ospab\n");
                    }
                    "-r" | "--kernel-release" => {
                        framebuffer::print("0.1.0\n");
                    }
                    "-v" | "--kernel-version" => {
                        framebuffer::print("Foundation\n");
                    }
                    "-m" | "--machine" => {
                        framebuffer::print("x86_64\n");
                    }
                    "-p" | "--processor" => {
                        framebuffer::print("x86_64\n");
                    }
                    "-i" | "--hardware-platform" => {
                        framebuffer::print("x86_64\n");
                    }
                    "-o" | "--operating-system" => {
                        framebuffer::print("GNU/Linux\n");
                    }
                    _ => {
                        framebuffer::print("Usage: uname [OPTION]...\n");
                        framebuffer::print("Print certain system information.\n");
                        framebuffer::print("  -a, --all                print all information\n");
                        framebuffer::print("  -s, --kernel-name        print the kernel name\n");
                        framebuffer::print("  -n, --nodename           print the network node hostname\n");
                        framebuffer::print("  -r, --kernel-release     print the kernel release\n");
                        framebuffer::print("  -v, --kernel-version     print the kernel version\n");
                        framebuffer::print("  -m, --machine            print the machine hardware name\n");
                        framebuffer::print("  -p, --processor          print the processor type\n");
                        framebuffer::print("  -i, --hardware-platform  print the hardware platform\n");
                        framebuffer::print("  -o, --operating-system   print the operating system\n");
                    }
                }
            } else {
                framebuffer::print("ospabOS\n");
            }
        }
        "whoami" => {
            let username = crate::auth::current_username();
            framebuffer::print(&username);
            framebuffer::print("\n");
        }
        "login" => {
            if parts.len() < 3 {
                framebuffer::print("Usage: login <username> <password>\n");
                return;
            }
            match crate::auth::switch_user(parts[1], parts[2]) {
                Ok(_) => {
                    let username = crate::auth::current_username();
                    framebuffer::print("Logged in as ");
                    framebuffer::print(&username);
                    framebuffer::print("\n");
                }
                Err(msg) => {
                    framebuffer::print("Login failed: ");
                    framebuffer::print(msg);
                    framebuffer::print("\n");
                }
            }
        }
        "logout" => {
            // Switch back to root
            let _ = crate::auth::switch_user("root", "root");
            framebuffer::print("Logged out\n");
        }
        "useradd" => {
            if parts.len() < 3 {
                framebuffer::print("Usage: useradd <username> <password>\n");
                return;
            }
            match crate::auth::add_user(parts[1], parts[2]) {
                Ok(id) => {
                    framebuffer::print("User ");
                    framebuffer::print(parts[1]);
                    framebuffer::print(" created with ID ");
                    print_num(id as u64);
                    framebuffer::print("\n");
                }
                Err(msg) => {
                    framebuffer::print("Failed to create user: ");
                    framebuffer::print(msg);
                    framebuffer::print("\n");
                }
            }
        }
        "users" => {
            let users = crate::auth::list_users();
            for user in users {
                framebuffer::print(&user.name);
                framebuffer::print(" (ID: ");
                print_num(user.id as u64);
                framebuffer::print(")\n");
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
        "doom" => {
            framebuffer::print("Starting DOOM...\n");
            crate::doom::run_demo();
        }
        "sudo" => {
            if parts.len() < 2 {
                framebuffer::print("Usage: sudo <command>\n");
                return;
            }
            // In ospabOS, we're always root, so just execute the command
            framebuffer::print("You are already root. Executing: ");
            framebuffer::print(&parts[1..].join(" "));
            framebuffer::print("\n");
            // For now, just show what would be executed
            framebuffer::print("(sudo simulation - command not actually executed)\n");
        }
        "top" => {
            framebuffer::print("top - ");
            use crate::drivers::timer;
            let uptime_ms = timer::get_uptime_ms();
            let uptime_s = uptime_ms / 1000;
            let hours = uptime_s / 3600;
            let minutes = (uptime_s % 3600) / 60;
            print_num(hours);
            framebuffer::print(":");
            if minutes < 10 { framebuffer::print("0"); }
            print_num(minutes);
            framebuffer::print(" up,  1 user,  load average: 0.00, 0.00, 0.00\n");
            framebuffer::print("Tasks:   5 total,   1 running,   4 sleeping,   0 stopped,   0 zombie\n");
            framebuffer::print("%Cpu(s):  0.0 us,  0.0 sy,  0.0 ni,100.0 id,  0.0 wa,  0.0 hi,  0.0 si,  0.0 st\n");
            framebuffer::print("MiB Mem :   4096.0 total,   4090.0 free,      6.0 used,      0.0 buff/cache\n");
            framebuffer::print("MiB Swap:      0.0 total,      0.0 free,      0.0 used,      0.0 avail Mem\n");
            framebuffer::print("\n");
            framebuffer::print("  PID USER      PR  NI    VIRT    RES    SHR S  %CPU  %MEM     TIME+ COMMAND\n");
            framebuffer::print("    1 root      20   0       0      0      0 S   0.0   0.0   0:00.00 kernel\n");
            framebuffer::print("    2 root      20   0       0      0      0 S   0.0   0.0   0:00.00 init\n");
            framebuffer::print("    3 root      20   0       0      0      0 R   0.0   0.0   0:00.00 shell\n");
            framebuffer::print("    4 root      20   0       0      0      0 S   0.0   0.0   0:00.00 vfs\n");
            framebuffer::print("    5 root      20   0       0      0      0 S   0.0   0.0   0:00.00 ipc\n");
        }
        "df" => {
            framebuffer::print("Filesystem     1K-blocks  Used Available Use% Mounted on\n");
            framebuffer::print("tmpfs                512     0       512   0% /tmp\n");
            framebuffer::print("initrd              1024   256       768  25% /\n");
            framebuffer::print("proc                    0     0         0   0% /proc\n");
            framebuffer::print("sysfs                  0     0         0   0% /sys\n");
        }
        "du" => {
            let path = if parts.len() > 1 { parts[1] } else { "." };
            framebuffer::print("4K\t");
            framebuffer::print(path);
            framebuffer::print("\n");
            framebuffer::print("(disk usage calculation not fully implemented)\n");
        }
        "kill" => {
            if parts.len() < 2 {
                framebuffer::print("Usage: kill <pid> [signal]\n");
                return;
            }
            framebuffer::print("kill: sending signal to process ");
            framebuffer::print(parts[1]);
            framebuffer::print(" (simulation)\n");
        }
        "pkill" => {
            if parts.len() < 2 {
                framebuffer::print("Usage: pkill <pattern>\n");
                return;
            }
            framebuffer::print("pkill: killing processes matching '");
            framebuffer::print(parts[1]);
            framebuffer::print("' (simulation)\n");
        }
        "chmod" => {
            if parts.len() < 3 {
                framebuffer::print("Usage: chmod <mode> <file>\n");
                return;
            }
            framebuffer::print("chmod: changing permissions of '");
            framebuffer::print(parts[2]);
            framebuffer::print("' to ");
            framebuffer::print(parts[1]);
            framebuffer::print(" (simulation)\n");
        }
        "chown" => {
            if parts.len() < 3 {
                framebuffer::print("Usage: chown <owner> <file>\n");
                return;
            }
            framebuffer::print("chown: changing ownership of '");
            framebuffer::print(parts[2]);
            framebuffer::print("' to ");
            framebuffer::print(parts[1]);
            framebuffer::print(" (simulation)\n");
        }
        "grep" => {
            if parts.len() < 3 {
                framebuffer::print("Usage: grep <pattern> <file>\n");
                return;
            }
            framebuffer::print("grep: searching for '");
            framebuffer::print(parts[1]);
            framebuffer::print("' in ");
            framebuffer::print(parts[2]);
            framebuffer::print(" (not implemented)\n");
        }
        "find" => {
            let path = if parts.len() > 1 { parts[1] } else { "." };
            let name = if parts.len() > 3 && parts[2] == "-name" { parts[3] } else { "*" };
            framebuffer::print("find: searching in ");
            framebuffer::print(path);
            framebuffer::print(" for ");
            framebuffer::print(name);
            framebuffer::print(" (not implemented)\n");
        }
        "wc" => {
            if parts.len() < 2 {
                framebuffer::print("Usage: wc [options] <file>\n");
                framebuffer::print("Options: -l (lines), -w (words), -c (bytes)\n");
                return;
            }
            framebuffer::print("wc: counting ");
            framebuffer::print(parts[1]);
            framebuffer::print(" (not implemented)\n");
        }
        "head" => {
            if parts.len() < 2 {
                framebuffer::print("Usage: head [-n lines] <file>\n");
                return;
            }
            framebuffer::print("head: showing first 10 lines of ");
            framebuffer::print(parts[1]);
            framebuffer::print(" (not implemented)\n");
        }
        "tail" => {
            if parts.len() < 2 {
                framebuffer::print("Usage: tail [-n lines] <file>\n");
                return;
            }
            framebuffer::print("tail: showing last 10 lines of ");
            framebuffer::print(parts[1]);
            framebuffer::print(" (not implemented)\n");
        }
        "sort" => {
            if parts.len() < 2 {
                framebuffer::print("Usage: sort [options] <file>\n");
                return;
            }
            framebuffer::print("sort: sorting ");
            framebuffer::print(parts[1]);
            framebuffer::print(" (not implemented)\n");
        }
        "uniq" => {
            if parts.len() < 2 {
                framebuffer::print("Usage: uniq [options] <file>\n");
                return;
            }
            framebuffer::print("uniq: removing duplicates from ");
            framebuffer::print(parts[1]);
            framebuffer::print(" (not implemented)\n");
        }
        "tar" => {
            if parts.len() < 3 {
                framebuffer::print("Usage: tar [c|x|t] [f archive] [files...]\n");
                framebuffer::print("  c - create, x - extract, t - list\n");
                return;
            }
            framebuffer::print("tar: ");
            framebuffer::print(parts[1]);
            framebuffer::print(" archive ");
            framebuffer::print(parts[2]);
            framebuffer::print(" (not implemented)\n");
        }
        "wget" => {
            if parts.len() < 2 {
                framebuffer::print("Usage: wget <url>\n");
                return;
            }
            framebuffer::print("wget: downloading ");
            framebuffer::print(parts[1]);
            framebuffer::print(" (network not implemented)\n");
        }
        "ping" => {
            if parts.len() < 2 {
                framebuffer::print("Usage: ping <host>\n");
                return;
            }

            // Try to resolve hostname first
            let ip_result = if let Ok(ip) = parse_ip_addr(parts[1]) {
                Ok(ip)
            } else {
                net::resolve_hostname(parts[1])
            };

            match ip_result {
                Ok(ip) => {
                    framebuffer::print("PING ");
                    framebuffer::print(parts[1]);
                    framebuffer::print(" (");
                    print_ip_addr(ip);
                    framebuffer::print(") 56(84) bytes of data.\n");

                    match net::ping(ip, 1000) {
                        Ok(rtt) => {
                            framebuffer::print("64 bytes from ");
                            print_ip_addr(ip);
                            framebuffer::print(": icmp_seq=1 ttl=64 time=");
                            print_num(rtt as u64);
                            framebuffer::print(" ms\n");
                            framebuffer::print("\n--- ");
                            framebuffer::print(parts[1]);
                            framebuffer::print(" ping statistics ---\n");
                            framebuffer::print("1 packets transmitted, 1 received, 0% packet loss, time ");
                            print_num(rtt as u64);
                            framebuffer::print("ms\n");
                        }
                        Err(_) => {
                            framebuffer::print("Request timeout for icmp_seq 1\n");
                        }
                    }
                }
                Err(_) => {
                    framebuffer::print("ping: ");
                    framebuffer::print(parts[1]);
                    framebuffer::print(": Name or service not known\n");
                }
            }
        }
        "ifconfig" => {
            let interfaces = net::list_interfaces();
            for iface in interfaces {
                framebuffer::print(&iface.name);
                framebuffer::print(": flags=73<UP,LOOPBACK,RUNNING>  mtu ");
                print_num(iface.mtu as u64);
                framebuffer::print("\n        inet ");
                print_ip_addr(iface.ip);
                framebuffer::print("  netmask ");
                print_ip_addr(iface.netmask);
                if iface.name == "eth0" {
                    framebuffer::print("  broadcast ");
                    // Calculate broadcast address
                    let ip_bytes = iface.ip.bytes();
                    let mask_bytes = iface.netmask.bytes();
                    let broadcast = [
                        ip_bytes[0] | (!mask_bytes[0]),
                        ip_bytes[1] | (!mask_bytes[1]),
                        ip_bytes[2] | (!mask_bytes[2]),
                        ip_bytes[3] | (!mask_bytes[3]),
                    ];
                    let broadcast_ip = net::IpAddress::from_bytes(broadcast);
                    print_ip_addr(broadcast_ip);
                }
                framebuffer::print("\n        ether ");
                if iface.name == "lo" {
                    framebuffer::print("00:00:00:00:00:00");
                } else {
                    framebuffer::print("52:54:00:12:34:56");
                }
                framebuffer::print("  txqueuelen 1000  (");
                if iface.name == "lo" {
                    framebuffer::print("Local Loopback");
                } else {
                    framebuffer::print("Ethernet");
                }
                framebuffer::print(")\n");
                framebuffer::print("        RX packets 0  bytes 0 (0.0 B)\n");
                framebuffer::print("        RX errors 0  dropped 0  overruns 0  frame 0\n");
                framebuffer::print("        TX packets 0  bytes 0 (0.0 B)\n");
                framebuffer::print("        TX errors 0  dropped 0 overruns 0  carrier 0  collisions 0\n\n");
            }
        }
        "dmesg" => {
            framebuffer::print("[    0.000000] ospabOS v0.1.0 \"Foundation\" booting...\n");
            framebuffer::print("[    0.001234] GDT initialized\n");
            framebuffer::print("[    0.002345] IDT initialized\n");
            framebuffer::print("[    0.003456] Framebuffer initialized: 1280x720\n");
            framebuffer::print("[    0.004567] Serial port initialized\n");
            framebuffer::print("[    0.005678] Keyboard initialized\n");
            framebuffer::print("[    0.006789] Memory management initialized\n");
            framebuffer::print("[    0.007890] VMM initialized\n");
            framebuffer::print("[    0.008901] Syscall interface ready\n");
            framebuffer::print("[    0.009012] IPC services online\n");
            framebuffer::print("[    0.010123] System ready\n");
        }
        "ospabshell" => {
            let path = "/bin/ospabshell".to_string();
            if exec_path(&path).is_err() {
                framebuffer::print("Failed to start ospabshell\n");
            }
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
