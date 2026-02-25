//! ER Diagram generator from database schemas

use std::collections::HashMap;

use anyhow::{Result, anyhow};
use petgraph::graph::{Graph, NodeIndex};
use petgraph::dot::{Dot, Config};

use crate::TableSchema;

/// ER Diagram generator
pub struct ERDiagramGenerator;

/// Table node in ER diagram
#[derive(Debug, Clone)]
struct TableNode {
    name: String,
    columns: Vec<String>,
    primary_key: Vec<String>,
}

/// Relationship between tables
#[derive(Debug, Clone)]
struct Relationship {
    from_table: String,
    from_column: String,
    to_table: String,
    to_column: String,
    relationship_type: RelationType,
}

/// Relationship type
#[derive(Debug, Clone, Copy)]
enum RelationType {
    OneToOne,
    OneToMany,
    ManyToOne,
    ManyToMany,
}

impl std::fmt::Display for TableNode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}\n({})", self.name, self.columns.join(", "))
    }
}

impl std::fmt::Display for Relationship {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}.{} -> {}.{}", self.from_table, self.from_column, self.to_table, self.to_column)
    }
}

impl ERDiagramGenerator {
    /// Create a new ER diagram generator
    pub fn new() -> Self {
        Self
    }

    /// Generate ER diagram from table schemas
    pub fn generate(&self, schemas: &[TableSchema]) -> Result<String> {
        let mut graph = Graph::<TableNode, Relationship>::new();
        let mut node_indices = HashMap::new();

        // Add table nodes
        for schema in schemas {
            let columns = schema.columns.iter()
                .map(|c| format!("{}: {}", c.name, c.data_type))
                .collect();

            let node = TableNode {
                name: schema.name.clone(),
                columns,
                primary_key: schema.primary_key.clone(),
            };

            let idx = graph.add_node(node);
            node_indices.insert(schema.name.clone(), idx);
        }

        // Add relationships based on foreign keys
        for schema in schemas {
            for fk in &schema.foreign_keys {
                if let (Some(from_idx), Some(to_idx)) = (
                    node_indices.get(&schema.name),
                    node_indices.get(&fk.foreign_table),
                ) {
                    let rel = Relationship {
                        from_table: schema.name.clone(),
                        from_column: fk.column.clone(),
                        to_table: fk.foreign_table.clone(),
                        to_column: fk.foreign_column.clone(),
                        relationship_type: RelationType::ManyToOne, // Default
                    };
                    graph.add_edge(*from_idx, *to_idx, rel);
                }
            }
        }

        // Generate DOT format
        let dot = Dot::with_config(&graph, &[Config::EdgeNoLabel]);
        Ok(format!("{}", dot))
    }

    /// Generate ASCII ER diagram (using custom algorithm, no dijkstra needed)
    pub fn generate_ascii(&self, schemas: &[TableSchema]) -> Result<String> {
        let mut output = String::new();

        for schema in schemas {
            output.push_str(&format!("+----------------------+\n"));
            output.push_str(&format!("|        {}        |\n", schema.name));
            output.push_str(&format!("+----------------------+\n"));

            for col in &schema.columns {
                let pk_marker = if schema.primary_key.contains(&col.name) {
                    "🔑 "
                } else {
                    "   "
                };
                output.push_str(&format!("| {}{}: {} |\n", pk_marker, col.name, col.data_type));
            }
            output.push_str(&format!("+----------------------+\n\n"));

            // Add relationships
            for fk in &schema.foreign_keys {
                output.push_str(&format!(
                    "    {} ──────→ {}.{}\n",
                    fk.column, fk.foreign_table, fk.foreign_column
                ));
            }
            if !schema.foreign_keys.is_empty() {
                output.push_str("\n");
            }
        }

        Ok(output)
    }

    /// Generate HTML ER diagram
    pub fn generate_html(&self, schemas: &[TableSchema]) -> Result<String> {
        let mut html = String::from(r#"<!DOCTYPE html>
<html>
<head>
    <title>ER Diagram</title>
    <style>
        body { font-family: Arial, sans-serif; margin: 20px; background: #1e1e1e; color: #fff; }
        .table { 
            border: 2px solid #444; 
            border-radius: 5px; 
            margin: 10px; 
            display: inline-block;
            vertical-align: top;
            background: #2d2d2d;
        }
        .table-name { 
            background: #007acc; 
            color: white; 
            padding: 8px; 
            font-weight: bold;
            border-radius: 3px 3px 0 0;
        }
        .columns { padding: 8px; }
        .column { padding: 4px; border-bottom: 1px solid #444; }
        .pk { color: #ffd700; }
        .fk { color: #98fb98; }
        .relationship { margin: 10px; color: #888; }
    </style>
</head>
<body>
    <h1>ER Diagram</h1>
    <div class="diagram">
"#);

        for schema in schemas {
            html.push_str(&format!(r#"
        <div class="table">
            <div class="table-name">{}</div>
            <div class="columns">
"#, schema.name));

            for col in &schema.columns {
                let pk_class = if schema.primary_key.contains(&col.name) { "pk" } else { "" };
                html.push_str(&format!(
                    r#"                <div class="column {}">{}: {}</div>\n"#,
                    pk_class, col.name, col.data_type
                ));
            }

            html.push_str(r#"            </div>
        </div>
"#);
        }

        html.push_str(r#"
    </div>
</body>
</html>"#);

        Ok(html)
    }

    /// Generate Mermaid ER diagram
    pub fn generate_mermaid(&self, schemas: &[TableSchema]) -> Result<String> {
        let mut mermaid = String::from("erDiagram\n");

        for schema in schemas {
            mermaid.push_str(&format!("    {} {{\n", schema.name));

            for col in &schema.columns {
                let pk_marker = if schema.primary_key.contains(&col.name) {
                    " PK"
                } else {
                    ""
                };
                mermaid.push_str(&format!("        {} {}{}\n", col.data_type, col.name, pk_marker));
            }

            mermaid.push_str("    }\n");
        }

        // Add relationships
        for schema in schemas {
            for fk in &schema.foreign_keys {
                mermaid.push_str(&format!(
                    "    {} ||--o{{ {} : \"{}\"\n",
                    fk.foreign_table, schema.name, fk.column
                ));
            }
        }

        Ok(mermaid)
    }
}

impl Default for ERDiagramGenerator {
    fn default() -> Self {
        Self::new()
    }
}