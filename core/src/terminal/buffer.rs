//! Terminal buffer for storing character cells and scrollback

use std::collections::VecDeque;

/// Terminal cell (character with attributes)
#[derive(Debug, Clone, PartialEq)]
pub struct Cell {
    pub character: char,
    pub foreground: Color,
    pub background: Color,
    pub bold: bool,
    pub dim: bool,
    pub italic: bool,
    pub underline: bool,
    pub blink: bool,
    pub reverse: bool,
    pub strikethrough: bool,
    pub width: u8,
    pub is_wide_char_part: bool,
}

impl Default for Cell {
    fn default() -> Self {
        Self {
            character: ' ',
            foreground: Color::Default,
            background: Color::Default,
            bold: false,
            dim: false,
            italic: false,
            underline: false,
            blink: false,
            reverse: false,
            strikethrough: false,
            width: 1,
            is_wide_char_part: false,
        }
    }
}

/// Terminal color
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Color {
    Default,
    Black,
    Red,
    Green,
    Yellow,
    Blue,
    Magenta,
    Cyan,
    White,
    BrightBlack,
    BrightRed,
    BrightGreen,
    BrightYellow,
    BrightBlue,
    BrightMagenta,
    BrightCyan,
    BrightWhite,
    Indexed(u8),
    Rgb(u8, u8, u8),
}

/// Terminal buffer with scrollback
#[derive(Debug)]
pub struct TerminalBuffer {
    /// Screen buffer (current visible area)
    screen: Vec<Vec<Cell>>,
    /// Scrollback buffer (history)
    scrollback: VecDeque<Vec<Cell>>,
    /// Maximum scrollback lines
    max_scrollback: usize,
    /// Current cursor position
    cursor: (u16, u16),
    /// Saved cursor position
    saved_cursor: Option<(u16, u16)>,
    /// Current screen size
    size: (u16, u16),
    /// Scroll offset (for viewing scrollback)
    scroll_offset: usize,
    /// Alternate screen buffer
    alt_screen: Option<Vec<Vec<Cell>>>,
    /// Using alternate screen
    using_alt_screen: bool,
    /// Tab stops
    tab_stops: Vec<bool>,
    /// Current attribute state
    attrs: CellAttributes,
    /// Mode flags
    mode: TerminalMode,
}

/// Cell attributes (for building cells)
#[derive(Debug, Clone, Copy)]
pub struct CellAttributes {
    pub foreground: Color,
    pub background: Color,
    pub bold: bool,
    pub dim: bool,
    pub italic: bool,
    pub underline: bool,
    pub blink: bool,
    pub reverse: bool,
    pub strikethrough: bool,
}

impl Default for CellAttributes {
    fn default() -> Self {
        Self {
            foreground: Color::Default,
            background: Color::Default,
            bold: false,
            dim: false,
            italic: false,
            underline: false,
            blink: false,
            reverse: false,
            strikethrough: false,
        }
    }
}

/// Terminal mode flags
#[derive(Debug, Clone, Copy)]
pub struct TerminalMode {
    pub line_wrap: bool,
    pub insert_mode: bool,
    pub app_cursor: bool,
    pub app_keypad: bool,
    pub mouse_reporting: bool,
    pub bracketed_paste: bool,
    pub focus_events: bool,
    pub origin_mode: bool,
    pub columns_132: bool,
    pub smooth_scroll: bool,
    pub reverse_video: bool,
    pub autowrap: bool,
}

impl Default for TerminalMode {
    fn default() -> Self {
        Self {
            line_wrap: true,
            insert_mode: false,
            app_cursor: false,
            app_keypad: false,
            mouse_reporting: false,
            bracketed_paste: false,
            focus_events: false,
            origin_mode: false,
            columns_132: false,
            smooth_scroll: false,
            reverse_video: false,
            autowrap: true,
        }
    }
}

