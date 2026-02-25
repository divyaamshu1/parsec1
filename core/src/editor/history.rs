//! Undo/Redo history management for the editor

use super::{Position, Range};
use chrono::{DateTime, Utc};

/// Represents an edit operation that can be undone/redone
#[derive(Debug, Clone)]
pub enum EditOperation {
    /// Text insertion at position
    Insert {
        position: Position,
        text: String,
    },
    /// Text deletion at position
    Delete {
        position: Position,
        text: String,
    },
    /// Text replacement over a range
    Replace {
        range: Range,
        text: String,
    },
}

impl EditOperation {
    /// Get the inverse operation (for undo)
    pub fn inverse(&self) -> Self {
        match self {
            EditOperation::Insert { position, text } => EditOperation::Delete {
                position: *position,
                text: text.clone(),
            },
            EditOperation::Delete { position, text } => EditOperation::Insert {
                position: *position,
                text: text.clone(),
            },
            EditOperation::Replace { range, text } => EditOperation::Replace {
                range: *range,
                text: text.clone(), // Note: This doesn't store the original text
                // In a real implementation, you'd need to store both old and new text
            },
        }
    }

    /// Get the affected range of this operation
    pub fn range(&self) -> Range {
        match self {
            EditOperation::Insert { position, text } => {
                Range::new(*position, position.add_columns(text.len()))
            }
            EditOperation::Delete { position, text } => {
                Range::new(*position, position.add_columns(text.len()))
            }
            EditOperation::Replace { range, .. } => *range,
        }
    }
}

/// Group of operations that should be undone/redone together
#[derive(Debug, Clone)]
struct OperationGroup {
    /// Operations in this group
    operations: Vec<EditOperation>,
    /// Timestamp when group was created
    timestamp: DateTime<Utc>,
    /// Description of the group (for UI display)
    description: Option<String>,
}

impl OperationGroup {
    fn new(description: Option<String>) -> Self {
        Self {
            operations: Vec::new(),
            timestamp: Utc::now(),
            description,
        }
    }

    fn push(&mut self, operation: EditOperation) {
        self.operations.push(operation);
    }

    fn is_empty(&self) -> bool {
        self.operations.is_empty()
    }

    fn len(&self) -> usize {
        self.operations.len()
    }

    /// Get the combined range of all operations in the group
    fn range(&self) -> Option<Range> {
        if self.operations.is_empty() {
            return None;
        }

        let mut start = self.operations[0].range().start;
        let mut end = self.operations[0].range().end;

        for op in &self.operations {
            let op_range = op.range();
            start = Position::min(start, op_range.start);
            end = Position::max(end, op_range.end);
        }

        Some(Range::new(start, end))
    }
}

/// Manages undo/redo history with support for operation grouping
#[derive(Debug)]
pub struct EditHistory {
    /// Past operations (for undo)
    undo_stack: Vec<OperationGroup>,
    /// Future operations (for redo)
    redo_stack: Vec<OperationGroup>,
    /// Current group being built (for combining multiple operations)
    current_group: Option<OperationGroup>,
    /// Maximum history size
    max_size: usize,
    /// Whether we're in the middle of a group operation
    in_group: bool,
    /// Current position in history (for limiting)
    position: usize,
}

impl EditHistory {
    /// Create a new edit history with given max size
    pub fn new(max_size: usize) -> Self {
        Self {
            undo_stack: Vec::with_capacity(max_size),
            redo_stack: Vec::with_capacity(max_size),
            current_group: None,
            max_size,
            in_group: false,
            position: 0,
        }
    }

    /// Start a group of operations (all operations until `end_group` will be undone/redone together)
    pub fn begin_group(&mut self, description: Option<String>) {
        self.in_group = true;
        self.current_group = Some(OperationGroup::new(description));
    }

    /// End the current group and add it to history
    pub fn end_group(&mut self) {
        if let Some(group) = self.current_group.take() {
            if !group.is_empty() {
                self.push_group(group);
            }
        }
        self.in_group = false;
    }

