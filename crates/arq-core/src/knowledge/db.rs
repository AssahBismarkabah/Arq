//! SurrealDB embedded database for the knowledge graph.

use std::path::Path;
use surrealdb::engine::local::{Db, RocksDb};
use surrealdb::Surreal;

use super::error::KnowledgeError;
use super::models::{CodeChunk, FileNode, IndexStats, SearchResult};

/// Database connection for the knowledge graph.
pub struct KnowledgeDb {
    db: Surreal<Db>,
}

impl KnowledgeDb {
    /// Open or create a database at the given path.
    pub async fn open(path: &Path) -> Result<Self, KnowledgeError> {
        let db = Surreal::new::<RocksDb>(path).await?;
        db.use_ns("arq").use_db("knowledge").await?;

        Ok(Self { db })
    }

    /// Initialize the database schema with rich ontology support.
    pub async fn initialize_schema(&self) -> Result<(), KnowledgeError> {
        // ===========================================================================
        // NODE TABLES - Code Entities
        // ===========================================================================

        // File table (structure node)
        self.db
            .query(
                r#"
                DEFINE TABLE file SCHEMAFULL;
                DEFINE FIELD path ON file TYPE string;
                DEFINE FIELD name ON file TYPE string;
                DEFINE FIELD extension ON file TYPE string;
                DEFINE FIELD hash ON file TYPE string;
                DEFINE FIELD size ON file TYPE int;
                DEFINE FIELD language ON file TYPE option<string>;
                DEFINE FIELD indexed_at ON file TYPE datetime;
                DEFINE INDEX file_path ON file FIELDS path UNIQUE;
                "#,
            )
            .await?;

        // Function node (rich ontology)
        self.db
            .query(
                r#"
                DEFINE TABLE fn_node SCHEMALESS;
                DEFINE FIELD name ON fn_node TYPE string;
                DEFINE FIELD qualified_name ON fn_node TYPE string;
                DEFINE FIELD file_path ON fn_node TYPE string;
                DEFINE FIELD start_line ON fn_node TYPE int;
                DEFINE FIELD end_line ON fn_node TYPE int;
                DEFINE INDEX fn_node_name ON fn_node FIELDS name;
                DEFINE INDEX fn_node_qualified ON fn_node FIELDS qualified_name;
                DEFINE INDEX fn_node_file ON fn_node FIELDS file_path;
                "#,
            )
            .await?;

        // Struct node (rich ontology)
        self.db
            .query(
                r#"
                DEFINE TABLE struct_node SCHEMALESS;
                DEFINE FIELD name ON struct_node TYPE string;
                DEFINE FIELD qualified_name ON struct_node TYPE string;
                DEFINE FIELD file_path ON struct_node TYPE string;
                DEFINE FIELD start_line ON struct_node TYPE int;
                DEFINE FIELD end_line ON struct_node TYPE int;
                DEFINE INDEX struct_name ON struct_node FIELDS name;
                DEFINE INDEX struct_qualified ON struct_node FIELDS qualified_name;
                DEFINE INDEX struct_file ON struct_node FIELDS file_path;
                "#,
            )
            .await?;

        // Trait node
        self.db
            .query(
                r#"
                DEFINE TABLE trait_node SCHEMALESS;
                DEFINE FIELD name ON trait_node TYPE string;
                DEFINE FIELD qualified_name ON trait_node TYPE string;
                DEFINE FIELD file_path ON trait_node TYPE string;
                DEFINE FIELD start_line ON trait_node TYPE int;
                DEFINE FIELD end_line ON trait_node TYPE int;
                DEFINE INDEX trait_name ON trait_node FIELDS name;
                DEFINE INDEX trait_file ON trait_node FIELDS file_path;
                "#,
            )
            .await?;

        // Impl node
        self.db
            .query(
                r#"
                DEFINE TABLE impl_node SCHEMALESS;
                DEFINE FIELD target_type ON impl_node TYPE string;
                DEFINE FIELD trait_name ON impl_node TYPE option<string>;
                DEFINE FIELD file_path ON impl_node TYPE string;
                DEFINE FIELD start_line ON impl_node TYPE int;
                DEFINE FIELD end_line ON impl_node TYPE int;
                DEFINE INDEX impl_target ON impl_node FIELDS target_type;
                DEFINE INDEX impl_trait ON impl_node FIELDS trait_name;
                DEFINE INDEX impl_file ON impl_node FIELDS file_path;
                "#,
            )
            .await?;

        // Enum node
        self.db
            .query(
                r#"
                DEFINE TABLE enum_node SCHEMALESS;
                DEFINE FIELD name ON enum_node TYPE string;
                DEFINE FIELD qualified_name ON enum_node TYPE string;
                DEFINE FIELD file_path ON enum_node TYPE string;
                DEFINE FIELD start_line ON enum_node TYPE int;
                DEFINE FIELD end_line ON enum_node TYPE int;
                DEFINE INDEX enum_name ON enum_node FIELDS name;
                DEFINE INDEX enum_file ON enum_node FIELDS file_path;
                "#,
            )
            .await?;

        // Constant node (const and static)
        self.db
            .query(
                r#"
                DEFINE TABLE const_node SCHEMALESS;
                DEFINE FIELD name ON const_node TYPE string;
                DEFINE FIELD qualified_name ON const_node TYPE string;
                DEFINE FIELD file_path ON const_node TYPE string;
                DEFINE FIELD line ON const_node TYPE int;
                DEFINE FIELD is_static ON const_node TYPE bool;
                DEFINE INDEX const_name ON const_node FIELDS name;
                DEFINE INDEX const_file ON const_node FIELDS file_path;
                "#,
            )
            .await?;

        // ===========================================================================
        // VECTOR SEARCH TABLE - Code chunks with embeddings
        // ===========================================================================

        self.db
            .query(
                r#"
                DEFINE TABLE chunk SCHEMAFULL;
                DEFINE FIELD file_path ON chunk TYPE string;
                DEFINE FIELD entity_id ON chunk TYPE option<string>;
                DEFINE FIELD entity_type ON chunk TYPE string;
                DEFINE FIELD content ON chunk TYPE string;
                DEFINE FIELD start_line ON chunk TYPE int;
                DEFINE FIELD end_line ON chunk TYPE int;
                DEFINE FIELD embedding ON chunk TYPE array<float>;
                DEFINE INDEX chunk_embedding ON chunk FIELDS embedding HNSW DIMENSION 384 DIST COSINE;
                DEFINE INDEX chunk_file ON chunk FIELDS file_path;
                "#,
            )
            .await?;

        // ===========================================================================
        // EDGE TABLES - Relations (using SurrealDB graph edges)
        // ===========================================================================

        // Structural edges
        self.db
            .query(
                r#"
                DEFINE TABLE contains TYPE RELATION;
                DEFINE TABLE belongs_to TYPE RELATION;
                DEFINE TABLE imports TYPE RELATION;
                DEFINE TABLE exports TYPE RELATION;
                DEFINE TABLE depends_on TYPE RELATION;
                "#,
            )
            .await?;

        // Behavioral edges - using regular tables for calls to support create()
        self.db
            .query(
                r#"
                DEFINE TABLE calls SCHEMAFULL;
                DEFINE FIELD caller_id ON calls TYPE string;
                DEFINE FIELD callee_id ON calls TYPE string;
                DEFINE FIELD caller_name ON calls TYPE string;
                DEFINE FIELD callee_name ON calls TYPE string;
                DEFINE INDEX idx_calls_caller ON calls FIELDS caller_id;
                DEFINE INDEX idx_calls_callee ON calls FIELDS callee_id;
                DEFINE INDEX idx_calls_caller_name ON calls FIELDS caller_name;
                DEFINE INDEX idx_calls_callee_name ON calls FIELDS callee_name;

                DEFINE TABLE returns TYPE RELATION;
                DEFINE TABLE throws TYPE RELATION;
                DEFINE TABLE awaits TYPE RELATION;
                DEFINE TABLE reads TYPE RELATION;
                DEFINE TABLE writes TYPE RELATION;
                "#,
            )
            .await?;

        // Type system edges - implements as regular table
        self.db
            .query(
                r#"
                DEFINE TABLE implements SCHEMAFULL;
                DEFINE FIELD impl_id ON implements TYPE string;
                DEFINE FIELD trait_id ON implements TYPE string;
                DEFINE INDEX idx_impl_impl ON implements FIELDS impl_id;
                DEFINE INDEX idx_impl_trait ON implements FIELDS trait_id;

                DEFINE TABLE extends TYPE RELATION;
                DEFINE TABLE uses_type TYPE RELATION;
                DEFINE TABLE returns_type TYPE RELATION;
                DEFINE TABLE has_field TYPE RELATION;
                "#,
            )
            .await?;

        // API edges
        self.db
            .query(
                r#"
                DEFINE TABLE exposes TYPE RELATION;
                DEFINE TABLE maps_to TYPE RELATION;
                DEFINE TABLE consumes TYPE RELATION;
                DEFINE TABLE produces TYPE RELATION;
                "#,
            )
            .await?;

        // Test edges
        self.db
            .query(
                r#"
                DEFINE TABLE tests TYPE RELATION;
                "#,
            )
            .await?;

        // ===========================================================================
        // BACKWARD COMPATIBILITY - Keep old table for migration
        // ===========================================================================

        // Legacy struct table (for backward compatibility)
        self.db
            .query(
                r#"
                DEFINE TABLE struct SCHEMAFULL;
                DEFINE FIELD name ON struct TYPE string;
                DEFINE FIELD file_path ON struct TYPE string;
                DEFINE FIELD start_line ON struct TYPE int;
                DEFINE FIELD end_line ON struct TYPE int;
                DEFINE FIELD visibility ON struct TYPE string;
                DEFINE FIELD doc_comment ON struct TYPE option<string>;
                DEFINE INDEX struct_name_legacy ON struct FIELDS name;
                "#,
            )
            .await?;

        // ===========================================================================
        // METADATA
        // ===========================================================================

        self.db
            .query(
                r#"
                DEFINE TABLE metadata SCHEMAFULL;
                DEFINE FIELD key ON metadata TYPE string;
                DEFINE FIELD value ON metadata TYPE any;
                DEFINE FIELD updated_at ON metadata TYPE datetime;
                DEFINE INDEX metadata_key ON metadata FIELDS key UNIQUE;

                INSERT INTO metadata { key: 'initialized', value: true, updated_at: time::now() };
                INSERT INTO metadata { key: 'schema_version', value: '2.0', updated_at: time::now() };
                "#,
            )
            .await?;

        Ok(())
    }

