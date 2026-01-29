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

    /// Initialize the database schema.
    pub async fn initialize_schema(&self) -> Result<(), KnowledgeError> {
        // File table
        self.db
            .query(
                r#"
                DEFINE TABLE file SCHEMAFULL;
                DEFINE FIELD path ON file TYPE string;
                DEFINE FIELD name ON file TYPE string;
                DEFINE FIELD extension ON file TYPE string;
                DEFINE FIELD hash ON file TYPE string;
                DEFINE FIELD size ON file TYPE int;
                DEFINE FIELD indexed_at ON file TYPE datetime;
                DEFINE INDEX file_path ON file FIELDS path UNIQUE;
                "#,
            )
            .await?;

        // Struct table
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
                DEFINE INDEX struct_name ON struct FIELDS name;
                "#,
            )
            .await?;

        // Function table (fn_node to avoid reserved keyword)
        self.db
            .query(
                r#"
                DEFINE TABLE fn_node SCHEMAFULL;
                DEFINE FIELD name ON fn_node TYPE string;
                DEFINE FIELD file_path ON fn_node TYPE string;
                DEFINE FIELD parent_struct ON fn_node TYPE option<string>;
                DEFINE FIELD start_line ON fn_node TYPE int;
                DEFINE FIELD end_line ON fn_node TYPE int;
                DEFINE FIELD visibility ON fn_node TYPE string;
                DEFINE FIELD is_async ON fn_node TYPE bool;
                DEFINE FIELD signature ON fn_node TYPE string;
                DEFINE FIELD doc_comment ON fn_node TYPE option<string>;
                DEFINE INDEX fn_node_name ON fn_node FIELDS name;
                "#,
            )
            .await?;

        // Code chunk table with vector index
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
                "#,
            )
            .await?;

        // Graph relations
        self.db
            .query(
                r#"
                DEFINE TABLE contains TYPE RELATION;
                DEFINE TABLE calls TYPE RELATION;
                DEFINE TABLE has_method TYPE RELATION;
                DEFINE TABLE implements TYPE RELATION;
                "#,
            )
            .await?;

        // Metadata table
        self.db
            .query(
                r#"
                DEFINE TABLE metadata SCHEMAFULL;
                DEFINE FIELD key ON metadata TYPE string;
                DEFINE FIELD value ON metadata TYPE any;
                DEFINE FIELD updated_at ON metadata TYPE datetime;
                DEFINE INDEX metadata_key ON metadata FIELDS key UNIQUE;

                INSERT INTO metadata { key: 'initialized', value: true, updated_at: time::now() };
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
        self.db
            .query("RELATE (SELECT id FROM file WHERE path = $file_path LIMIT 1)->contains->$entity_id")
            .bind(("file_path", file_path.to_string()))
            .bind(("entity_id", entity_id.to_string()))
            .await?;
        Ok(())
    }

    /// Create a "calls" relation (function calls another function).
    pub async fn relate_calls(&self, caller_id: &str, callee_name: &str) -> Result<(), KnowledgeError> {
        // Find the callee by name and create relation
        self.db
            .query("RELATE $caller_id->calls->(SELECT id FROM fn_node WHERE name = $callee_name LIMIT 1)")
            .bind(("caller_id", caller_id.to_string()))
            .bind(("callee_name", callee_name.to_string()))
            .await?;
        Ok(())
    }

    /// Create a "has_method" relation (struct has method).
    pub async fn relate_has_method(&self, struct_id: &str, method_id: &str) -> Result<(), KnowledgeError> {
        self.db
            .query("RELATE $struct_id->has_method->$method_id")
            .bind(("struct_id", struct_id.to_string()))
            .bind(("method_id", method_id.to_string()))
            .await?;
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

    /// Get entities that the given entity depends on.
    pub async fn get_dependencies(&self, entity_id: &str) -> Result<Vec<String>, KnowledgeError> {
        let entity_id_owned = entity_id.to_string();
        let results: Vec<String> = self
            .db
            .query(
                r#"
                SELECT out as dep FROM calls WHERE in = $entity_id
                "#,
            )
            .bind(("entity_id", entity_id_owned))
            .await?
            .take(0)?;

        Ok(results)
    }

    /// Get entities that depend on the given entity (reverse dependencies).
    pub async fn get_impact(&self, entity_id: &str) -> Result<Vec<String>, KnowledgeError> {
        let entity_id_owned = entity_id.to_string();
        let results: Vec<String> = self
            .db
            .query(
                r#"
                SELECT in as caller FROM calls WHERE out = $entity_id
                "#,
            )
            .bind(("entity_id", entity_id_owned))
            .await?
            .take(0)?;

        Ok(results)
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
}