    /// Record an edit operation
    pub fn record(&mut self, operation: EditOperation) {
        // Clear redo stack on new operation
        self.redo_stack.clear();
        
        if self.in_group {
            // Add to current group
            if let Some(group) = &mut self.current_group {
                group.push(operation);
            }
        } else {
            // Create single-operation group
            let _group = OperationGroup::new(None);
            let mut group = OperationGroup::new(None);
            group.push(operation);
            self.push_group(group);
        }
    }

    /// Push a group to undo stack
    fn push_group(&mut self, group: OperationGroup) {
        self.undo_stack.push(group);
        
        // Limit stack size
        while self.undo_stack.len() > self.max_size {
            self.undo_stack.remove(0);
        }
        
        self.position = self.undo_stack.len();
    }

    /// Undo last operation(s)
    pub fn undo(&mut self) -> Option<EditOperation> {
        if let Some(group) = self.undo_stack.pop() {
            // Move to redo stack
            self.redo_stack.push(group.clone());
            self.position = self.undo_stack.len();
            
            // If it's a single operation, return it directly
            if group.len() == 1 {
                Some(group.operations[0].inverse())
            } else {
                // For groups, we need to apply operations in reverse order
                // Return the first operation, the caller will need to handle the rest
                group.operations.last().map(|op| op.inverse())
            }
        } else {
            None
        }
    }

    /// Undo multiple steps
    pub fn undo_steps(&mut self, steps: usize) -> Vec<EditOperation> {
        let mut operations = Vec::new();
        for _ in 0..steps {
            if let Some(op) = self.undo() {
                operations.push(op);
            } else {
                break;
            }
        }
        operations
    }

    /// Redo last undone operation(s)
    pub fn redo(&mut self) -> Option<EditOperation> {
        if let Some(group) = self.redo_stack.pop() {
            self.undo_stack.push(group.clone());
            self.position = self.undo_stack.len();
            
            if group.len() == 1 {
                Some(group.operations[0].clone())
            } else {
                group.operations.first().cloned()
            }
        } else {
            None
        }
    }

    /// Redo multiple steps
    pub fn redo_steps(&mut self, steps: usize) -> Vec<EditOperation> {
        let mut operations = Vec::new();
        for _ in 0..steps {
            if let Some(op) = self.redo() {
                operations.push(op);
            } else {
                break;
            }
        }
        operations
    }

    /// Get the last operation description (for UI)
    pub fn last_operation(&self) -> Option<String> {
        self.undo_stack.last().and_then(|group| {
            if group.len() == 1 {
                match &group.operations[0] {
                    EditOperation::Insert { text, .. } => {
                        Some(format!("Insert \"{}\"", truncate(text, 20)))
                    }
                    EditOperation::Delete { text, .. } => {
                        Some(format!("Delete \"{}\"", truncate(text, 20)))
                    }
                    EditOperation::Replace { text, .. } => {
                        Some(format!("Replace with \"{}\"", truncate(text, 20)))
                    }
                }
            } else {
                group.description.clone()
                    .or_else(|| Some(format!("Group of {} operations", group.len())))
            }
        })
    }

    /// Clear history
    pub fn clear(&mut self) {
        self.undo_stack.clear();
        self.redo_stack.clear();
        self.current_group = None;
        self.in_group = false;
        self.position = 0;
    }

    /// Check if undo is available
    pub fn can_undo(&self) -> bool {
        !self.undo_stack.is_empty()
    }

    /// Check if redo is available
    pub fn can_redo(&self) -> bool {
        !self.redo_stack.is_empty()
    }

    /// Get current stack sizes
    pub fn sizes(&self) -> (usize, usize) {
        (self.undo_stack.len(), self.redo_stack.len())
    }

    /// Get current position in history
    pub fn position(&self) -> usize {
        self.position
    }