    /// Check if the database has been initialized.
    pub async fn is_initialized(&self) -> Result<bool, KnowledgeError> {
        let result: Option<serde_json::Value> = self
            .db
            .query("SELECT value FROM metadata WHERE key = 'initialized'")
            .await?
            .take(0)?;

        Ok(result.is_some())
    }

    /// Insert or update a file node.
    pub async fn upsert_file(&self, file: &FileNode) -> Result<(), KnowledgeError> {
        let path = file.path.clone();
        self.db
            .query("DELETE file WHERE path = $path")
            .bind(("path", path))
            .await?;

        let _: Option<FileNode> = self.db.create("file").content(file.clone()).await?;
        Ok(())
    }

    /// Get a file by path.
    pub async fn get_file(&self, path: &str) -> Result<Option<FileNode>, KnowledgeError> {
        let path_owned = path.to_string();
        let mut result = self
            .db
            .query("SELECT * FROM file WHERE path = $path")
            .bind(("path", path_owned))
            .await?;

        let file: Option<FileNode> = result.take(0)?;
        Ok(file)
    }

    /// Remove a file and its associated chunks.
    pub async fn remove_file(&self, path: &str) -> Result<(), KnowledgeError> {
        // Delete in separate queries - SurrealDB 2.0 multi-statement issues
        let path_owned = path.to_string();
        self.db
            .query("DELETE chunk WHERE file_path = $path")
            .bind(("path", path_owned.clone()))
            .await?;
        self.db
            .query("DELETE struct WHERE file_path = $path")
            .bind(("path", path_owned.clone()))
            .await?;
        self.db
            .query("DELETE fn_node WHERE file_path = $path")
            .bind(("path", path_owned.clone()))
            .await?;
        self.db
            .query("DELETE file WHERE path = $path")
            .bind(("path", path_owned))
            .await?;

        Ok(())
    }

