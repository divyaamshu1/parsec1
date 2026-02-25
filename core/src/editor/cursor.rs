//! Multi-cursor management for the editor

use super::Position;

/// Manages multiple cursors in the editor (VS Code style multi-cursor support)
#[derive(Debug, Clone)]
pub struct CursorManager {
    /// List of cursor positions
    cursors: Vec<Cursor>,
    /// Index of primary cursor (the one that shows as main cursor)
    primary: usize,
    /// Whether multiple cursors are enabled
    multi_cursor_enabled: bool,
}

/// Individual cursor with its own state
#[derive(Debug, Clone)]
pub struct Cursor {
    /// Current position
    position: Position,
    /// Preferred column for vertical movement (maintains column when moving up/down)
    preferred_column: Option<usize>,
    /// Whether this cursor is currently selecting
    selecting: bool,
    /// Selection start position
    selection_start: Option<Position>,
}

impl Cursor {
    /// Create a new cursor at given position
    pub fn new(position: Position) -> Self {
        Self {
            position,
            preferred_column: Some(position.column),
            selecting: false,
            selection_start: None,
        }
    }

    /// Get cursor position
    pub fn position(&self) -> Position {
        self.position
    }

    /// Set cursor position
    pub fn set_position(&mut self, pos: Position) {
        self.position = pos;
        self.preferred_column = Some(pos.column);
    }

    /// Start selection from current position
    pub fn start_selection(&mut self) {
        self.selecting = true;
        self.selection_start = Some(self.position);
    }

    /// End selection
    pub fn end_selection(&mut self) {
        self.selecting = false;
        self.selection_start = None;
    }

    /// Get selection range (start, end) if selecting
    pub fn selection_range(&self) -> Option<(Position, Position)> {
        if self.selecting {
            if let Some(start) = self.selection_start {
                return Some((start, self.position));
            }
        }
        None
    }

    /// Check if cursor is currently selecting
    pub fn is_selecting(&self) -> bool {
        self.selecting
    }

    /// Get preferred column for vertical movement
    pub fn preferred_column(&self) -> Option<usize> {
        self.preferred_column
    }

    /// Set preferred column
    pub fn set_preferred_column(&mut self, col: usize) {
        self.preferred_column = Some(col);
    }
}

impl CursorManager {
    /// Create a new cursor manager with a single cursor at (0,0)
    pub fn new() -> Self {
        Self {
            cursors: vec![Cursor::new(Position::default())],
            primary: 0,
            multi_cursor_enabled: true,
        }
    }

    /// Get primary cursor position
    pub fn primary(&self) -> Position {
        self.cursors[self.primary].position()
    }

    /// Get primary cursor reference
    pub fn primary_cursor(&self) -> &Cursor {
        &self.cursors[self.primary]
    }

    /// Get primary cursor mutable reference
    pub fn primary_cursor_mut(&mut self) -> &mut Cursor {
        &mut self.cursors[self.primary]
    }

    /// Get all cursor positions
    pub fn positions(&self) -> Vec<Position> {
        self.cursors.iter().map(|c| c.position()).collect()
    }

    /// Get all cursors
    pub fn cursors(&self) -> &[Cursor] {
        &self.cursors
    }

    /// Get mutable cursors
    pub fn cursors_mut(&mut self) -> &mut [Cursor] {
        &mut self.cursors
    }

    /// Add a cursor at position (if multi-cursor enabled)
    pub fn add_cursor(&mut self, position: Position) -> bool {
        if !self.multi_cursor_enabled {
            return false;
        }
        
        // Check for duplicates
        if self.cursors.iter().any(|c| c.position() == position) {
            return false;
        }
        
        self.cursors.push(Cursor::new(position));
        true
    }

    /// Remove cursor at index (can't remove last cursor)
    pub fn remove_cursor(&mut self, index: usize) -> bool {
        if self.cursors.len() <= 1 {
            return false; // Keep at least one cursor
        }
        
        if index < self.cursors.len() {
            self.cursors.remove(index);
            if self.primary >= self.cursors.len() {
                self.primary = self.cursors.len() - 1;
            }
            true
        } else {
            false
        }
    }

