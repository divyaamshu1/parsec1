//! Text selection management for the editor

use super::{Position, Range};

/// Represents a text selection with multiple ranges (supports multi-cursor selections)
#[derive(Debug, Clone)]
pub struct Selection {
    /// Main selection ranges
    ranges: Vec<Range>,
    /// Index of primary range (the one that shows as main selection)
    primary: usize,
    /// Whether selections are reversed (cursor at start vs end)
    reversed: bool,
}

impl Selection {
    /// Create a new empty selection
    pub fn new() -> Self {
        Self {
            ranges: Vec::new(),
            primary: 0,
            reversed: false,
        }
    }

    /// Create a selection with a single range
    pub fn single(range: Range) -> Self {
        Self {
            ranges: vec![range],
            primary: 0,
            reversed: false,
        }
    }

    /// Create a selection at a single point (cursor position)
    pub fn point(position: Position) -> Self {
        Self::single(Range::point(position))
    }

    /// Add a range to the selection (merges overlapping ranges)
    pub fn add_range(&mut self, range: Range) {
        // Merge overlapping ranges
        let mut new_ranges = Vec::new();
        let mut inserted = false;
        
        for r in &self.ranges {
            if r.end < range.start {
                // Range is before new range
                new_ranges.push(*r);
            } else if r.start > range.end {
                // Range is after new range
                if !inserted {
                    new_ranges.push(range);
                    inserted = true;
                }
                new_ranges.push(*r);
            } else {
                // Ranges overlap, merge them
                let merged = Range::new(
                    Position::min(r.start, range.start),
                    Position::max(r.end, range.end)
                );
                if !inserted {
                    new_ranges.push(merged);
                    inserted = true;
                } else {
                    *new_ranges.last_mut().unwrap() = merged;
                }
            }
        }
        
        if !inserted {
            new_ranges.push(range);
        }
        
        self.ranges = new_ranges;
    }

    /// Remove a range from selection
    pub fn remove_range(&mut self, range: Range) {
        let mut new_ranges = Vec::new();
        
        for r in &self.ranges {
            if let Some(intersection) = r.intersect(&range) {
                // Split the range if needed
                if r.start < intersection.start {
                    new_ranges.push(Range::new(r.start, intersection.start));
                }
                if intersection.end < r.end {
                    new_ranges.push(Range::new(intersection.end, r.end));
                }
            } else {
                new_ranges.push(*r);
            }
        }
        
        self.ranges = new_ranges;
        if self.primary >= self.ranges.len() {
            self.primary = self.ranges.len().saturating_sub(1);
        }
    }

    /// Clear all selections
    pub fn clear(&mut self) {
        self.ranges.clear();
        self.primary = 0;
    }

    /// Check if selection is empty
    pub fn is_empty(&self) -> bool {
        self.ranges.is_empty()
    }

    /// Get all ranges
    pub fn ranges(&self) -> &[Range] {
        &self.ranges
    }

    /// Get primary range
    pub fn primary(&self) -> Option<Range> {
        self.ranges.get(self.primary).copied()
    }

    /// Set primary range index
    pub fn set_primary(&mut self, index: usize) -> bool {
        if index < self.ranges.len() {
            self.primary = index;
            true
        } else {
            false
        }
    }

    /// Get all cursor positions (ends of selections)
    pub fn cursor_positions(&self) -> Vec<Position> {
        let mut positions = Vec::new();
        
        for range in &self.ranges {
            if self.reversed {
                positions.push(range.start);
            } else {
                positions.push(range.end);
            }
        }
        
        positions
    }

    /// Get all selected text ranges
    pub fn selected_ranges(&self) -> Vec<Range> {
        self.ranges.clone()
    }

    /// Check if position is selected
    pub fn contains(&self, pos: Position) -> bool {
        self.ranges.iter().any(|r| r.contains(pos))
    }

    /// Get the extent of all selections (min start to max end)
    pub fn extent(&self) -> Option<Range> {
        if self.ranges.is_empty() {
            return None;
        }
        
        let mut start = self.ranges[0].start;
        let mut end = self.ranges[0].end;
        
        for range in &self.ranges {
            start = Position::min(start, range.start);
            end = Position::max(end, range.end);
        }
        
        Some(Range::new(start, end))
    }

    /// Merge adjacent ranges
    pub fn merge_adjacent(&mut self) {
        if self.ranges.len() <= 1 {
            return;
        }
        
        self.ranges.sort_by_key(|r| r.start);
        
        let mut merged = Vec::new();
        let mut current = self.ranges[0];
        
        for range in self.ranges.iter().skip(1) {
            if current.end >= range.start {
                // Ranges are adjacent or overlapping
                current = Range::new(current.start, Position::max(current.end, range.end));
            } else {
                merged.push(current);
                current = *range;
            }
        }
        merged.push(current);
        
        self.ranges = merged;
    }

    /// Toggle reverse flag
    pub fn set_reversed(&mut self, reversed: bool) {
        self.reversed = reversed;
    }

    /// Check if selection is reversed
    pub fn is_reversed(&self) -> bool {
        self.reversed
    }

    /// Get selected text from buffer content
    pub fn get_text(&self, content: &str) -> String {
        let lines: Vec<&str> = content.lines().collect();
        let mut result = String::new();
        
        for range in &self.ranges {
            if range.start.line == range.end.line {
                // Single line selection
                if let Some(line) = lines.get(range.start.line) {
                    let start = range.start.column;
                    let end = range.end.column.min(line.len());
                    if start < end {
                        result.push_str(&line[start..end]);
                    }
                }
            } else {
                // Multi-line selection
                for line_idx in range.start.line..=range.end.line {
                    if let Some(line) = lines.get(line_idx) {
                        if line_idx == range.start.line {
                            let start = range.start.column;
                            if start < line.len() {
                                result.push_str(&line[start..]);
                            }
                        } else if line_idx == range.end.line {
                            let end = range.end.column.min(line.len());
                            result.push_str(&line[..end]);
                        } else {
                            result.push_str(line);
                        }
                    }
                    if line_idx < range.end.line {
                        result.push('\n');
                    }
                }
            }
        }
        
        result
    }
}

