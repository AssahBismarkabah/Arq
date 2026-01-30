//! Graph building logic for the visualization server.
//!
//! This module handles the conversion from KnowledgeGraph entities
//! to the graph format expected by Sigma.js/Graphology.

use std::collections::{HashMap, HashSet};

use arq_core::knowledge::KnowledgeGraph;

use super::models::{EdgeAttributes, GraphData, GraphEdge, GraphNode, NodeAttributes};

// =============================================================================
// Node Styling (Language-Agnostic)
// =============================================================================

/// Get the hex color for a category.
/// Colors are designed to be visually distinct.
fn get_category_color(category: &str) -> &'static str {
    match category {
        "function" | "method" => "#0969da",    // Blue
        "struct" | "class" => "#1a7f37",       // Green
        "trait" | "interface" => "#9a6700",    // Yellow/Orange
        "enum" => "#cf222e",                   // Red
        "impl" | "implementation" => "#8250df", // Purple
        _ => "#57606a",                        // Gray (default)
    }
}

/// Get the node size for a category.
fn get_category_size(category: &str) -> u32 {
    match category {
        "struct" | "class" | "trait" | "interface" | "enum" => 12,
        "impl" | "implementation" => 10,
        "function" | "method" => 8,
        _ => 6,
    }
}

// =============================================================================
// Graph Builder
// =============================================================================

/// Builder for constructing graph data from knowledge graph.
pub struct GraphBuilder {
    nodes: Vec<GraphNode>,
    edges: Vec<GraphEdge>,
    seen_keys: HashSet<String>,
    seen_edges: HashSet<String>,
    id_to_key: HashMap<String, String>,
}

impl GraphBuilder {
    /// Create a new graph builder.
    pub fn new() -> Self {
        Self {
            nodes: Vec::new(),
            edges: Vec::new(),
            seen_keys: HashSet::new(),
            seen_edges: HashSet::new(),
            id_to_key: HashMap::new(),
        }
    }

    /// Build graph data from a knowledge graph.
    pub async fn build_from_kg(self, kg: &KnowledgeGraph) -> GraphData {
        let mut builder = self;

        // Load all entity types
        builder.load_functions(kg).await;
        builder.load_structs(kg).await;
        builder.load_traits(kg).await;
        builder.load_enums(kg).await;
        builder.load_impls(kg).await;

        // Build ID mapping for edge resolution
        builder.build_id_mapping(kg).await;

        // Load edges
        builder.load_call_edges(kg).await;

        GraphData {
            nodes: builder.nodes,
            edges: builder.edges,
        }
    }

    /// Load function nodes from knowledge graph.
    async fn load_functions(&mut self, kg: &KnowledgeGraph) {
        if let Ok(functions) = kg.list_all_functions().await {
            for func in functions {
                let key = Self::make_key("fn", &func.file_path, func.start_line, &func.name);
                self.add_node(key, func.name, "function", Some(func.file_path), Some(func.start_line), Some(func.end_line));
            }
        }
    }

    /// Load struct/class nodes from knowledge graph.
    async fn load_structs(&mut self, kg: &KnowledgeGraph) {
        if let Ok(structs) = kg.list_structs().await {
            for s in structs {
                let key = Self::make_key("struct", &s.file_path, s.start_line, &s.name);
                self.add_node(key, s.name, "struct", Some(s.file_path), Some(s.start_line), Some(s.end_line));
            }
        }
    }

    /// Load trait/interface nodes from knowledge graph.
    async fn load_traits(&mut self, kg: &KnowledgeGraph) {
        if let Ok(traits) = kg.list_traits().await {
            for t in traits {
                let key = Self::make_key("trait", &t.file_path, t.start_line, &t.name);
                self.add_node(key, t.name, "trait", Some(t.file_path), Some(t.start_line), Some(t.end_line));
            }
        }
    }

    /// Load enum nodes from knowledge graph.
    async fn load_enums(&mut self, kg: &KnowledgeGraph) {
        if let Ok(enums) = kg.list_enums().await {
            for e in enums {
                let key = Self::make_key("enum", &e.file_path, e.start_line, &e.name);
                self.add_node(key, e.name, "enum", Some(e.file_path), Some(e.start_line), Some(e.end_line));
            }
        }
    }