    /// Insert a code chunk.
    pub async fn insert_chunk(&self, chunk: &CodeChunk) -> Result<(), KnowledgeError> {
        let _: Option<CodeChunk> = self.db.create("chunk").content(chunk.clone()).await?;
        Ok(())
    }

    /// Insert a struct node.
    pub async fn insert_struct(&self, s: &super::models::StructNode) -> Result<String, KnowledgeError> {
        let result: Option<super::models::StructNode> = self.db.create("struct").content(s.clone()).await?;
        let id = result
            .and_then(|r| r.id)
            .map(|t| t.to_string())
            .unwrap_or_else(|| format!("struct:{}", s.name));
        Ok(id)
    }

    /// Insert a function node.
    pub async fn insert_function(&self, f: &super::models::FunctionNode) -> Result<String, KnowledgeError> {
        let result: Option<super::models::FunctionNode> = self.db.create("fn_node").content(f.clone()).await?;
        let id = result
            .and_then(|r| r.id)
            .map(|t| t.to_string())
            .unwrap_or_else(|| format!("fn_node:{}", f.name));
        Ok(id)
    }

    /// Create a "contains" relation (file contains struct/function).
    pub async fn relate_contains(&self, file_path: &str, entity_id: &str) -> Result<(), KnowledgeError> {
        // First find the file record
        let file: Option<FileNode> = self
            .db
            .query("SELECT * FROM file WHERE path = $path LIMIT 1")
            .bind(("path", file_path.to_string()))
            .await?
            .take(0)?;

        if let Some(f) = file {
            if let Some(file_id) = f.id {
                let query = format!("RELATE {}->contains->{}", file_id, entity_id);
                self.db.query(&query).await?;
            }
        }
        Ok(())
    }