impl TerminalBuffer {
    /// Create new terminal buffer
    pub fn new(max_scrollback: usize) -> Self {
        let mut buffer = Self {
            screen: Vec::new(),
            scrollback: VecDeque::with_capacity(max_scrollback),
            max_scrollback,
            cursor: (0, 0),
            saved_cursor: None,
            size: (24, 80),
            scroll_offset: 0,
            alt_screen: None,
            using_alt_screen: false,
            tab_stops: Vec::new(),
            attrs: CellAttributes::default(),
            mode: TerminalMode::default(),
        };
        
        buffer.resize(24, 80);
        buffer.init_tab_stops();
        
        buffer
    }

    /// Resize buffer
    pub fn resize(&mut self, rows: u16, cols: u16) {
        let old_screen = std::mem::take(&mut self.screen);
        self.screen = vec![vec![Cell::default(); cols as usize]; rows as usize];
        
        // Copy old content where possible
        for (r, row) in old_screen.into_iter().enumerate() {
            if r < rows as usize {
                for (c, cell) in row.into_iter().enumerate() {
                    if c < cols as usize {
                        self.screen[r][c] = cell;
                    }
                }
            }
        }
        
        self.size = (rows, cols);
        self.cursor.0 = self.cursor.0.min(rows - 1);
        self.cursor.1 = self.cursor.1.min(cols - 1);
        
        self.tab_stops.resize(cols as usize, false);
        self.init_tab_stops();
    }

    /// Initialize default tab stops (every 8 columns)
    fn init_tab_stops(&mut self) {
        for i in (0..self.tab_stops.len()).step_by(8) {
            if i < self.tab_stops.len() {
                self.tab_stops[i] = true;
            }
        }
    }

    /// Write data to buffer (process escape sequences)
    pub fn write(&mut self, data: &[u8]) {
        let mut i = 0;
        while i < data.len() {
            match data[i] {
                0x1B => { // ESC
                    i += self.handle_escape(&data[i..]);
                }
                0x07 => self.bell(), // BEL
                0x08 => self.backspace(), // BS
                0x09 => self.tab(), // TAB
                0x0A | 0x0B | 0x0C => self.newline(), // LF, VT, FF
                0x0D => self.carriage_return(), // CR
                0x18 | 0x1A => self.cancel(), // CAN, SUB
                0x7F => self.delete(), // DEL
                0x80..=0x9F => self.handle_c1(data[i]), // C1 control codes
                _ => self.print_char(data[i] as char), // Printable characters
            }
            i += 1;
        }
    }

    /// Print a character
    fn print_char(&mut self, c: char) {
        let (_row, col) = self.cursor;
        
        // Check if we need to wrap
        if col >= self.size.1 && self.mode.autowrap {
            self.newline();
        }
        
        let (row, col) = self.cursor;
        
        if row < self.size.0 && col < self.size.1 {
            self.screen[row as usize][col as usize] = Cell {
                character: c,
                foreground: self.attrs.foreground,
                background: self.attrs.background,
                bold: self.attrs.bold,
                dim: self.attrs.dim,
                italic: self.attrs.italic,
                underline: self.attrs.underline,
                blink: self.attrs.blink,
                reverse: self.attrs.reverse,
                strikethrough: self.attrs.strikethrough,
                width: 1,
                is_wide_char_part: false,
            };
            
            self.cursor.1 = col + 1;
        }
    }

    /// Newline (LF)
    fn newline(&mut self) {
        let (row, _) = self.cursor;
        
        if row + 1 >= self.size.0 {
            // Scroll up
            self.scroll_up(1);
        } else {
            self.cursor.0 = row + 1;
        }
    }

    /// Carriage return
    fn carriage_return(&mut self) {
        self.cursor.1 = 0;
    }

    /// Backspace
    fn backspace(&mut self) {
        if self.cursor.1 > 0 {
            self.cursor.1 -= 1;
        }
    }

    /// Tab
    fn tab(&mut self) {
        let col = self.cursor.1 as usize;
        
        // Find next tab stop
        for i in col + 1..self.tab_stops.len() {
            if self.tab_stops[i] {
                self.cursor.1 = i as u16;
                return;
            }
        }
        
        // Default to next 8-column boundary
        self.cursor.1 = ((col / 8 + 1) * 8) as u16;
    }

    /// Delete character
    fn delete(&mut self) {
        // DEL is usually ignored
    }

