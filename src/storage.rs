use crate::{cf::CfClient, state::TempAccount};
use anyhow::{Context, Result};
use chrono::Utc;
use serde_json::Value;

#[derive(Debug, Clone)]
pub enum CommentStorage {
    D1 {
        database_id: String,
        database_name: String,
    },
    #[allow(dead_code)]
    Kv {
        namespace_id: String,
        namespace_name: String,
    },
}

impl CommentStorage {
    pub fn binding(&self) -> Value {
        match self {
            CommentStorage::D1 {
                database_id,
                database_name,
            } => {
                let _ = database_name;
                serde_json::json!({
                    "type": "d1",
                    "name": "DB",
                    "id": database_id
                })
            }
            CommentStorage::Kv {
                namespace_id,
                namespace_name,
            } => {
                let _ = namespace_name;
                serde_json::json!({
                    "type": "kv_namespace",
                    "name": "COMMENTS",
                    "namespace_id": namespace_id
                })
            }
        }
    }

    pub fn label(&self) -> &'static str {
        match self {
            CommentStorage::D1 { .. } => "d1",
            CommentStorage::Kv { .. } => "kv",
        }
    }
}

fn base36(mut value: u64) -> String {
    const DIGITS: &[u8; 36] = b"0123456789abcdefghijklmnopqrstuvwxyz";
    if value == 0 {
        return "0".to_string();
    }
    let mut out = Vec::new();
    while value > 0 {
        out.push(DIGITS[(value % 36) as usize] as char);
        value /= 36;
    }
    out.iter().rev().collect()
}

fn unique_storage_suffix() -> String {
    let millis = Utc::now().timestamp_millis().max(0) as u64;
    base36(millis)
}

fn comments_database_name(script_name: &str, suffix: &str) -> String {
    const SUFFIX: &str = "-comments";
    const MAX_NAME_LEN: usize = 54;
    let max_suffix_len = MAX_NAME_LEN.saturating_sub(SUFFIX.len() + 2);
    let suffix: String = suffix.chars().take(max_suffix_len).collect();
    let unique = format!("-{suffix}");
    let max_prefix_len = MAX_NAME_LEN.saturating_sub(unique.len() + SUFFIX.len());
    let prefix: String = script_name.chars().take(max_prefix_len).collect();
    format!("{prefix}{unique}{SUFFIX}")
}

pub fn provision_comments_storage(
    client: &CfClient,
    account: &TempAccount,
    script_name: &str,
) -> Result<CommentStorage> {
    let database_name = comments_database_name(script_name, &unique_storage_suffix());
    eprintln!("Creating temporary D1 database {database_name}...");
    let created = client.create_d1_database(account, &database_name)?;
    let database_id = created
        .get("uuid")
        .or_else(|| created.get("id"))
        .and_then(|v| v.as_str())
        .context("D1 create response did not include uuid/id")?
        .to_string();
    client.execute_d1_sql(
        account,
        &database_id,
        r#"
CREATE TABLE IF NOT EXISTS comments (
  id TEXT PRIMARY KEY,
  report_id TEXT NOT NULL,
  kind TEXT NOT NULL DEFAULT 'comment',
  author TEXT,
  body TEXT NOT NULL,
  anchor_text TEXT,
  selector TEXT,
  path TEXT NOT NULL DEFAULT '/',
  quote_context_before TEXT,
  quote_context_after TEXT,
  user_agent TEXT,
  ip_hash TEXT,
  created_at TEXT NOT NULL
);
CREATE INDEX IF NOT EXISTS comments_report_created_idx ON comments (report_id, created_at);
"#,
    )?;
    Ok(CommentStorage::D1 {
        database_id,
        database_name,
    })
}

#[cfg(test)]
mod tests {
    use super::{comments_database_name, CommentStorage};

    #[test]
    fn d1_binding_shape_matches_worker_metadata() {
        let binding = CommentStorage::D1 {
            database_id: "db-id".into(),
            database_name: "cfdrop-comments".into(),
        }
        .binding();
        assert_eq!(binding["type"], "d1");
        assert_eq!(binding["name"], "DB");
        assert_eq!(binding["id"], "db-id");
    }

    #[test]
    fn comments_database_name_leaves_room_for_suffix() {
        let name = comments_database_name("abcdefghijklmnopqrstuvwxyzabcdefghijklmnopqrstuvwxyz", "abc123");
        assert!(name.ends_with("-comments"));
        assert!(name.contains("-abc123-"));
        assert!(name.len() <= 54);
    }

    #[test]
    fn kv_binding_shape_matches_worker_metadata() {
        let binding = CommentStorage::Kv {
            namespace_id: "kv-id".into(),
            namespace_name: "cfdrop-comments".into(),
        }
        .binding();
        assert_eq!(binding["type"], "kv_namespace");
        assert_eq!(binding["name"], "COMMENTS");
        assert_eq!(binding["namespace_id"], "kv-id");
    }
}