    /// Create a "calls" relation (function calls another function).
    pub async fn relate_calls(&self, caller_id: &str, callee_name: &str) -> Result<(), KnowledgeError> {
        // First find if the callee exists
        let callee: Option<super::models::FunctionNode> = self
            .db
            .query("SELECT * FROM fn_node WHERE name = $name LIMIT 1")
            .bind(("name", callee_name.to_string()))
            .await?
            .take(0)?;

        // Only create relation if callee exists
        if let Some(c) = callee {
            if let Some(callee_id) = c.id {
                let query = format!("RELATE {}->calls->{}", caller_id, callee_id);
                self.db.query(&query).await?;
            }
        }
        Ok(())
    }

    /// Count call relations in the database (for debugging).
    pub async fn count_calls(&self) -> Result<usize, KnowledgeError> {
        #[derive(serde::Deserialize)]
        struct CountResult {
            count: i64,
        }

        let result: Option<CountResult> = self
            .db
            .query("SELECT count() FROM calls GROUP ALL")
            .await?
            .take(0)?;

        Ok(result.map(|r| r.count as usize).unwrap_or(0))
    }

    /// Create a "has_method" relation (struct has method).
    pub async fn relate_has_method(&self, struct_id: &str, method_id: &str) -> Result<(), KnowledgeError> {
        let query = format!("RELATE {}->has_method->{}", struct_id, method_id);
        self.db.query(&query).await?;
        Ok(())
    }

    /// Search for chunks by embedding similarity.
    pub async fn search_by_embedding(
        &self,
        embedding: &[f32],
        limit: usize,
    ) -> Result<Vec<SearchResult>, KnowledgeError> {
        // K must be a literal in HNSW query, format it directly
        let query = format!(
            r#"
            SELECT
                file_path as path,
                vector::similarity::cosine(embedding, $embedding) as score,
                start_line,
                end_line,
                string::slice(content, 0, 200) as preview,
                entity_id,
                entity_type
            FROM chunk
            WHERE embedding <|{},COSINE|> $embedding
            ORDER BY score DESC
            "#,
            limit
        );

        let results: Vec<SearchResult> = self
            .db
            .query(&query)
            .bind(("embedding", embedding.to_vec()))
            .await?
            .take(0)?;

        Ok(results)
    }