    /// Bell
    fn bell(&mut self) {
        // Will be handled by terminal
    }

    /// Cancel
    fn cancel(&mut self) {
        // Cancel escape sequence
    }

    /// Handle C1 control codes
    fn handle_c1(&mut self, byte: u8) {
        match byte {
            0x84 => self.newline(), // IND
            0x85 => self.newline(), // NEL
            0x88 => self.set_tab(), // HTS
            0x8D => self.carriage_return(), // RI (reverse index)
            _ => {}
        }
    }

    /// Handle escape sequence
    fn handle_escape(&mut self, data: &[u8]) -> usize {
        if data.len() < 2 {
            return data.len();
        }
        
        match data[1] {
            b'[' => self.handle_csi(&data[2..]) + 2,
            b']' => self.handle_osc(&data[2..]) + 2,
            b'(' => self.handle_charset(&data[2..]) + 2,
            b')' => self.handle_charset(&data[2..]) + 2,
            b'#' => self.handle_control(&data[2..]) + 2,
            b'=' => { self.mode.app_keypad = true; 2 }
            b'>' => { self.mode.app_keypad = false; 2 }
            b'7' => { self.save_cursor(); 2 }
            b'8' => { self.restore_cursor(); 2 }
            b'c' => { self.reset(); 2 }
            _ => 2,
        }
    }

    /// Handle CSI (Control Sequence Introducer)
    fn handle_csi(&mut self, data: &[u8]) -> usize {
        let mut i = 0;
        let mut params = vec![0];
        let mut current_param = 0;
        
        // Parse parameters
        while i < data.len() {
            match data[i] {
                b'0'..=b'9' => {
                    current_param = current_param * 10 + (data[i] - b'0') as i32;
                }
                b';' => {
                    params.push(current_param);
                    current_param = 0;
                }
                _ => break,
            }
            i += 1;
        }
        
        if params.len() == 1 {
            params[0] = current_param;
        } else {
            params.push(current_param);
        }
        
        if i < data.len() {
            match data[i] {
                b'A' => self.csi_cuu(&params), // Cursor Up
                b'B' => self.csi_cud(&params), // Cursor Down
                b'C' => self.csi_cuf(&params), // Cursor Forward
                b'D' => self.csi_cub(&params), // Cursor Back
                b'E' => self.csi_cnl(&params), // Cursor Next Line
                b'F' => self.csi_cpl(&params), // Cursor Previous Line
                b'G' => self.csi_cha(&params), // Cursor Horizontal Absolute
                b'H' => self.csi_cup(&params), // Cursor Position
                b'J' => self.csi_ed(&params), // Erase in Display
                b'K' => self.csi_el(&params), // Erase in Line
                b'L' => self.csi_il(&params), // Insert Lines
                b'M' => self.csi_dl(&params), // Delete Lines
                b'P' => self.csi_dch(&params), // Delete Characters
                b'@' => self.csi_ich(&params), // Insert Characters
                b'S' => self.csi_su(&params), // Scroll Up
                b'T' => self.csi_sd(&params), // Scroll Down
                b'm' => self.csi_sgr(&params), // Select Graphic Rendition
                b'h' => self.csi_sm(&params), // Set Mode
                b'l' => self.csi_rm(&params), // Reset Mode
                b'r' => self.csi_decstbm(&params), // Set Top/Bottom Margins
                b's' => self.save_cursor(), // Save Cursor
                b'u' => self.restore_cursor(), // Restore Cursor
                _ => {}
            }
            i += 1;
        }
        
        i
    }

    /// Cursor Up
    fn csi_cuu(&mut self, params: &[i32]) {
        let count = params.get(0).copied().unwrap_or(1).max(1) as u16;
        self.cursor.0 = self.cursor.0.saturating_sub(count);
    }

    /// Cursor Down
    fn csi_cud(&mut self, params: &[i32]) {
        let count = params.get(0).copied().unwrap_or(1).max(1) as u16;
        self.cursor.0 = (self.cursor.0 + count).min(self.size.0 - 1);
    }

