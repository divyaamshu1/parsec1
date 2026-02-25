//! Cursor movement operations with word boundaries and bracket matching

use super::{Position, Buffer};

/// Defines cursor movement operations
pub trait Movement {
    /// Move cursor up by n lines
    fn move_up(&self, pos: Position, lines: usize) -> Position;
    
    /// Move cursor down by n lines
    fn move_down(&self, pos: Position, lines: usize) -> Position;
    
    /// Move cursor left by n characters
    fn move_left(&self, pos: Position, chars: usize) -> Position;
    
    /// Move cursor right by n characters
    fn move_right(&self, pos: Position, chars: usize) -> Position;
    
    /// Move to start of line (with smart home: first non-whitespace on second press)
    fn move_to_line_start(&self, pos: Position, smart: bool) -> Position;
    
    /// Move to end of line
    fn move_to_line_end(&self, pos: Position) -> Position;
    
    /// Move to start of document
    fn move_to_document_start(&self) -> Position;
    
    /// Move to end of document
    fn move_to_document_end(&self) -> Position;
    
    /// Move to next word boundary
    fn move_to_next_word(&self, pos: Position) -> Position;
    
    /// Move to previous word boundary
    fn move_to_prev_word(&self, pos: Position) -> Position;
    
    /// Move to next word start
    fn move_to_next_word_start(&self, pos: Position) -> Position;
    
    /// Move to next word end
    fn move_to_next_word_end(&self, pos: Position) -> Position;
    
    /// Move to previous word start
    fn move_to_prev_word_start(&self, pos: Position) -> Position;
    
    /// Move to matching bracket
    fn move_to_matching_bracket(&self, pos: Position) -> Option<Position>;
    
    /// Move to next paragraph
    fn move_to_next_paragraph(&self, pos: Position) -> Position;
    
    /// Move to previous paragraph
    fn move_to_prev_paragraph(&self, pos: Position) -> Position;
}

/// Movement implementation for a buffer
pub struct BufferMovement<'a> {
    buffer: &'a Buffer,
}

impl<'a> BufferMovement<'a> {
    pub fn new(buffer: &'a Buffer) -> Self {
        Self { buffer }
    }

    /// Get line length
    fn line_length(&self, line: usize) -> usize {
        if line < self.buffer.line_count() {
            self.buffer.line_length(line)
        } else {
            0
        }
    }

    /// Find matching bracket
    fn find_matching_bracket(&self, pos: Position) -> Option<Position> {
        let chars: Vec<char> = self.buffer.text().chars().collect();
        let idx = self.buffer.position_to_index(pos);
        
        if idx >= chars.len() {
            return None;
        }
        
        let c = chars[idx];
        let (open, close) = match c {
            '(' => ('(', ')'),
            ')' => (')', '('),
            '[' => ('[', ']'),
            ']' => (']', '['),
            '{' => ('{', '}'),
            '}' => ('}', '{'),
            '<' => ('<', '>'),
            '>' => ('>', '<'),
            _ => return None,
        };
        
        let mut depth = 1;
        let step: i32 = if c == open { 1 } else { -1 };
        let mut i = idx as i32 + step;
        
        while i >= 0 && i < chars.len() as i32 {
            let current = chars[i as usize];
            if current == open {
                depth += 1;
            } else if current == close {
                depth -= 1;
                if depth == 0 {
                    return Some(self.buffer.index_to_position(i as usize));
                }
            }
            i += step;
        }
        
        None
    }

    /// Find next word boundary
    fn next_word_boundary(&self, pos: Position, start_of_word: bool) -> Position {
        let chars: Vec<char> = self.buffer.text().chars().collect();
        let mut idx = self.buffer.position_to_index(pos);
        
        if idx >= chars.len() {
            return self.move_to_document_end();
        }
        
        // Define word characters
        let is_word_char = |c: &char| c.is_alphanumeric() || *c == '_';
        
        // Skip current word if we're in one
        if start_of_word {
            while idx < chars.len() && is_word_char(&chars[idx]) {
                idx += 1;
            }
        }
        
        // Skip whitespace
        while idx < chars.len() && chars[idx].is_whitespace() {
            idx += 1;
        }
        
        if idx >= chars.len() {
            return self.move_to_document_end();
        }
        
        self.buffer.index_to_position(idx)
    }

