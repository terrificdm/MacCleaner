use serde::{Deserialize, Serialize};

/// One cleanable entry shown in a result list.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct Item {
    pub path: String,
    pub name: String,
    pub size: u64,
    /// Grouping key; the frontend translates known keys (`groups.<key>`).
    pub group: String,
    /// "file" | "dir" | "command"
    pub kind: String,
    /// Default selection suggested by the scanner.
    pub selected: bool,
    /// Optional i18n note code, e.g. "running", "apple", "sameVendor".
    pub note: Option<String>,
    pub modified: Option<i64>,
    pub last_used: Option<i64>,
    /// For kind == "command": "brewCleanup" | "dockerPrune".
    pub command: Option<String>,
    pub needs_admin: bool,
}

impl Item {
    pub fn path(path: &std::path::Path, group: &str, size: u64, selected: bool) -> Item {
        let is_dir = std::fs::symlink_metadata(path).map(|m| m.is_dir()).unwrap_or(false);
        Item {
            path: crate::util::path_str(path),
            name: crate::util::file_name(path),
            size,
            group: group.into(),
            kind: if is_dir { "dir".into() } else { "file".into() },
            selected,
            ..Default::default()
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CleanTarget {
    pub path: String,
    /// Size seen at scan time, re-verified before acting.
    pub size: u64,
    #[serde(default)]
    pub kind: String,
    #[serde(default)]
    pub command: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct Done {
    pub path: String,
    pub size: u64,
    pub trash_path: Option<String>,
    /// "trash" | "finder" | "command" | "dryRun" | "emptyTrash"
    pub method: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Failed {
    pub path: String,
    /// i18n code: errors.<code>
    pub code: String,
    pub detail: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct CleanResult {
    pub dry_run: bool,
    pub done: Vec<Done>,
    pub failed: Vec<Failed>,
    /// Deliberately not processed, not an error (e.g. a newer copy exists).
    #[serde(default)]
    pub skipped: Vec<Failed>,
    pub total: u64,
}