    /// Cursor Forward
    fn csi_cuf(&mut self, params: &[i32]) {
        let count = params.get(0).copied().unwrap_or(1).max(1) as u16;
        self.cursor.1 = (self.cursor.1 + count).min(self.size.1 - 1);
    }

    /// Cursor Back
    fn csi_cub(&mut self, params: &[i32]) {
        let count = params.get(0).copied().unwrap_or(1).max(1) as u16;
        self.cursor.1 = self.cursor.1.saturating_sub(count);
    }

    /// Cursor Next Line
    fn csi_cnl(&mut self, params: &[i32]) {
        let count = params.get(0).copied().unwrap_or(1).max(1) as u16;
        self.cursor.0 = (self.cursor.0 + count).min(self.size.0 - 1);
        self.cursor.1 = 0;
    }

    /// Cursor Previous Line
    fn csi_cpl(&mut self, params: &[i32]) {
        let count = params.get(0).copied().unwrap_or(1).max(1) as u16;
        self.cursor.0 = self.cursor.0.saturating_sub(count);
        self.cursor.1 = 0;
    }

    /// Cursor Horizontal Absolute
    fn csi_cha(&mut self, params: &[i32]) {
        let col = params.get(0).copied().unwrap_or(1).max(1) as u16 - 1;
        self.cursor.1 = col.min(self.size.1 - 1);
    }

    /// Cursor Position
    fn csi_cup(&mut self, params: &[i32]) {
        let row = params.get(0).copied().unwrap_or(1).max(1) as u16 - 1;
        let col = params.get(1).copied().unwrap_or(1).max(1) as u16 - 1;
        self.cursor.0 = row.min(self.size.0 - 1);
        self.cursor.1 = col.min(self.size.1 - 1);
    }

    /// Erase in Display
    fn csi_ed(&mut self, params: &[i32]) {
        match params.get(0).copied().unwrap_or(0) {
            0 => self.erase_below(),
            1 => self.erase_above(),
            2 | 3 => self.erase_all(),
            _ => {}
        }
    }

    /// Erase in Line
    fn csi_el(&mut self, params: &[i32]) {
        match params.get(0).copied().unwrap_or(0) {
            0 => self.erase_to_eol(),
            1 => self.erase_from_bol(),
            2 => self.erase_line(),
            _ => {}
        }
    }

    /// Insert Lines
    fn csi_il(&mut self, params: &[i32]) {
        let count = params.get(0).copied().unwrap_or(1).max(1) as usize;
        let row = self.cursor.0 as usize;
        
        for _ in 0..count {
            if row < self.screen.len() {
                self.screen.insert(row, vec![Cell::default(); self.size.1 as usize]);
                self.screen.pop();
            }
        }
    }

    /// Delete Lines
    fn csi_dl(&mut self, params: &[i32]) {
        let count = params.get(0).copied().unwrap_or(1).max(1) as usize;
        let row = self.cursor.0 as usize;
        
        for _ in 0..count {
            if row < self.screen.len() {
                self.screen.remove(row);
                self.screen.push(vec![Cell::default(); self.size.1 as usize]);
            }
        }
    }

    /// Delete Characters
    fn csi_dch(&mut self, params: &[i32]) {
        let count = params.get(0).copied().unwrap_or(1).max(1) as usize;
        let (row, col) = (self.cursor.0 as usize, self.cursor.1 as usize);
        
        if row < self.screen.len() && col < self.screen[row].len() {
            for i in col..self.screen[row].len() - count {
                self.screen[row][i] = self.screen[row][i + count].clone();
            }
            for i in self.screen[row].len() - count..self.screen[row].len() {
                self.screen[row][i] = Cell::default();
            }
        }
    }

    /// Insert Characters
    fn csi_ich(&mut self, params: &[i32]) {
        let count = params.get(0).copied().unwrap_or(1).max(1) as usize;
        let (row, col) = (self.cursor.0 as usize, self.cursor.1 as usize);
        
        if row < self.screen.len() && col < self.screen[row].len() {
            for i in (col..self.screen[row].len() - count).rev() {
                self.screen[row][i + count] = self.screen[row][i].clone();
            }
            for i in col..col + count {
                if i < self.screen[row].len() {
                    self.screen[row][i] = Cell::default();
                }
            }
        }
    }