    /// Load impl/implementation nodes from knowledge graph.
    async fn load_impls(&mut self, kg: &KnowledgeGraph) {
        if let Ok(impls) = kg.list_impls().await {
            for i in impls {
                // Extract just the type name (remove generics and clean up)
                let target = Self::extract_simple_name(&i.target_type);

                // Create a short label for the impl
                let label = if let Some(ref trait_name) = i.trait_name {
                    let trait_short = Self::extract_simple_name(trait_name);
                    format!("{} for {}", trait_short, target)
                } else {
                    target.clone()
                };

                let key = Self::make_key("impl", &i.file_path, i.start_line, &label);
                self.add_node(key, label, "impl", Some(i.file_path), Some(i.start_line), Some(i.end_line));
            }
        }
    }

    /// Build ID-to-key mapping for edge resolution.
    async fn build_id_mapping(&mut self, kg: &KnowledgeGraph) {
        if let Ok(functions) = kg.list_all_functions().await {
            for func in functions {
                let key = Self::make_key("fn", &func.file_path, func.start_line, &func.name);

                // Map various ID formats to the canonical key
                self.id_to_key.insert(func.qualified_name.clone(), key.clone());
                self.id_to_key.insert(format!("function:{}:{}", func.file_path, func.name), key.clone());
                self.id_to_key.insert(func.name.clone(), key.clone());
            }
        }
    }

    /// Load call relationship edges from knowledge graph.
    async fn load_call_edges(&mut self, kg: &KnowledgeGraph) {
        if let Ok(calls) = kg.list_calls().await {
            for call in calls {
                // Resolve source and target node keys
                let source = self.id_to_key.get(&call.caller_id)
                    .or_else(|| self.id_to_key.get(&call.caller_name))
                    .cloned();

                let target = self.id_to_key.get(&call.callee_id)
                    .or_else(|| self.id_to_key.get(&call.callee_name))
                    .cloned();

                if let (Some(src), Some(tgt)) = (source, target) {
                    // Only add if both nodes exist and edge is unique
                    if self.seen_keys.contains(&src) && self.seen_keys.contains(&tgt) {
                        let edge_key = format!("{}:{}", src, tgt);

                        if self.seen_edges.insert(edge_key) {
                            self.edges.push(GraphEdge {
                                source: src,
                                target: tgt,
                                attributes: Some(EdgeAttributes {
                                    relationship: format!("{:?}", call.call_type),
                                }),
                            });
                        }
                    }
                }
            }
        }
    }

    // =========================================================================
    // Helper Methods
    // =========================================================================

    /// Create a unique key for a node.
    fn make_key(prefix: &str, file_path: &str, start_line: u32, name: &str) -> String {
        format!("{}:{}:{}:{}", prefix, file_path, start_line, name)
    }

    /// Extract a simple name from a potentially complex type string.
    /// Removes generics, doc comments, and other noise.
    fn extract_simple_name(s: &str) -> String {
        // Take only the part before '<' (generics) or '{' (body) or '(' (params)
        let name = s.split(&['<', '{', '(', '#'][..])
            .next()
            .unwrap_or(s)
            .trim();

        // If it starts with "impl ", extract what comes after
        let name = name.strip_prefix("impl ").unwrap_or(name);

        // Take the last segment if it's a path (e.g., "std::fmt::Display" -> "Display")
        let name = name.rsplit("::").next().unwrap_or(name);

        // Limit length
        if name.len() > 30 {
            format!("{}...", &name[..27])
        } else {
            name.to_string()
        }
    }

    /// Add a node if not already present.
    fn add_node(
        &mut self,
        key: String,
        label: String,
        category: &str,
        file: Option<String>,
        start_line: Option<u32>,
        end_line: Option<u32>,
    ) {
        if self.seen_keys.insert(key.clone()) {
            self.nodes.push(GraphNode {
                key,
                attributes: NodeAttributes {
                    label,
                    category: category.to_string(),
                    color: get_category_color(category).to_string(),
                    size: get_category_size(category),
                    file,
                    start_line,
                    end_line,
                },
            });
        }
    }
}

impl Default for GraphBuilder {
    fn default() -> Self {
        Self::new()
    }
}
