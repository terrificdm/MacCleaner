use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use std::sync::Mutex;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Settings {
    /// "system" | "zh-CN" | "en"
    pub language: String,
    /// "system" | "light" | "dark"
    pub theme: String,
    /// When true nothing is ever moved or deleted; enforced in Rust.
    pub dry_run: bool,
    /// Paths MacCleaner never moves (module scans skip them too). Starts
    /// with `DEFAULT_WHITELIST`; the user may remove or add entries.
    /// Command items and Empty Trash are not path-based and not covered.
    pub whitelist: Vec<String>,
    /// Set once the default whitelist has been merged into `whitelist`.
    /// Field-level default: files written before this field existed must
    /// read as `false` (the struct-level default would say `true`).
    #[serde(default)]
    pub default_whitelist_applied: bool,
    pub large_min_mb: u64,
    pub old_min_mb: u64,
    pub old_days: u64,
    pub app_unused_days: u64,
    pub node_modules_days: u64,
    pub node_modules_roots: Vec<String>,
    pub onboarding_done: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Settings {
            language: "system".into(),
            theme: "system".into(),
            dry_run: true,
            whitelist: DEFAULT_WHITELIST.iter().map(|s| s.to_string()).collect(),
            default_whitelist_applied: true,
            large_min_mb: 500,
            old_min_mb: 100,
            old_days: 365,
            app_unused_days: 90,
            node_modules_days: 90,
            node_modules_roots: vec!["~".into()],
            onboarding_done: false,
        }
    }
}

/// Personal data that is protected by default. Unlike the hard-coded list in
/// `safety.rs`, these can be removed in Settings.
pub const DEFAULT_WHITELIST: &[&str] = &[
    "~/Documents",
    "~/.ssh",
    "~/.gnupg",
    "~/Library/Mobile Documents",
    "~/Library/CloudStorage",
    "~/Library/Mail",
    "~/Library/Messages",
    "~/Library/Safari",
    "~/Library/Accounts",
    "~/Library/Calendars",
    "~/Library/Photos",
    "~/Library/Application Support/AddressBook",
    "~/Library/Application Support/CallHistoryDB",
    "~/Library/Application Support/MobileSync",
];

/// Settings files written before the default whitelist existed get it merged
/// in once, so nothing loses protection on upgrade.
fn migrate(mut s: Settings) -> (Settings, bool) {
    if s.default_whitelist_applied {
        return (s, false);
    }
    for d in DEFAULT_WHITELIST {
        if !s.whitelist.iter().any(|w| w == d) {
            s.whitelist.push(d.to_string());
        }
    }
    s.default_whitelist_applied = true;
    (s, true)
}

pub fn data_dir() -> PathBuf {
    // Tests must never read, rewrite or rename the user's real settings.
    if cfg!(test) {
        return std::env::temp_dir().join(format!("maccleaner-test-{}", std::process::id()));
    }
    crate::util::home().join("Library/Application Support/MacCleaner")
}

fn settings_file() -> PathBuf {
    data_dir().join("settings.json")
}

static CURRENT: Mutex<Option<Settings>> = Mutex::new(None);

pub fn get() -> Settings {
    let mut cur = CURRENT.lock().unwrap();
    if let Some(s) = cur.as_ref() {
        return s.clone();
    }
    let file = fs::read(settings_file()).ok();
    let loaded = match file.as_deref().map(serde_json::from_slice::<Settings>) {
        Some(Ok(s)) => s,
        // A missing file gets defaults; an unreadable one too, but it is kept
        // aside so the user's list is not silently overwritten.
        Some(Err(_)) => {
            let _ = fs::rename(settings_file(), settings_file().with_extension("json.bad"));
            Settings::default()
        }
        None => Settings::default(),
    };
    let (s, changed) = migrate(loaded);
    *cur = Some(s.clone());
    drop(cur);
    if changed {
        let _ = save(s.clone());
    }
    s
}

pub fn save(s: Settings) -> Result<Settings, String> {
    let mut s = s;
    s.whitelist = s
        .whitelist
        .into_iter()
        .map(|p| p.trim().to_string())
        .filter(|p| !p.is_empty())
        .collect();
    s.whitelist.sort();
    s.whitelist.dedup();
    fs::create_dir_all(data_dir()).map_err(|e| e.to_string())?;
    s.default_whitelist_applied = true;
    let json = serde_json::to_vec_pretty(&s).map_err(|e| e.to_string())?;
    let tmp = settings_file().with_extension("json.tmp");
    fs::write(&tmp, json).map_err(|e| e.to_string())?;
    fs::rename(&tmp, settings_file()).map_err(|e| e.to_string())?;
    *CURRENT.lock().unwrap() = Some(s.clone());
    Ok(s)
}

pub fn dry_run() -> bool {
    get().dry_run
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn old_settings_get_default_whitelist_once() {
        // A settings file from before the default whitelist existed.
        let old: Settings = serde_json::from_str(r#"{"whitelist": ["~/Projects/keep"], "dryRun": true}"#).unwrap();
        assert!(!old.default_whitelist_applied);
        let (s, changed) = migrate(old);
        assert!(changed);
        assert!(s.whitelist.contains(&"~/Documents".to_string()));
        assert!(s.whitelist.contains(&"~/Projects/keep".to_string()));
        // After the user removes a default, it must not come back.
        let mut s2 = s.clone();
        s2.whitelist.retain(|w| w != "~/Documents");
        let (s3, changed) = migrate(s2);
        assert!(!changed);
        assert!(!s3.whitelist.contains(&"~/Documents".to_string()));
    }

    #[test]
    fn tests_never_use_the_real_settings_folder() {
        let real = crate::util::home().join("Library/Application Support/MacCleaner");
        assert!(!data_dir().starts_with(&real), "{}", data_dir().display());
    }

    #[test]
    fn fresh_defaults_include_whitelist() {
        let s = Settings::default();
        assert!(s.default_whitelist_applied);
        assert_eq!(s.whitelist.len(), DEFAULT_WHITELIST.len());
    }
}
