use crate::model::{CleanResult, Done};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use std::sync::Mutex;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Record {
    pub id: String,
    pub time: i64,
    pub module: String,
    pub dry_run: bool,
    pub total: u64,
    pub failed: usize,
    pub items: Vec<Done>,
    /// Items of this record still in the Trash (computed when listing).
    #[serde(default)]
    pub restorable: usize,
    /// Old copies in the Trash whose original path has a newer copy.
    #[serde(default)]
    pub superseded: usize,
}

const MAX_RECORDS: usize = 500;
static LOCK: Mutex<()> = Mutex::new(());

fn file() -> PathBuf {
    crate::settings::data_dir().join("history.json")
}

pub fn list() -> Vec<Record> {
    let _g = LOCK.lock().unwrap();
    let mut all = read();
    for r in all.iter_mut() {
        r.restorable = if r.dry_run { 0 } else { restorable(r) };
        r.superseded = if r.dry_run { 0 } else { superseded(r) };
    }
    all
}

fn read() -> Vec<Record> {
    fs::read(file())
        .ok()
        .and_then(|b| serde_json::from_slice(&b).ok())
        .unwrap_or_default()
}

pub fn append(module: &str, r: &CleanResult) {
    if r.done.is_empty() && r.failed.is_empty() {
        return;
    }
    let _g = LOCK.lock().unwrap();
    let mut all = read();
    let now = crate::util::now_ms();
    all.insert(
        0,
        Record {
            id: format!("{now}-{module}"),
            time: now,
            module: module.into(),
            dry_run: r.dry_run,
            total: r.total,
            failed: r.failed.len(),
            items: r.done.clone(),
            restorable: 0,
            superseded: 0,
        },
    );
    all.truncate(MAX_RECORDS);
    let _ = fs::create_dir_all(crate::settings::data_dir());
    if let Ok(json) = serde_json::to_vec_pretty(&all) {
        let tmp = file().with_extension("json.tmp");
        if fs::write(&tmp, json).is_ok() {
            let _ = fs::rename(tmp, file());
        }
    }
}

pub fn restorable(r: &Record) -> usize {
    // Can't be put back if something already sits at the original path.
    in_trash(r).filter(|d| std::fs::symlink_metadata(&d.path).is_err()).count()
}

/// Items still in the Trash whose original path now holds a newer copy.
pub fn superseded(r: &Record) -> usize {
    in_trash(r).filter(|d| std::fs::symlink_metadata(&d.path).is_ok()).count()
}

fn in_trash(r: &Record) -> impl Iterator<Item = &Done> {
    r.items
        .iter()
        .filter(|d| d.method == "trash" || d.method == "finder")
        .filter(|d| d.trash_path.as_deref().map(|t| std::fs::symlink_metadata(t).is_ok()).unwrap_or(false))
}

pub fn clear() -> Result<(), String> {
    let _g = LOCK.lock().unwrap();
    match fs::remove_file(file()) {
        Ok(()) => Ok(()),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(e) => Err(e.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn counts_items_still_in_the_trash() {
        let d = tempfile::tempdir().unwrap();
        let there = d.path().join("in-trash.bin");
        std::fs::write(&there, b"x").unwrap();
        let done = |p: &str, tp: Option<String>, m: &str| Done { path: p.into(), size: 1, trash_path: tp, method: m.into() };
        let r = Record {
            id: "1".into(), time: 0, module: "leftovers".into(), dry_run: false, total: 3, failed: 0,
            items: vec![
                done("/a", Some(crate::util::path_str(&there)), "trash"),
                done("/b", Some(crate::util::path_str(&d.path().join("gone"))), "trash"),
                done("/c", None, "command"),
                // Still in the Trash, but something new already sits at the original path.
                done(&crate::util::path_str(d.path()), Some(crate::util::path_str(&there)), "trash"),
            ],
            restorable: 0,
            superseded: 0,
        };
        assert_eq!(restorable(&r), 1);
        assert_eq!(superseded(&r), 1);
    }
}