    /// Clear all cursors except primary
    pub fn clear_secondary(&mut self) {
        let primary = self.cursors[self.primary].clone();
        self.cursors = vec![primary];
        self.primary = 0;
    }

    /// Move all cursors by delta (for column editing)
    pub fn move_all(&mut self, delta_line: i32, delta_column: i32) {
        for cursor in &mut self.cursors {
            let new_line = (cursor.position().line as i32 + delta_line).max(0) as usize;
            let new_col = (cursor.position().column as i32 + delta_column).max(0) as usize;
            
            cursor.set_position(Position::new(new_line, new_col));
        }
    }

    /// Move all cursors to the same position (collapsing multiple cursors)
    pub fn collapse_to_primary(&mut self) {
        let primary_pos = self.primary();
        self.cursors = vec![Cursor::new(primary_pos)];
        self.primary = 0;
    }

    /// Get the number of cursors
    pub fn count(&self) -> usize {
        self.cursors.len()
    }

    /// Check if there are multiple cursors
    pub fn has_multiple(&self) -> bool {
        self.cursors.len() > 1
    }

    /// Update cursor position at index
    pub fn update_position(&mut self, index: usize, new_position: Position) {
        if index < self.cursors.len() {
            self.cursors[index].set_position(new_position);
        }
    }

    /// Start selection at all cursor positions
    pub fn start_selection_all(&mut self) {
        for cursor in &mut self.cursors {
            cursor.start_selection();
        }
    }

    /// End selection at all cursor positions
    pub fn end_selection_all(&mut self) {
        for cursor in &mut self.cursors {
            cursor.end_selection();
        }
    }

    /// Get all selection ranges
    pub fn selection_ranges(&self) -> Vec<(Position, Position)> {
        self.cursors
            .iter()
            .filter_map(|c| c.selection_range())
            .collect()
    }

    /// Enable/disable multi-cursor mode
    pub fn set_multi_cursor_enabled(&mut self, enabled: bool) {
        self.multi_cursor_enabled = enabled;
        if !enabled && self.cursors.len() > 1 {
            self.collapse_to_primary();
        }
    }

    /// Check if multi-cursor is enabled
    pub fn is_multi_cursor_enabled(&self) -> bool {
        self.multi_cursor_enabled
    }
}

impl Default for CursorManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cursor_manager_new() {
        let manager = CursorManager::new();
        assert_eq!(manager.count(), 1);
        assert_eq!(manager.primary(), Position::default());
    }

    #[test]
    fn test_add_cursor() {
        let mut manager = CursorManager::new();
        let pos = Position::new(5, 10);
        
        assert!(manager.add_cursor(pos));
        assert_eq!(manager.count(), 2);
        
        // Can't add duplicate
        assert!(!manager.add_cursor(pos));
        assert_eq!(manager.count(), 2);
    }

    #[test]
    fn test_remove_cursor() {
        let mut manager = CursorManager::new();
        let pos = Position::new(5, 10);
        
        manager.add_cursor(pos);
        assert_eq!(manager.count(), 2);
        
        assert!(manager.remove_cursor(1));
        assert_eq!(manager.count(), 1);
        
        // Can't remove last cursor
        assert!(!manager.remove_cursor(0));
        assert_eq!(manager.count(), 1);
    }

    #[test]
    fn test_move_all() {
        let mut manager = CursorManager::new();
        manager.add_cursor(Position::new(5, 10));
        
        manager.move_all(2, 3);
        
        let positions = manager.positions();
        assert_eq!(positions[0], Position::new(2, 3));
        assert_eq!(positions[1], Position::new(7, 13));
    }

    #[test]
    fn test_selection() {
        let mut manager = CursorManager::new();
        manager.start_selection_all();
        
        manager.move_all(0, 5);
        
        let ranges = manager.selection_ranges();
        assert_eq!(ranges.len(), 1);
        assert_eq!(ranges[0].0, Position::new(0, 0));
        assert_eq!(ranges[0].1, Position::new(0, 5));
    }
}