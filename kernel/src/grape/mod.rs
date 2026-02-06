//! Grape Text Editor - Simple nano-like editor for ospabOS

use alloc::string::{String, ToString};
use alloc::vec::Vec;
use alloc::format;
use crate::drivers::framebuffer;
use crate::services::vfs;
use crate::ipc::message::{FSRequest, FSResponse};

/// Grape editor state
pub struct GrapeEditor {
    filename: String,
    lines: Vec<String>,
    cursor_row: usize,
    cursor_col: usize,
    scroll_offset: usize,
    modified: bool,
    message: Option<String>,
    max_rows: usize,  // Visible rows (screen height - status bar)
}

impl GrapeEditor {
    /// Create new editor instance
    pub fn new(filename: &str, max_rows: usize) -> Self {
        Self {
            filename: filename.to_string(),
            lines: Vec::new(),
            cursor_row: 0,
            cursor_col: 0,
            scroll_offset: 0,
            modified: false,
            message: None,
            max_rows: max_rows.saturating_sub(2), // Reserve 2 lines for status
        }
    }
    
    /// Load file from VFS
    pub fn load_file(&mut self) -> Result<(), String> {
        let response = vfs::process_request(FSRequest::ReadFile { 
            path: self.filename.clone() 
        });
        
        match response {
            FSResponse::FileData(data) => {
                // Parse file into lines
                if let Ok(text) = core::str::from_utf8(&data) {
                    self.lines = text.lines().map(|s| s.to_string()).collect();
                    if self.lines.is_empty() {
                        self.lines.push(String::new());
                    }
                    Ok(())
                } else {
                    Err("File is not valid UTF-8".to_string())
                }
            }
            FSResponse::Error(msg) => {
                // File doesn't exist - start with empty buffer
                self.lines.push(String::new());
                Err(msg)
            }
            _ => Err("Unexpected response".to_string())
        }
    }
    
    /// Draw the editor screen
    pub fn draw(&self) {
        framebuffer::clear();
        
        // Draw file content
        let end_row = core::cmp::min(
            self.scroll_offset + self.max_rows,
            self.lines.len()
        );
        
        for (_screen_row, file_row) in (self.scroll_offset..end_row).enumerate() {
            let line = &self.lines[file_row];
            framebuffer::print(line);
            framebuffer::print_char('\n');
        }
        
        // Draw status bar
        self.draw_status_bar();
        
        // Draw help bar
        self.draw_help_bar();
        
        // Draw cursor at editing position
        if self.cursor_row >= self.scroll_offset && self.cursor_row < end_row {
            let screen_row = self.cursor_row - self.scroll_offset;
            framebuffer::draw_cursor_at(screen_row, self.cursor_col, true);
        }
    }
    
    /// Draw status bar
    fn draw_status_bar(&self) {
        framebuffer::print("\n-- ");
        framebuffer::print(&self.filename);
        if self.modified {
            framebuffer::print(" [Modified]");
        }
        framebuffer::print(" -- ");
        
        // Row/Col position
        framebuffer::print("Ln ");
        print_num(self.cursor_row + 1);
        framebuffer::print(", Col ");
        print_num(self.cursor_col + 1);
        
        // Message if any
        if let Some(ref msg) = self.message {
            framebuffer::print(" | ");
            framebuffer::print(msg);
        }
    }
    
    /// Draw help bar
    fn draw_help_bar(&self) {
        framebuffer::print_char('\n');
        framebuffer::print("^G Help  ^X Save  ^C Exit  ^W Search  ^K Cut  ^U Paste");
    }
    
    /// Handle key input
    pub fn handle_key(&mut self, key: crate::drivers::keyboard::EditorKey) -> bool {
        use crate::drivers::keyboard::EditorKey;
        
        match key {
            EditorKey::Char(c) => return self.handle_char_input(c),
            EditorKey::ArrowUp => self.move_up(),
            EditorKey::ArrowDown => self.move_down(),
            EditorKey::ArrowLeft => self.move_left(),
            EditorKey::ArrowRight => self.move_right(),
            EditorKey::PageUp => {
                for _ in 0..10 {
                    self.move_up();
                }
            }
            EditorKey::PageDown => {
                for _ in 0..10 {
                    self.move_down();
                }
            }
            EditorKey::Home => {
                self.cursor_col = 0;
            }
            EditorKey::End => {
                self.cursor_col = self.lines[self.cursor_row].len();
            }
            EditorKey::Delete => {
                // Delete character at cursor
                if self.cursor_col < self.lines[self.cursor_row].len() {
                    self.lines[self.cursor_row].remove(self.cursor_col);
                    self.modified = true;
                } else if self.cursor_row + 1 < self.lines.len() {
                    // Join with next line
                    let next_line = self.lines.remove(self.cursor_row + 1);
                    self.lines[self.cursor_row].push_str(&next_line);
                    self.modified = true;
                }
            }
        }
        
        false // Don't exit
    }
    
