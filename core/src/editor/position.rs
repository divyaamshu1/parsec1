//! Position and range types for the editor

use serde::{Serialize, Deserialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct Position {
    pub line: usize,
    pub column: usize,
}

impl Position {
    pub fn new(line: usize, column: usize) -> Self {
        Self { line, column }
    }
    pub fn start_of_line(line: usize) -> Self {
        Self { line, column: 0 }
    }
    pub fn is_start(&self) -> bool {
        self.line == 0 && self.column == 0
    }
    pub fn add_columns(&self, offset: usize) -> Self {
        Self {
            line: self.line,
            column: self.column + offset,
        }
    }
    pub fn min(a: Position, b: Position) -> Position {
        if a < b { a } else { b }
    }
    pub fn max(a: Position, b: Position) -> Position {
        if a > b { a } else { b }
    }
}

impl Default for Position {
    fn default() -> Self {
        Self { line: 0, column: 0 }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Range {
    pub start: Position,
    pub end: Position,
}

impl Range {
    pub fn new(start: Position, end: Position) -> Self {
        Self { start, end }
    }
    pub fn point(at: Position) -> Self {
        Self { start: at, end: at }
    }
    pub fn contains(&self, pos: Position) -> bool {
        self.start <= pos && pos < self.end
    }
    pub fn intersect(&self, other: &Range) -> Option<Range> {
        let start = Position::max(self.start, other.start);
        let end = Position::min(self.end, other.end);
        if start <= end {
            Some(Range::new(start, end))
        } else {
            None
        }
    }
}