    /// Get total history size
    pub fn total_size(&self) -> usize {
        self.undo_stack.len() + self.redo_stack.len()
    }

    /// Merge adjacent groups if they are of the same type
    pub fn merge_adjacent(&mut self) {
        if self.undo_stack.len() < 2 {
            return;
        }

        let mut merged = Vec::new();
        let mut i = 0;

        while i < self.undo_stack.len() {
            let mut current = self.undo_stack[i].clone();
            let mut j = i + 1;

            while j < self.undo_stack.len() && should_merge(&current, &self.undo_stack[j]) {
                // Merge groups
                for op in &self.undo_stack[j].operations {
                    current.operations.push(op.clone());
                }
                j += 1;
            }

            merged.push(current);
            i = j;
        }

        self.undo_stack = merged;
    }
}

/// Helper to truncate strings for display
fn truncate(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else {
        format!("{}…", &s[..max_len])
    }
}

/// Check if two groups should be merged (simplified heuristic)
fn should_merge(group1: &OperationGroup, group2: &OperationGroup) -> bool {
    // If they're within 1 second of each other, consider merging
    let time_diff = (group2.timestamp - group1.timestamp).num_milliseconds();
    time_diff.abs() < 1000
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_single_operation() {
        let mut history = EditHistory::new(10);
        
        history.record(EditOperation::Insert {
            position: Position::new(0, 0),
            text: "Hello".to_string(),
        });
        
        assert!(history.can_undo());
        assert!(!history.can_redo());
        
        let op = history.undo();
        assert!(op.is_some());
        assert!(!history.can_undo());
        assert!(history.can_redo());
        
        let op = history.redo();
        assert!(op.is_some());
        assert!(history.can_undo());
        assert!(!history.can_redo());
    }

    #[test]
    fn test_operation_group() {
        let mut history = EditHistory::new(10);
        
        history.begin_group(Some("Type 'Hello'".to_string()));
        history.record(EditOperation::Insert {
            position: Position::new(0, 0),
            text: "H".to_string(),
        });
        history.record(EditOperation::Insert {
            position: Position::new(0, 1),
            text: "e".to_string(),
        });
        history.record(EditOperation::Insert {
            position: Position::new(0, 2),
            text: "l".to_string(),
        });
        history.record(EditOperation::Insert {
            position: Position::new(0, 3),
            text: "l".to_string(),
        });
        history.record(EditOperation::Insert {
            position: Position::new(0, 4),
            text: "o".to_string(),
        });
        history.end_group();
        
        assert_eq!(history.sizes().0, 1);
        
        let op = history.undo();
        assert!(op.is_some());
        assert_eq!(history.sizes().0, 0);
        assert_eq!(history.sizes().1, 1);
    }

    #[test]
    fn test_max_size() {
        let mut history = EditHistory::new(3);
        
        for i in 0..5 {
            history.record(EditOperation::Insert {
                position: Position::new(0, i),
                text: "x".to_string(),
            });
        }
        
        assert_eq!(history.sizes().0, 3); // Only last 3 kept
    }

    #[test]
    fn test_undo_redo_steps() {
        let mut history = EditHistory::new(10);
        
        for i in 0..5 {
            history.record(EditOperation::Insert {
                position: Position::new(0, i),
                text: i.to_string(),
            });
        }
        
        let undone = history.undo_steps(3);
        assert_eq!(undone.len(), 3);
        assert_eq!(history.sizes().0, 2);
        assert_eq!(history.sizes().1, 3);
        
        let redone = history.redo_steps(2);
        assert_eq!(redone.len(), 2);
        assert_eq!(history.sizes().0, 4);
        assert_eq!(history.sizes().1, 1);
    }

    #[test]
    fn test_last_operation() {
        let mut history = EditHistory::new(10);
        
        history.record(EditOperation::Insert {
            position: Position::new(0, 0),
            text: "Hello world".to_string(),
        });
        
        assert_eq!(history.last_operation(), Some("Insert \"Hello world\"".to_string()));
    }
}