    /// Find previous word boundary
    fn prev_word_boundary(&self, pos: Position, start_of_word: bool) -> Position {
        let chars: Vec<char> = self.buffer.text().chars().collect();
        let mut idx = self.buffer.position_to_index(pos);
        
        if idx == 0 {
            return Position::default();
        }
        
        idx -= 1;
        
        // Skip whitespace
        while idx > 0 && chars[idx].is_whitespace() {
            idx -= 1;
        }
        
        // Skip to start of word
        if start_of_word {
            while idx > 0 && is_word_char(&chars[idx - 1]) {
                idx -= 1;
            }
        } else {
            // Skip to end of previous word
            while idx > 0 && is_word_char(&chars[idx]) {
                idx -= 1;
            }
            if idx > 0 || is_word_char(&chars[0]) {
                // Adjust to end of word
                while idx < chars.len() && is_word_char(&chars[idx]) {
                    idx += 1;
                }
                if idx > 0 {
                    idx -= 1;
                }
            }
        }
        
        self.buffer.index_to_position(idx)
    }

    /// Find next paragraph
    fn next_paragraph(&self, pos: Position) -> Position {
        let mut line = pos.line;
        let line_count = self.buffer.line_count();
        
        // Skip current empty lines
        while line < line_count && self.buffer.line(line).trim().is_empty() {
            line += 1;
        }
        
        // Find next empty line
        while line < line_count && !self.buffer.line(line).trim().is_empty() {
            line += 1;
        }
        
        if line >= line_count {
            return Position::new(line_count - 1, self.line_length(line_count - 1));
        }
        
        Position::new(line, 0)
    }

    /// Find previous paragraph
    fn prev_paragraph(&self, pos: Position) -> Position {
        let mut line = pos.line;
        
        // Move up one line if at start of line
        if line > 0 && pos.column == 0 {
            line -= 1;
        }
        
        // Skip current empty lines going up
        while line > 0 && self.buffer.line(line).trim().is_empty() {
            line -= 1;
        }
        
        // Find previous empty line
        while line > 0 && !self.buffer.line(line).trim().is_empty() {
            line -= 1;
        }
        
        Position::new(line, 0)
    }
}

impl<'a> Movement for BufferMovement<'a> {
    fn move_up(&self, pos: Position, lines: usize) -> Position {
        if pos.line < lines {
            return Position::start_of_line(0);
        }
        
        let new_line = pos.line - lines;
        let preferred_col = pos.column; // Use original column as preferred
        
        Position::new(
            new_line,
            preferred_col.min(self.line_length(new_line))
        )
    }

    fn move_down(&self, pos: Position, lines: usize) -> Position {
        let max_line = self.buffer.line_count().saturating_sub(1);
        let new_line = (pos.line + lines).min(max_line);
        let preferred_col = pos.column;
        
        Position::new(
            new_line,
            preferred_col.min(self.line_length(new_line))
        )
    }

    fn move_left(&self, pos: Position, chars: usize) -> Position {
        if pos.column >= chars {
            return Position::new(pos.line, pos.column - chars);
        }
        
        if pos.line == 0 {
            return Position::default();
        }
        
        // Move to previous line
        let prev_line_len = self.line_length(pos.line - 1);
        Position::new(pos.line - 1, prev_line_len)
    }

    fn move_right(&self, pos: Position, chars: usize) -> Position {
        let line_len = self.line_length(pos.line);
        
        if pos.column + chars < line_len {
            return Position::new(pos.line, pos.column + chars);
        }
        
        if pos.line + 1 >= self.buffer.line_count() {
            return Position::new(pos.line, line_len);
        }
        
        // Move to next line
        Position::new(pos.line + 1, 0)
    }

    fn move_to_line_start(&self, pos: Position, smart: bool) -> Position {
        if !smart {
            return Position::new(pos.line, 0);
        }
        
        // Find first non-whitespace character (smart home)
        let line = self.buffer.line(pos.line);
        let first_non_ws = line.chars()
            .position(|c| !c.is_whitespace())
            .unwrap_or(0);
        
        if pos.column > first_non_ws {
            Position::new(pos.line, first_non_ws)
        } else {
            Position::new(pos.line, 0)
        }
    }

    fn move_to_line_end(&self, pos: Position) -> Position {
        let line_len = self.line_length(pos.line);
        Position::new(pos.line, line_len)
    }

    fn move_to_document_start(&self) -> Position {
        Position::default()
    }

    fn move_to_document_end(&self) -> Position {
        let last_line = self.buffer.line_count().saturating_sub(1);
        let line_len = self.line_length(last_line);
        Position::new(last_line, line_len)
    }

    fn move_to_next_word(&self, pos: Position) -> Position {
        self.next_word_boundary(pos, true)
    }

