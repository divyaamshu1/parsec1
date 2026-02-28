//! Terminal split pane management

use slotmap::{SlotMap, new_key_type};
use std::collections::HashMap;
use crate::Result;

new_key_type! {
    /// Pane identifier
    pub struct PaneId;
}

/// Layout direction
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Direction {
    Horizontal,
    Vertical,
}

/// Pane node in split tree
#[derive(Debug, Clone)]
enum PaneNode {
    Leaf {
        pane_id: PaneId,
        size: Option<u16>,
        min_size: u16,
    },
    Split {
        direction: Direction,
        children: Vec<PaneId>,
        sizes: Vec<f32>,
    },
}

/// Split pane
#[derive(Debug, Clone)]
pub struct SplitPane {
    pub id: PaneId,
    pub terminal_id: String,
    pub title: String,
    pub active: bool,
    pub x: u16,
    pub y: u16,
    pub width: u16,
    pub height: u16,
}

/// Split manager
pub struct SplitManager {
    panes: SlotMap<PaneId, SplitPane>,
    tree: PaneNode,
    root_id: Option<PaneId>,
    total_width: u16,
    total_height: u16,
}

impl SplitManager {
    /// Create a new split manager
    pub fn new() -> Self {
        Self {
            panes: SlotMap::with_key(),
            tree: PaneNode::Leaf {
                pane_id: PaneId::default(),
                size: None,
                min_size: 10,
            },
            root_id: None,
            total_width: 80,
            total_height: 24,
        }
    }

    /// Add a new pane
    pub fn add_pane(
        &mut self,
        pane_id: PaneId,
        direction: Option<Direction>,
    ) -> Result<()> {
        let pane = SplitPane {
            id: pane_id,
            terminal_id: format!("term-pane"),
            title: format!("Terminal {}", self.panes.len() + 1),
            active: self.panes.is_empty(),
            x: 0,
            y: 0,
            width: self.total_width,
            height: self.total_height,
        };

        // insert pane into slotmap; ignore the provided id for now
        let _ = self.panes.insert(pane);

        // continue with split logic if direction specified

        if let Some(dir) = direction {
            if let Some(root_id) = self.root_id {
                // Split existing root
                let old_root = std::mem::replace(
                    &mut self.tree,
                    PaneNode::Split {
                        direction: dir,
                        children: vec![root_id, pane_id],
                        sizes: vec![0.5, 0.5],
                    },
                );
                
                match old_root {
                    PaneNode::Leaf { pane_id: old_id, .. } => {
                        if let PaneNode::Split { ref mut children, .. } = self.tree {
                            children[0] = old_id;
                        }
                    }
                    _ => {}
                }
            } else {
                self.root_id = Some(pane_id);
                self.tree = PaneNode::Leaf {
                    pane_id,
                    size: None,
                    min_size: 10,
                };
            }
        } else {
            self.root_id = Some(pane_id);
            self.tree = PaneNode::Leaf {
                pane_id,
                size: None,
                min_size: 10,
            };
        }

        self.layout();
        Ok(())
    }

    /// Remove a pane
    pub fn remove_pane(&mut self, pane_id: PaneId) -> Result<()> {
        self.panes.remove(pane_id);
        
        // Rebuild tree
        self.rebuild_tree();
        self.layout();
        
        Ok(())
    }

    /// Rebuild tree after removal
    fn rebuild_tree(&mut self) {
        if let Some(root_id) = self.root_id {
            if !self.panes.contains_key(root_id) {
                // Root was removed, find new root
                self.root_id = self.panes.keys().next();
            }
        }

        if let Some(root_id) = self.root_id {
            self.tree = PaneNode::Leaf {
                pane_id: root_id,
                size: None,
                min_size: 10,
            };
        }
    }

    /// Layout panes
    fn layout(&mut self) {
        if let Some(_root_id) = self.root_id {
            let tree = self.tree.clone();
            self.layout_node(&tree, 0, 0, self.total_width, self.total_height);
        }
    }

    /// Layout a node recursively
    fn layout_node(
        &mut self,
        node: &PaneNode,
        x: u16,
        y: u16,
        width: u16,
        height: u16,
    ) {
        match node {
            PaneNode::Leaf { pane_id, .. } => {
                if let Some(pane) = self.panes.get_mut(*pane_id) {
                    pane.x = x;
                    pane.y = y;
                    pane.width = width;
                    pane.height = height;
                }
            }
            PaneNode::Split { direction, children, sizes } => {
                if children.is_empty() {
                    return;
                }

                match direction {
                    Direction::Horizontal => {
                        let mut current_x = x;
                        for (i, child_id) in children.iter().enumerate() {
                            let child_width = if i == children.len() - 1 {
                                x + width - current_x
                            } else {
                                (width as f32 * sizes[i]) as u16
                            };
                            
                            if let Some(child_node) = self.get_node(*child_id) {
                                self.layout_node(
                                    &child_node,
                                    current_x,
                                    y,
                                    child_width,
                                    height,
                                );
                            }
                            
                            current_x += child_width;
                        }
                    }
                    Direction::Vertical => {
                        let mut current_y = y;
                        for (i, child_id) in children.iter().enumerate() {
                            let child_height = if i == children.len() - 1 {
                                y + height - current_y
                            } else {
                                (height as f32 * sizes[i]) as u16
                            };
                            
                            if let Some(child_node) = self.get_node(*child_id) {
                                self.layout_node(
                                    &child_node,
                                    x,
                                    current_y,
                                    width,
                                    child_height,
                                );
                            }
                            
                            current_y += child_height;
                        }
                    }
                }
            }
        }
    }

    /// Get node by pane ID
    fn get_node(&self, pane_id: PaneId) -> Option<PaneNode> {
        // Simplified - would need to traverse tree
        Some(PaneNode::Leaf {
            pane_id,
            size: None,
            min_size: 10,
        })
    }

    /// Resize split
    pub fn resize_split(&mut self, _pane_id: PaneId, _delta: i32) -> Result<()> {
        // Find parent split and adjust sizes
        self.layout();
        Ok(())
    }

    /// Set total size
    pub fn set_size(&mut self, width: u16, height: u16) {
        self.total_width = width;
        self.total_height = height;
        self.layout();
    }

    /// Get all panes
    pub fn panes(&self) -> Vec<SplitPane> {
        self.panes.values().cloned().collect()
    }

    /// Get active pane
    pub fn active_pane(&self) -> Option<SplitPane> {
        self.panes.values().find(|p| p.active).cloned()
    }

    /// Set active pane
    pub fn set_active(&mut self, pane_id: PaneId) {
        for pane in self.panes.values_mut() {
            pane.active = pane.id == pane_id;
        }
    }
}

impl Default for SplitManager {
    fn default() -> Self {
        Self::new()
    }
}