    /// Get entities that the given entity depends on (what it calls).
    pub async fn get_dependencies(&self, entity_id: &str) -> Result<Vec<String>, KnowledgeError> {
        // Extract function name from entity_id (format: "function:path:name" or "fn_node:name")
        // Also trim backticks that SurrealDB may add for escaping
        let func_name = entity_id
            .rsplit(':')
            .next()
            .unwrap_or(entity_id)
            .trim_matches('`');

        #[derive(serde::Deserialize)]
        struct DepResult {
            callee_name: String,
        }

        let results: Vec<DepResult> = self
            .db
            .query("SELECT callee_name FROM calls WHERE caller_name = $name")
            .bind(("name", func_name.to_string()))
            .await?
            .take(0)?;

        // Get unique callee names
        let mut names: Vec<String> = results.into_iter().map(|r| r.callee_name).collect();
        names.sort();
        names.dedup();
        Ok(names)
    }

    /// Get entities that depend on the given entity (what calls it).
    pub async fn get_impact(&self, entity_id: &str) -> Result<Vec<String>, KnowledgeError> {
        // Extract function name from entity_id (format: "function:path:name" or just "name")
        // Also trim backticks that SurrealDB may add for escaping
        let func_name = entity_id
            .rsplit(':')
            .next()
            .unwrap_or(entity_id)
            .trim_matches('`');

        #[derive(serde::Deserialize)]
        struct ImpactResult {
            caller_name: String,
        }

        let results: Vec<ImpactResult> = self
            .db
            .query("SELECT caller_name FROM calls WHERE callee_name = $name")
            .bind(("name", func_name.to_string()))
            .await?
            .take(0)?;

        // Get unique caller names
        let mut names: Vec<String> = results.into_iter().map(|r| r.caller_name).collect();
        names.sort();
        names.dedup();
        Ok(names)
    }

    /// List all functions in the database.
    pub async fn list_functions(&self, limit: usize) -> Result<Vec<super::models::FunctionNode>, KnowledgeError> {
        let query = format!("SELECT * FROM fn_node LIMIT {}", limit);
        let results: Vec<super::models::FunctionNode> = self.db.query(&query).await?.take(0)?;
        Ok(results)
    }

    /// Find a function by name.
    pub async fn find_function_by_name(&self, name: &str) -> Result<Option<super::models::FunctionNode>, KnowledgeError> {
        let result: Option<super::models::FunctionNode> = self
            .db
            .query("SELECT * FROM fn_node WHERE name = $name LIMIT 1")
            .bind(("name", name.to_string()))
            .await?
            .take(0)?;
        Ok(result)
    }

    /// Get statistics about the indexed data.
    pub async fn get_stats(&self) -> Result<IndexStats, KnowledgeError> {
        // SurrealDB returns count as { count: N }
        #[derive(serde::Deserialize)]
        struct CountResult {
            count: i64,
        }

        let files: Option<CountResult> = self
            .db
            .query("SELECT count() FROM file GROUP ALL")
            .await?
            .take(0)?;
        let structs: Option<CountResult> = self
            .db
            .query("SELECT count() FROM struct GROUP ALL")
            .await?
            .take(0)?;
        let functions: Option<CountResult> = self
            .db
            .query("SELECT count() FROM fn_node GROUP ALL")
            .await?
            .take(0)?;
        let chunks: Option<CountResult> = self
            .db
            .query("SELECT count() FROM chunk GROUP ALL")
            .await?
            .take(0)?;

        Ok(IndexStats {
            files: files.map(|r| r.count as usize).unwrap_or(0),
            structs: structs.map(|r| r.count as usize).unwrap_or(0),
            functions: functions.map(|r| r.count as usize).unwrap_or(0),
            chunks: chunks.map(|r| r.count as usize).unwrap_or(0),
            total_size: 0, // TODO: Calculate from file sizes
            last_updated: Some(chrono::Utc::now()),
        })
    }