    /// Scroll Up
    fn csi_su(&mut self, params: &[i32]) {
        let count = params.get(0).copied().unwrap_or(1).max(1) as usize;
        self.scroll_up(count);
    }

    /// Scroll Down
    fn csi_sd(&mut self, params: &[i32]) {
        let count = params.get(0).copied().unwrap_or(1).max(1) as usize;
        self.scroll_down(count);
    }

    /// Select Graphic Rendition
    fn csi_sgr(&mut self, params: &[i32]) {
        if params.is_empty() || params[0] == 0 {
            // Reset
            self.attrs = CellAttributes::default();
            return;
        }
        
        let mut i = 0;
        while i < params.len() {
            match params[i] {
                1 => self.attrs.bold = true,
                2 => self.attrs.dim = true,
                3 => self.attrs.italic = true,
                4 => self.attrs.underline = true,
                5 | 6 => self.attrs.blink = true,
                7 => self.attrs.reverse = true,
                8 => self.attrs.strikethrough = true,
                22 => {
                    self.attrs.bold = false;
                    self.attrs.dim = false;
                }
                23 => self.attrs.italic = false,
                24 => self.attrs.underline = false,
                25 => self.attrs.blink = false,
                27 => self.attrs.reverse = false,
                29 => self.attrs.strikethrough = false,
                30..=37 => self.attrs.foreground = Color::Indexed((params[i] - 30) as u8),
                38 => {
                    if i + 2 < params.len() && params[i + 1] == 5 {
                        self.attrs.foreground = Color::Indexed(params[i + 2] as u8);
                        i += 2;
                    } else if i + 4 < params.len() && params[i + 1] == 2 {
                        self.attrs.foreground = Color::Rgb(
                            params[i + 2] as u8,
                            params[i + 3] as u8,
                            params[i + 4] as u8,
                        );
                        i += 4;
                    }
                }
                39 => self.attrs.foreground = Color::Default,
                40..=47 => self.attrs.background = Color::Indexed((params[i] - 40) as u8),
                48 => {
                    if i + 2 < params.len() && params[i + 1] == 5 {
                        self.attrs.background = Color::Indexed(params[i + 2] as u8);
                        i += 2;
                    } else if i + 4 < params.len() && params[i + 1] == 2 {
                        self.attrs.background = Color::Rgb(
                            params[i + 2] as u8,
                            params[i + 3] as u8,
                            params[i + 4] as u8,
                        );
                        i += 4;
                    }
                }
                49 => self.attrs.background = Color::Default,
                90..=97 => self.attrs.foreground = Color::Indexed((params[i] - 90 + 8) as u8),
                100..=107 => self.attrs.background = Color::Indexed((params[i] - 100 + 8) as u8),
                _ => {}
            }
            i += 1;
        }
    }

    /// Set Mode
    fn csi_sm(&mut self, params: &[i32]) {
        for &p in params {
            match p {
                1 => self.mode.app_cursor = true,
                2 => self.mode.app_keypad = true,
                3 => self.mode.columns_132 = true,
                4 => self.mode.smooth_scroll = true,
                5 => self.mode.reverse_video = true,
                6 => self.mode.origin_mode = true,
                7 => self.mode.autowrap = true,
                12 => self.attrs.blink = true,
                25 => {} // Show cursor
                47 => self.use_alt_screen(true),
                1049 => {
                    self.use_alt_screen(true);
                    self.save_cursor();
                }
                _ => {}
            }
        }
    }

    /// Reset Mode
    fn csi_rm(&mut self, params: &[i32]) {
        for &p in params {
            match p {
                1 => self.mode.app_cursor = false,
                2 => self.mode.app_keypad = false,
                3 => self.mode.columns_132 = false,
                4 => self.mode.smooth_scroll = false,
                5 => self.mode.reverse_video = false,
                6 => self.mode.origin_mode = false,
                7 => self.mode.autowrap = false,
                12 => self.attrs.blink = false,
                25 => {} // Hide cursor
                47 => self.use_alt_screen(false),
                1049 => {
                    self.use_alt_screen(false);
                    self.restore_cursor();
                }
                _ => {}
            }
        }
    }