    fn move_to_prev_word(&self, pos: Position) -> Position {
        self.prev_word_boundary(pos, true)
    }

    fn move_to_next_word_start(&self, pos: Position) -> Position {
        self.next_word_boundary(pos, true)
    }

    fn move_to_next_word_end(&self, pos: Position) -> Position {
        self.next_word_boundary(pos, false)
    }

    fn move_to_prev_word_start(&self, pos: Position) -> Position {
        self.prev_word_boundary(pos, true)
    }

    fn move_to_matching_bracket(&self, pos: Position) -> Option<Position> {
        self.find_matching_bracket(pos)
    }

    fn move_to_next_paragraph(&self, pos: Position) -> Position {
        self.next_paragraph(pos)
    }

    fn move_to_prev_paragraph(&self, pos: Position) -> Position {
        self.prev_paragraph(pos)
    }
}

/// Helper to check if a character is a word character
fn is_word_char(c: &char) -> bool {
    c.is_alphanumeric() || *c == '_'
}

#[cfg(test)]
mod tests {
    use super::*;
    
    fn create_test_buffer() -> Buffer {
        let mut buffer = Buffer::new(None);
        buffer.insert(0, "Hello world\nThis is a test\n\nLast paragraph\nMore text");
        buffer
    }

    #[test]
    fn test_move_up_down() {
        let buffer = create_test_buffer();
        let movement = BufferMovement::new(&buffer);
        
        let pos = Position::new(1, 5);
        let up = movement.move_up(pos, 1);
        assert_eq!(up.line, 0);
        assert_eq!(up.column, 5); // Should maintain column
        
        let down = movement.move_down(pos, 1);
        assert_eq!(down.line, 2);
        assert_eq!(down.column, 5.min(buffer.line_length(2)));
    }

    #[test]
    fn test_move_left_right() {
        let buffer = create_test_buffer();
        let movement = BufferMovement::new(&buffer);
        
        let pos = Position::new(0, 5);
        let left = movement.move_left(pos, 3);
        assert_eq!(left.column, 2);
        
        let right = movement.move_right(pos, 3);
        assert_eq!(right.column, 8);
        
        // Test line wrapping
        let pos_end = Position::new(0, 11); // "world" length
        let right_wrap = movement.move_right(pos_end, 1);
        assert_eq!(right_wrap.line, 1);
        assert_eq!(right_wrap.column, 0);
    }

    #[test]
    fn test_smart_home() {
        let mut buffer = Buffer::new(None);
        buffer.insert(0, "    indented line");
        let movement = BufferMovement::new(&buffer);
        
        let pos = Position::new(0, 10);
        let smart = movement.move_to_line_start(pos, true);
        assert_eq!(smart.column, 4); // First non-whitespace
        
        let normal = movement.move_to_line_start(pos, false);
        assert_eq!(normal.column, 0); // Absolute start
    }

    #[test]
    fn test_word_boundaries() {
        let buffer = create_test_buffer();
        let movement = BufferMovement::new(&buffer);
        
        let pos = Position::new(0, 0);
        let next = movement.move_to_next_word(pos);
        assert!(next.column > 0);
        assert_eq!(next.column, 6); // Should be at "world"
        
        let prev = movement.move_to_prev_word(Position::new(0, 6));
        assert_eq!(prev.column, 0);
    }

    #[test]
    fn test_matching_brackets() {
        let mut buffer = Buffer::new(None);
        buffer.insert(0, "fn main() { if (x) { y(); } }");
        let movement = BufferMovement::new(&buffer);
        
        let pos = Position::new(0, 8); // Position at '('
        let matching = movement.move_to_matching_bracket(pos);
        assert!(matching.is_some());
        assert_eq!(matching.unwrap().column, 9); // Should be at ')'
        
        let pos = Position::new(0, 17); // Position at '{'
        let matching = movement.move_to_matching_bracket(pos);
        assert!(matching.is_some());
        assert_eq!(matching.unwrap().column, 28); // Should be at '}'
    }

    #[test]
    fn test_paragraph_movement() {
        let buffer = create_test_buffer();
        let movement = BufferMovement::new(&buffer);
        
        let pos = Position::new(0, 0);
        let next_para = movement.move_to_next_paragraph(pos);
        assert_eq!(next_para.line, 2); // Empty line after first paragraph
        
        let next_para2 = movement.move_to_next_paragraph(next_para);
        assert_eq!(next_para2.line, 4); // End of document
        
        let prev_para = movement.move_to_prev_paragraph(Position::new(4, 0));
        assert_eq!(prev_para.line, 2); // Empty line before last paragraph
    }
}