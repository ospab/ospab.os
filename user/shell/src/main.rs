#![no_std]
#![no_main]

mod syscall;

const COLS: usize = 80;
const ROWS: usize = 25;
const FG: u32 = 0x00E0E0E0;
const BG: u32 = 0x00000000;
const ACCENT: u32 = 0x00FFA500;
const INPUT_BUF_LEN: usize = 256;

static mut INPUT_BUF: [u8; INPUT_BUF_LEN] = [0; INPUT_BUF_LEN];

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    loop {}
}

#[no_mangle]
pub extern "C" fn _start() -> ! {
    let mut term = Terminal::new();
    term.clear();
    term.banner();

    loop {
        term.prompt();
        let len = term.read_line();
        if len == 0 {
            continue;
        }
        let line = unsafe { core::str::from_utf8_unchecked(&INPUT_BUF[..len]) };
        if !handle_command(line, &mut term) {
            term.write_str("unknown command. try: help\n");
        }
    }
}

fn handle_command(line: &str, term: &mut Terminal) -> bool {
    let mut parts = line.split_whitespace();
    let cmd = match parts.next() {
        Some(c) => c,
        None => return true,
    };

    match cmd {
        "help" => {
            term.write_str("commands: help clear echo ls cat cd pwd uptime version exec shutdown reboot doom tomato grape history\n");
            true
        }
        "clear" => {
            term.clear();
            true
        }
        "echo" => {
            if let Some(rest) = line.splitn(2, ' ').nth(1) {
                term.write_str(rest);
            }
            term.write_str("\n");
            true
        }
        "ls" => {
            let path = parts.next().unwrap_or(".");
            term.print_listdir(path);
            true
        }
        "cat" => {
            let path = match parts.next() {
                Some(p) => p,
                None => {
                    term.write_str("usage: cat <file>\n");
                    return true;
                }
            };
            term.print_file(path);
            true
        }
        "cd" => {
            let path = parts.next().unwrap_or("/");
            term.chdir(path);
            true
        }
        "pwd" => {
            term.print_cwd();
            true
        }
        "uptime" => {
            let ms = unsafe { syscall::uptime() };
            term.write_str("Uptime: ");
            term.write_u64(ms / 1000);
            term.write_str(" seconds\n");
            true
        }
        "version" => {
            term.write_str("ospabOS user shell (Ring3)\n");
            true
        }
        "shutdown" => unsafe {
            syscall::shutdown();
        },
        "reboot" => unsafe {
            syscall::reboot();
        },
        "doom" => {
            term.write_str("doom: not available in userland yet\n");
            true
        }
        "tomato" => {
            term.write_str("tomato: not implemented in userland yet\n");
            true
        }
        "grape" => {
            term.write_str("grape: not implemented in userland yet\n");
            true
        }
        "history" => {
            term.write_str("history: not available in userland yet\n");
            true
        }
        "exit" => unsafe {
            syscall::exit(0);
        },
        "exec" => {
            let path = match parts.next() {
                Some(p) => p,
                None => {
                    term.write_str("usage: exec /bin/app\n");
                    return true;
                }
            };
            let mut c_buf = [0u8; 256];
            let bytes = path.as_bytes();
            let count = core::cmp::min(bytes.len(), c_buf.len().saturating_sub(1));
            c_buf[..count].copy_from_slice(&bytes[..count]);
            c_buf[count] = 0;
            let ret = unsafe { syscall::exec(c_buf.as_ptr()) };
            if ret != 0 {
                term.write_str("exec failed\n");
            }
            true
        }
        _ => false,
    }
}

struct Terminal {
    row: usize,
    col: usize,
}

impl Terminal {
    fn new() -> Self {
        Self { row: 0, col: 0 }
    }

    fn banner(&mut self) {
        self.draw_bar();
        self.row = 2;
        self.col = 2;
        self.write_str("ospabshell — userland\n");
        self.write_str("type help to list commands\n");
        self.row += 1;
        self.col = 0;
    }

    fn draw_bar(&mut self) {
        self.row = 0;
        self.col = 0;
        for _ in 0..COLS {
            self.put_char(' ', ACCENT, ACCENT);
        }
        self.row = 1;
        self.col = 0;
        for _ in 0..COLS {
            self.put_char(' ', BG, BG);
        }
        self.row = 0;
        self.col = 2;
        self.write_str_colored("OSPAB OS", 0x00000000, ACCENT);
        self.row = 1;
        self.col = 0;
    }

    fn prompt(&mut self) {
        self.write_str_colored("ospab> ", ACCENT, BG);
    }

    fn clear(&mut self) {
        for r in 0..ROWS {
            for c in 0..COLS {
                unsafe { syscall::draw_char(c as u64, r as u64, ' ' as u64, BG as u64, BG as u64); }
            }
        }
        self.row = 2;
        self.col = 0;
    }