    // ===========================================================================
    // RICH ONTOLOGY METHODS
    // ===========================================================================

    /// Insert a rich function entity.
    pub async fn insert_function_entity(
        &self,
        func: &super::ontology::nodes::FunctionEntity,
    ) -> Result<String, KnowledgeError> {
        let _: Option<serde_json::Value> = self
            .db
            .create("fn_node")
            .content(func.clone())
            .await?;

        let id = func.id.clone().unwrap_or_else(|| {
            format!("fn_node:{}", func.qualified_name)
        });
        Ok(id)
    }

    /// Insert a rich struct entity.
    pub async fn insert_struct_entity(
        &self,
        s: &super::ontology::nodes::StructEntity,
    ) -> Result<String, KnowledgeError> {
        let _: Option<serde_json::Value> = self
            .db
            .create("struct_node")
            .content(s.clone())
            .await?;

        let id = s.id.clone().unwrap_or_else(|| {
            format!("struct_node:{}", s.qualified_name)
        });
        Ok(id)
    }

    /// Insert a trait entity.
    pub async fn insert_trait_entity(
        &self,
        t: &super::ontology::nodes::TraitEntity,
    ) -> Result<String, KnowledgeError> {
        let _: Option<serde_json::Value> = self
            .db
            .create("trait_node")
            .content(t.clone())
            .await?;

        let id = t.id.clone().unwrap_or_else(|| {
            format!("trait_node:{}", t.qualified_name)
        });
        Ok(id)
    }

    /// Insert an impl entity.
    pub async fn insert_impl_entity(
        &self,
        i: &super::ontology::nodes::ImplEntity,
    ) -> Result<String, KnowledgeError> {
        let _: Option<serde_json::Value> = self
            .db
            .create("impl_node")
            .content(i.clone())
            .await?;

        let id = i.id.clone().unwrap_or_else(|| {
            if let Some(ref trait_name) = i.trait_name {
                format!("impl_node:{}_for_{}", trait_name, i.target_type)
            } else {
                format!("impl_node:{}", i.target_type)
            }
        });
        Ok(id)
    }

    /// Insert an enum entity.
    pub async fn insert_enum_entity(
        &self,
        e: &super::ontology::nodes::EnumEntity,
    ) -> Result<String, KnowledgeError> {
        let _: Option<serde_json::Value> = self
            .db
            .create("enum_node")
            .content(e.clone())
            .await?;

        let id = e.id.clone().unwrap_or_else(|| {
            format!("enum_node:{}", e.qualified_name)
        });
        Ok(id)
    }

    /// Insert a constant entity.
    pub async fn insert_const_entity(
        &self,
        c: &super::ontology::nodes::ConstantEntity,
    ) -> Result<String, KnowledgeError> {
        let _: Option<serde_json::Value> = self
            .db
            .create("const_node")
            .content(c.clone())
            .await?;

        let id = c.id.clone().unwrap_or_else(|| {
            format!("const_node:{}", c.qualified_name)
        });
        Ok(id)
    }

    /// Create a generic relation between two entities.
    pub async fn create_relation(
        &self,
        from_id: &str,
        relation: &str,
        to_id: &str,
    ) -> Result<(), KnowledgeError> {
        // Store as records in relation-specific tables for proper querying
        match relation {
            "calls" => {
                self.store_call_edge(from_id, to_id).await?;
            }
            "implements" => {
                self.store_implements_edge(from_id, to_id).await?;
            }
            _ => {
                // For other relations, try RELATE with properly escaped IDs
                let from_escaped = Self::escape_record_id(from_id);
                let to_escaped = Self::escape_record_id(to_id);
                let query = format!("RELATE {}->{}->{}",from_escaped, relation, to_escaped);
                let _ = self.db.query(&query).await;
            }
        }
        Ok(())
    }