    /// Set Top/Bottom Margins
    fn csi_decstbm(&mut self, _params: &[i32]) {
        // Not implemented
    }

    /// Handle OSC (Operating System Command)
    fn handle_osc(&mut self, data: &[u8]) -> usize {
        let mut i = 0;
        let mut command = 0;
        
        while i < data.len() && data[i].is_ascii_digit() {
            command = command * 10 + (data[i] - b'0') as i32;
            i += 1;
        }
        
        if i < data.len() && data[i] == b';' {
            i += 1;
            // Parse parameters
            while i < data.len() && data[i] != 0x07 && !(data[i] == 0x1B && i + 1 < data.len() && data[i + 1] == b'\\') {
                i += 1;
            }
        }
        
        match command {
            0 | 1 | 2 => {} // Set window title
            8 => {} // Hyperlink
            _ => {}
        }
        
        if i < data.len() {
            if data[i] == 0x07 {
                i += 1;
            } else if i + 1 < data.len() && data[i] == 0x1B && data[i + 1] == b'\\' {
                i += 2;
            }
        }
        
        i
    }

    /// Handle charset selection
    fn handle_charset(&mut self, _data: &[u8]) -> usize {
        // Simplified - ignore charset selection
        1
    }

    /// Handle control functions
    fn handle_control(&mut self, data: &[u8]) -> usize {
        if !data.is_empty() && data[0] == b'8' {
            self.decaln(); // DEC screen alignment test
        }
        1
    }

    /// DEC screen alignment test
    fn decaln(&mut self) {
        for row in &mut self.screen {
            for cell in row {
                cell.character = 'E';
                cell.foreground = Color::Default;
                cell.background = Color::Default;
            }
        }
    }

    /// Save cursor
    fn save_cursor(&mut self) {
        self.saved_cursor = Some(self.cursor);
    }

    /// Restore cursor
    fn restore_cursor(&mut self) {
        if let Some(cursor) = self.saved_cursor {
            self.cursor = cursor;
        }
    }

    /// Set tab stop
    fn set_tab(&mut self) {
        let col = self.cursor.1 as usize;
        if col < self.tab_stops.len() {
            self.tab_stops[col] = true;
        }
    }

    /// Use alternate screen
    fn use_alt_screen(&mut self, use_alt: bool) {
        if use_alt && !self.using_alt_screen {
            self.alt_screen = Some(std::mem::take(&mut self.screen));
            self.screen = vec![vec![Cell::default(); self.size.1 as usize]; self.size.0 as usize];
            self.using_alt_screen = true;
            self.cursor = (0, 0);
        } else if !use_alt && self.using_alt_screen {
            if let Some(alt_screen) = self.alt_screen.take() {
                self.screen = alt_screen;
            }
            self.using_alt_screen = false;
        }
    }

    /// Scroll up by count lines
    fn scroll_up(&mut self, count: usize) {
        for _ in 0..count {
            if self.scrollback.len() >= self.max_scrollback {
                self.scrollback.pop_front();
            }
            
            if !self.screen.is_empty() {
                let top_line = self.screen.remove(0);
                self.scrollback.push_back(top_line);
                self.screen.push(vec![Cell::default(); self.size.1 as usize]);
            }
        }
    }

    /// Scroll down by count lines
    fn scroll_down(&mut self, count: usize) {
        for _ in 0..count {
            if self.screen.len() > 1 {
                self.screen.pop();
                self.screen.insert(0, vec![Cell::default(); self.size.1 as usize]);
            }
        }
    }

    /// Erase below cursor
    fn erase_below(&mut self) {
        let (row, _col) = self.cursor;
        
        // Erase from cursor to end of line
        self.erase_to_eol();
        
        // Clear all lines below
        for r in (row as usize + 1)..self.screen.len() {
            for c in 0..self.screen[r].len() {
                self.screen[r][c] = Cell::default();
            }
        }
    }