impl Default for Selection {
    fn default() -> Self {
        Self::new()
    }
}

/// Manages multiple selections for multi-cursor editing
#[derive(Debug, Clone)]
pub struct SelectionManager {
    /// Primary selection
    primary: Selection,
    /// Additional selections
    selections: Vec<Selection>,
    /// Whether to merge selections automatically
    auto_merge: bool,
}

impl SelectionManager {
    /// Create a new selection manager
    pub fn new() -> Self {
        Self {
            primary: Selection::new(),
            selections: Vec::new(),
            auto_merge: true,
        }
    }

    /// Add a selection
    pub fn add_selection(&mut self, selection: Selection) {
        self.selections.push(selection);
        if self.auto_merge {
            self.merge_overlapping();
        }
    }

    /// Remove a selection at index
    pub fn remove_selection(&mut self, index: usize) -> bool {
        if index < self.selections.len() {
            self.selections.remove(index);
            true
        } else {
            false
        }
    }

    /// Get all selections
    pub fn all_selections(&self) -> Vec<&Selection> {
        let mut all = vec![&self.primary];
        all.extend(self.selections.iter());
        all
    }

    /// Get all selections mutably
    pub fn all_selections_mut(&mut self) -> Vec<&mut Selection> {
        let mut all = vec![&mut self.primary];
        all.extend(self.selections.iter_mut());
        all
    }

    /// Get all cursor positions across all selections
    pub fn all_cursor_positions(&self) -> Vec<Position> {
        let mut positions = self.primary.cursor_positions();
        
        for selection in &self.selections {
            positions.extend(selection.cursor_positions());
        }
        
        positions
    }

    /// Get all selected ranges across all selections
    pub fn all_selected_ranges(&self) -> Vec<Range> {
        let mut ranges = self.primary.selected_ranges();
        
        for selection in &self.selections {
            ranges.extend(selection.selected_ranges());
        }
        
        ranges
    }

    /// Check if any selection contains position
    pub fn contains(&self, pos: Position) -> bool {
        if self.primary.contains(pos) {
            return true;
        }
        
        for selection in &self.selections {
            if selection.contains(pos) {
                return true;
            }
        }
        
        false
    }

    /// Merge all selections if they overlap
    pub fn merge_overlapping(&mut self) {
        if !self.auto_merge {
            return;
        }
        
        let mut all_ranges = self.all_selected_ranges();
        all_ranges.sort_by_key(|r| r.start);
        
        let mut merged_ranges = Vec::new();
        let mut current = all_ranges[0];
        
        for range in all_ranges.iter().skip(1) {
            if current.end >= range.start {
                current = Range::new(current.start, Position::max(current.end, range.end));
            } else {
                merged_ranges.push(current);
                current = *range;
            }
        }
        merged_ranges.push(current);
        
        // Recreate selections from merged ranges
        self.primary = Selection::single(merged_ranges[0]);
        self.selections = merged_ranges[1..]
            .iter()
            .map(|r| Selection::single(*r))
            .collect();
    }

    /// Count total selections
    pub fn count(&self) -> usize {
        1 + self.selections.len()
    }

    /// Clear all selections
    pub fn clear(&mut self) {
        self.primary.clear();
        self.selections.clear();
    }

    /// Set auto-merge flag
    pub fn set_auto_merge(&mut self, auto_merge: bool) {
        self.auto_merge = auto_merge;
    }

    /// Check if auto-merge is enabled
    pub fn auto_merge(&self) -> bool {
        self.auto_merge
    }
}

impl Default for SelectionManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_selection_add_range() {
        let mut selection = Selection::new();
        
        selection.add_range(Range::new(
            Position::new(1, 5),
            Position::new(1, 10)
        ));
        assert_eq!(selection.ranges().len(), 1);
        
        selection.add_range(Range::new(
            Position::new(2, 0),
            Position::new(2, 5)
        ));
        assert_eq!(selection.ranges().len(), 2);
        
        selection.add_range(Range::new(
            Position::new(1, 8),
            Position::new(2, 3)
        ));
        assert_eq!(selection.ranges().len(), 1); // Should merge
    }

    #[test]
    fn test_selection_remove_range() {
        let mut selection = Selection::new();
        selection.add_range(Range::new(Position::new(1, 0), Position::new(1, 5)));
        selection.add_range(Range::new(Position::new(2, 0), Position::new(2, 5)));
        
        selection.remove_range(Range::new(Position::new(1, 2), Position::new(1, 3)));
        assert_eq!(selection.ranges().len(), 3); // Split into two ranges
    }

    #[test]
    fn test_selection_get_text() {
        let content = "Hello world\nThis is a test\nThird line";
        let mut selection = Selection::new();
        
        selection.add_range(Range::new(
            Position::new(0, 6),
            Position::new(0, 11)
        ));
        
        assert_eq!(selection.get_text(content), "world");
    }

    #[test]
    fn test_selection_manager() {
        let mut manager = SelectionManager::new();
        
        manager.primary = Selection::single(Range::new(
            Position::new(1, 0),
            Position::new(1, 5)
        ));
        
        manager.add_selection(Selection::single(Range::new(
            Position::new(3, 0),
            Position::new(3, 5)
        )));
        
        assert_eq!(manager.count(), 2);
        assert_eq!(manager.all_cursor_positions().len(), 2);
    }
}