    /// Store a call edge as a regular record.
    pub async fn store_call_edge(&self, caller_id: &str, callee_id: &str) -> Result<(), KnowledgeError> {
        #[derive(serde::Serialize)]
        struct CallRecord {
            caller_id: String,
            callee_id: String,
            caller_name: String,
            callee_name: String,
        }

        // Extract names from IDs (format: function:path:name)
        let caller_name = caller_id.rsplit(':').next().unwrap_or(caller_id).to_string();
        let callee_name = callee_id.rsplit(':').next().unwrap_or(callee_id).to_string();

        let record = CallRecord {
            caller_id: caller_id.to_string(),
            callee_id: callee_id.to_string(),
            caller_name,
            callee_name,
        };

        let _: Option<serde_json::Value> = self.db.create("calls").content(record).await?;
        Ok(())
    }

    /// Store an implements edge as a regular record.
    pub async fn store_implements_edge(&self, from_id: &str, to_id: &str) -> Result<(), KnowledgeError> {
        #[derive(serde::Serialize)]
        struct ImplementsRecord {
            impl_id: String,
            trait_id: String,
        }

        let record = ImplementsRecord {
            impl_id: from_id.to_string(),
            trait_id: to_id.to_string(),
        };

        let _: Option<serde_json::Value> = self.db.create("implements").content(record).await?;
        Ok(())
    }

    /// Escape a record ID for use in SurrealDB queries.
    fn escape_record_id(id: &str) -> String {
        // If ID contains special chars, wrap the id part in backticks
        if let Some(pos) = id.find(':') {
            let (table, rest) = id.split_at(pos);
            let id_part = &rest[1..]; // skip the colon
            if id_part.contains(':') || id_part.contains('/') || id_part.contains('.') {
                format!("{}:`{}`", table, id_part)
            } else {
                id.to_string()
            }
        } else {
            id.to_string()
        }
    }

    /// Remove all entities associated with a file (v2 schema).
    pub async fn remove_file_entities(&self, path: &str) -> Result<(), KnowledgeError> {
        let path_owned = path.to_string();

        // Delete from all node tables
        for table in &["fn_node", "struct_node", "trait_node", "impl_node", "enum_node", "const_node", "chunk"] {
            self.db
                .query(&format!("DELETE {} WHERE file_path = $path", table))
                .bind(("path", path_owned.clone()))
                .await?;
        }

        // Delete file itself
        self.db
            .query("DELETE file WHERE path = $path")
            .bind(("path", path_owned))
            .await?;

        Ok(())
    }

    /// Get extended statistics including new entity types.
    pub async fn get_extended_stats(&self) -> Result<ExtendedIndexStats, KnowledgeError> {
        #[derive(serde::Deserialize)]
        struct CountResult {
            count: i64,
        }

        async fn count_table(db: &Surreal<Db>, table: &str) -> Result<usize, KnowledgeError> {
            let result: Option<CountResult> = db
                .query(&format!("SELECT count() FROM {} GROUP ALL", table))
                .await?
                .take(0)?;
            Ok(result.map(|r| r.count as usize).unwrap_or(0))
        }

        Ok(ExtendedIndexStats {
            files: count_table(&self.db, "file").await?,
            functions: count_table(&self.db, "fn_node").await?,
            structs: count_table(&self.db, "struct_node").await?,
            traits: count_table(&self.db, "trait_node").await?,
            impls: count_table(&self.db, "impl_node").await?,
            enums: count_table(&self.db, "enum_node").await?,
            constants: count_table(&self.db, "const_node").await?,
            chunks: count_table(&self.db, "chunk").await?,
            calls: count_table(&self.db, "calls").await?,
            implements: count_table(&self.db, "implements").await?,
        })
    }
}

/// Extended statistics for the rich ontology.
#[derive(Debug, Clone, Default)]
pub struct ExtendedIndexStats {
    pub files: usize,
    pub functions: usize,
    pub structs: usize,
    pub traits: usize,
    pub impls: usize,
    pub enums: usize,
    pub constants: usize,
    pub chunks: usize,
    pub calls: usize,
    pub implements: usize,
}