    /// Erase above cursor
    fn erase_above(&mut self) {
        let (row, col) = self.cursor;
        
        // Clear all lines above
        for r in 0..row as usize {
            for c in 0..self.screen[r].len() {
                self.screen[r][c] = Cell::default();
            }
        }
        
        // Erase from start to cursor on current line
        for c in 0..=col as usize {
            if c < self.screen[row as usize].len() {
                self.screen[row as usize][c] = Cell::default();
            }
        }
    }

    /// Erase entire screen
    fn erase_all(&mut self) {
        for row in &mut self.screen {
            for cell in row {
                *cell = Cell::default();
            }
        }
    }

    /// Erase to end of line
    fn erase_to_eol(&mut self) {
        let (row, col) = self.cursor;
        
        for c in col as usize..self.screen[row as usize].len() {
            self.screen[row as usize][c] = Cell::default();
        }
    }

    /// Erase from beginning of line
    fn erase_from_bol(&mut self) {
        let (row, col) = self.cursor;
        
        for c in 0..=col as usize {
            if c < self.screen[row as usize].len() {
                self.screen[row as usize][c] = Cell::default();
            }
        }
    }

    /// Erase entire line
    fn erase_line(&mut self) {
        let (row, _) = self.cursor;
        
        for cell in &mut self.screen[row as usize] {
            *cell = Cell::default();
        }
    }

    /// Reset terminal
    fn reset(&mut self) {
        self.screen = vec![vec![Cell::default(); self.size.1 as usize]; self.size.0 as usize];
        self.scrollback.clear();
        self.cursor = (0, 0);
        self.saved_cursor = None;
        self.attrs = CellAttributes::default();
        self.mode = TerminalMode::default();
        self.using_alt_screen = false;
        self.alt_screen = None;
        self.init_tab_stops();
    }

    /// Clear screen
    pub fn clear(&mut self) {
        self.erase_all();
    }

    /// Clear scrollback
    pub fn clear_scrollback(&mut self) {
        self.scrollback.clear();
    }

    /// Get visible content (with scroll offset)
    pub fn visible_content(&self) -> Vec<Vec<Cell>> {
        let start = self.scrollback.len().saturating_sub(self.scroll_offset);
        let end = (start + self.size.0 as usize).min(self.scrollback.len() + self.screen.len());
        
        let mut visible = Vec::new();
        
        for i in start..end {
            if i < self.scrollback.len() {
                visible.push(self.scrollback[i].clone());
            } else {
                let screen_idx = i - self.scrollback.len();
                if screen_idx < self.screen.len() {
                    visible.push(self.screen[screen_idx].clone());
                }
            }
        }
        
        // Pad if needed
        while visible.len() < self.size.0 as usize {
            visible.push(vec![Cell::default(); self.size.1 as usize]);
        }
        
        visible
    }

    /// Get cursor position
    pub fn cursor(&self) -> (u16, u16) {
        self.cursor
    }

    /// Set scroll offset
    pub fn set_scroll_offset(&mut self, offset: usize) {
        self.scroll_offset = offset.min(self.scrollback.len());
    }

    /// Get scroll offset
    pub fn scroll_offset(&self) -> usize {
        self.scroll_offset
    }

    /// Get screen size
    pub fn size(&self) -> (u16, u16) {
        self.size
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_buffer_creation() {
        let buffer = TerminalBuffer::new(1000);
        assert_eq!(buffer.size(), (24, 80));
        assert_eq!(buffer.visible_content().len(), 24);
    }

    #[test]
    fn test_print_char() {
        let mut buffer = TerminalBuffer::new(1000);
        buffer.write(b"Hello");
        
        let visible = buffer.visible_content();
        assert_eq!(visible[0][0].character, 'H');
        assert_eq!(visible[0][1].character, 'e');
        assert_eq!(visible[0][2].character, 'l');
        assert_eq!(visible[0][3].character, 'l');
        assert_eq!(visible[0][4].character, 'o');
    }

    #[test]
    fn test_cursor_movement() {
        let mut buffer = TerminalBuffer::new(1000);
        buffer.write(b"\x1b[5;10H"); // Move to row 5, col 10
        
        assert_eq!(buffer.cursor(), (4, 9)); // 0-based
    }
}