    fn read_line(&mut self) -> usize {
        let mut len = 0usize;
        loop {
            let mut ch: u8 = 0;
            let read = unsafe { syscall::read(0, &mut ch as *mut u8, 1) };
            if read == 0 {
                continue;
            }
            match ch {
                b'\r' | b'\n' => {
                    self.new_line();
                    unsafe { INPUT_BUF[len] = 0; }
                    return len;
                }
                8 | 127 => {
                    if len > 0 {
                        len -= 1;
                        self.backspace();
                    }
                }
                _ => {
                    if len + 1 >= INPUT_BUF_LEN {
                        continue;
                    }
                    unsafe { INPUT_BUF[len] = ch; }
                    len += 1;
                    self.put_char(ch as char, FG, BG);
                }
            }
        }
    }

    fn backspace(&mut self) {
        if self.col == 0 {
            return;
        }
        self.col -= 1;
        self.put_char(' ', BG, BG);
        self.col -= 1;
    }

    fn new_line(&mut self) {
        self.col = 0;
        if self.row + 1 < ROWS {
            self.row += 1;
        }
    }

    fn write_str(&mut self, s: &str) {
        self.write_str_colored(s, FG, BG);
    }

    fn write_str_colored(&mut self, s: &str, fg: u32, bg: u32) {
        for ch in s.chars() {
            if ch == '\n' {
                self.new_line();
            } else {
                self.put_char(ch, fg, bg);
            }
        }
    }

    fn write_u64(&mut self, mut value: u64) {
        if value == 0 {
            self.put_char('0', FG, BG);
            return;
        }
        let mut buf = [0u8; 20];
        let mut i = 0;
        while value > 0 {
            buf[i] = b'0' + (value % 10) as u8;
            value /= 10;
            i += 1;
        }
        while i > 0 {
            i -= 1;
            self.put_char(buf[i] as char, FG, BG);
        }
    }

    fn put_char(&mut self, ch: char, fg: u32, bg: u32) {
        if self.row >= ROWS {
            return;
        }
        unsafe {
            syscall::draw_char(self.col as u64, self.row as u64, ch as u64, fg as u64, bg as u64);
        }
        self.col += 1;
        if self.col >= COLS {
            self.col = 0;
            if self.row + 1 < ROWS {
                self.row += 1;
            }
        }
    }

    fn chdir(&mut self, path: &str) {
        let mut c_buf = [0u8; 256];
        let bytes = path.as_bytes();
        let count = core::cmp::min(bytes.len(), c_buf.len().saturating_sub(1));
        c_buf[..count].copy_from_slice(&bytes[..count]);
        c_buf[count] = 0;
        let ret = unsafe { syscall::chdir(c_buf.as_ptr()) };
        if ret != 0 {
            self.write_str("cd failed\n");
        }
    }

    fn print_cwd(&mut self) {
        let mut buf = [0u8; 256];
        let written = unsafe { syscall::getcwd(buf.as_mut_ptr(), buf.len()) } as usize;
        if written == 0 || written == !0usize {
            self.write_str("cwd unavailable\n");
            return;
        }
        let s = unsafe { core::str::from_utf8_unchecked(&buf[..written]) };
        self.write_str(s);
        self.write_str("\n");
    }

    fn print_listdir(&mut self, path: &str) {
        let mut path_buf = [0u8; 256];
        let bytes = path.as_bytes();
        let count = core::cmp::min(bytes.len(), path_buf.len().saturating_sub(1));
        path_buf[..count].copy_from_slice(&bytes[..count]);
        path_buf[count] = 0;

        let mut out = [0u8; 1024];
        let written = unsafe { syscall::listdir(path_buf.as_ptr(), out.as_mut_ptr(), out.len()) } as usize;
        if written == 0 || written == !0usize {
            self.write_str("ls failed\n");
            return;
        }
        let s = unsafe { core::str::from_utf8_unchecked(&out[..written]) };
        self.write_str(s);
        self.write_str("\n");
    }

    fn print_file(&mut self, path: &str) {
        let mut path_buf = [0u8; 256];
        let bytes = path.as_bytes();
        let count = core::cmp::min(bytes.len(), path_buf.len().saturating_sub(1));
        path_buf[..count].copy_from_slice(&bytes[..count]);
        path_buf[count] = 0;

        let fd = unsafe { syscall::open(path_buf.as_ptr(), 0) };
        if fd == !0 {
            self.write_str("open failed\n");
            return;
        }

        let mut buf = [0u8; 256];
        loop {
            let read = unsafe { syscall::read(fd, buf.as_mut_ptr(), buf.len()) } as usize;
            if read == 0 || read == !0usize {
                break;
            }
            let s = unsafe { core::str::from_utf8_unchecked(&buf[..read]) };
            self.write_str(s);
        }
        self.write_str("\n");
    }
}