    /// Handle character input (separate from key navigation)
    fn handle_char_input(&mut self, c: char) -> bool {
            match c {
                // Ctrl+G = Get Help
                '\x07' => {
                    self.show_help();
                }
                // Ctrl+X = Save (Write Out)
                '\x18' => {
                    self.save_file();
                }
                // Ctrl+C = Exit (always exit, show warning if modified)
                '\x03' => {
                    if self.modified {
                        self.message = Some("Warning: Unsaved changes! (^X to save)".to_string());
                    }
                    return true; // Always exit
                }
                // Ctrl+W = Where Is (Search)
                '\x17' => {
                    self.message = Some("Search: Not implemented yet".to_string());
                }
                // Ctrl+K = Cut line
                '\x0B' => {
                    self.message = Some("Cut: Not implemented yet".to_string());
                }
                // Ctrl+U = Uncut (Paste)
            '\x15' => {
                self.message = Some("Paste: Not implemented yet".to_string());
            }
            // Backspace
            '\x08' => {
                self.handle_backspace();
            }
            // Enter
            '\n' | '\r' => {
                self.handle_enter();
            }
            // Printable character
            _ if c >= ' ' => {
                self.handle_char(c);
            }
            _ => {}
        }
        
        false // Don't exit by default
    }
    
    /// Show help screen
    fn show_help(&mut self) {
        self.message = Some("^G=Help ^X=Save ^C=Exit ^W=Search ^K=Cut ^U=Paste".to_string());
    }
    
    /// Save file
    fn save_file(&mut self) {
        // Collect all lines into single string
        let mut content = String::new();
        for (i, line) in self.lines.iter().enumerate() {
            content.push_str(line);
            if i < self.lines.len() - 1 {
                content.push('\n');
            }
        }
        
        let response = vfs::process_request(FSRequest::WriteFile {
            path: self.filename.clone(),
            data: content.as_bytes().to_vec(),
        });
        
        match response {
            FSResponse::Success => {
                self.modified = false;
                self.message = Some("Saved!".to_string());
            }
            FSResponse::Error(msg) => {
                self.message = Some(format!("Save failed: {}", msg));
            }
            _ => {}
        }
    }
    
    /// Handle backspace
    fn handle_backspace(&mut self) {
        if self.cursor_col > 0 {
            let line = &mut self.lines[self.cursor_row];
            if self.cursor_col <= line.len() {
                line.remove(self.cursor_col - 1);
                self.cursor_col -= 1;
                self.modified = true;
            }
        } else if self.cursor_row > 0 {
            // Join with previous line
            let current_line = self.lines.remove(self.cursor_row);
            self.cursor_row -= 1;
            self.cursor_col = self.lines[self.cursor_row].len();
            self.lines[self.cursor_row].push_str(&current_line);
            self.modified = true;
        }
    }
    
    /// Handle enter (new line)
    fn handle_enter(&mut self) {
        let line = &mut self.lines[self.cursor_row];
        let remainder = line.split_off(self.cursor_col);
        self.cursor_row += 1;
        self.cursor_col = 0;
        self.lines.insert(self.cursor_row, remainder);
        self.modified = true;
        
        // Auto-scroll if needed
        if self.cursor_row >= self.scroll_offset + self.max_rows {
            self.scroll_offset += 1;
        }
    }
    
    /// Handle regular character input
    fn handle_char(&mut self, c: char) {
        let line = &mut self.lines[self.cursor_row];
        line.insert(self.cursor_col, c);
        self.cursor_col += 1;
        self.modified = true;
    }
    
    /// Move cursor up
    pub fn move_up(&mut self) {
        if self.cursor_row > 0 {
            self.cursor_row -= 1;
            if self.cursor_row < self.scroll_offset {
                self.scroll_offset = self.cursor_row;
            }
            // Adjust column if line is shorter
            let line_len = self.lines[self.cursor_row].len();
            if self.cursor_col > line_len {
                self.cursor_col = line_len;
            }
        }
    }
    
    /// Move cursor down
    pub fn move_down(&mut self) {
        if self.cursor_row + 1 < self.lines.len() {
            self.cursor_row += 1;
            if self.cursor_row >= self.scroll_offset + self.max_rows {
                self.scroll_offset += 1;
            }
            // Adjust column if line is shorter
            let line_len = self.lines[self.cursor_row].len();
            if self.cursor_col > line_len {
                self.cursor_col = line_len;
            }
        }
    }
    
    /// Move cursor left
    pub fn move_left(&mut self) {
        if self.cursor_col > 0 {
            self.cursor_col -= 1;
        } else if self.cursor_row > 0 {
            // Jump to end of previous line
            self.cursor_row -= 1;
            self.cursor_col = self.lines[self.cursor_row].len();
        }
    }
    
    /// Move cursor right
    pub fn move_right(&mut self) {
        let line_len = self.lines[self.cursor_row].len();
        if self.cursor_col < line_len {
            self.cursor_col += 1;
        } else if self.cursor_row + 1 < self.lines.len() {
            // Jump to start of next line
            self.cursor_row += 1;
            self.cursor_col = 0;
        }
    }
}

/// Helper to print numbers
fn print_num(n: usize) {
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

/// Open file in grape editor
pub fn open(filename: &str) -> Result<(), String> {
    let mut editor = GrapeEditor::new(filename, 20); // ~20 lines visible
    
    // Try to load file
    match editor.load_file() {
        Ok(_) => editor.message = Some("File loaded".to_string()),
        Err(_) => editor.message = Some("New file".to_string()),
    }
    
    // Main editor loop - handle keyboard input
    loop {
        editor.draw();
        
        // Wait for keyboard input
        if let Some(key) = crate::drivers::keyboard::read_editor_key_blocking() {
            let should_exit = editor.handle_key(key);
            if should_exit {
                break;  // Exit requested
            }
        }
    }
    
    Ok(())
}
