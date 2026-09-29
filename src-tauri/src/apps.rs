//! Installed apps, their related files, and leftovers of removed apps.

use crate::mac;
use crate::model::Item;
use crate::safety::{self, Ctx};
use crate::util::{self, file_name, home, list_dir, mtime_ms, now_ms, path_size};
use rayon::prelude::*;
use serde::Serialize;
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppInfo {
    pub path: String,
    pub name: String,
    pub bundle_id: String,
    pub version: String,
    pub size: u64,
    /// "appStore" | "brew" | "manual"
    pub source: String,
    pub brew_token: Option<String>,
    pub last_used: Option<i64>,
    pub running: bool,
    /// Why the app cannot be uninstalled here, if so: "systemExtension", "self",
    /// or a safety error code such as "protected" or "whitelisted".
    pub blocked: Option<String>,
    pub needs_admin: bool,
}

struct Plist {
    id: String,
    name: Option<String>,
    version: String,
}

fn read_info(bundle: &Path) -> Option<Plist> {
    let v = plist::Value::from_file(bundle.join("Contents/Info.plist")).ok()?;
    let d = v.as_dictionary()?;
    let s = |k: &str| d.get(k).and_then(|x| x.as_string()).map(|x| x.to_string());
    Some(Plist {
        id: s("CFBundleIdentifier")?,
        name: s("CFBundleName"),
        version: s("CFBundleShortVersionString").unwrap_or_default(),
    })
}

fn is_app(p: &Path) -> bool {
    p.extension().map(|e| e.eq_ignore_ascii_case("app")).unwrap_or(false)
}

fn app_paths() -> Vec<PathBuf> {
    let mut out = Vec::new();
    for root in [PathBuf::from("/Applications"), home().join("Applications")] {
        for p in list_dir(&root) {
            let Ok(m) = fs::symlink_metadata(&p) else { continue };
            if m.file_type().is_symlink() {
                continue; // e.g. Safari, which lives on the sealed system volume
            }
            if is_app(&p) {
                out.push(p);
            } else if m.is_dir() && !util::is_package(&file_name(&p)) {
                // One level of vendor folders such as /Applications/Utilities.
                out.extend(list_dir(&p).into_iter().filter(|c| {
                    is_app(c) && fs::symlink_metadata(c).map(|m| !m.file_type().is_symlink()).unwrap_or(false)
                }));
            }
        }
    }
    out
}

/// App file name -> cask token, for apps installed with Homebrew.
fn brew_apps() -> HashMap<String, String> {
    let mut map = HashMap::new();
    let Some(brew) = util::find_tool("brew") else { return map };
    let Ok(out) = util::run(&brew, &["info", "--cask", "--json=v2", "--installed"]) else { return map };
    let Ok(json) = serde_json::from_str::<serde_json::Value>(&out.stdout) else { return map };
    for cask in json["casks"].as_array().into_iter().flatten() {
        let Some(token) = cask["token"].as_str() else { continue };
        for art in cask["artifacts"].as_array().into_iter().flatten() {
            for app in art["app"].as_array().into_iter().flatten() {
                if let Some(name) = app.as_str() {
                    map.insert(name.to_string(), token.to_string());
                }
            }
        }
    }
    map
}

fn needs_admin(p: &Path) -> bool {
    let Some(parent) = p.parent() else { return true };
    let c = std::ffi::CString::new(parent.to_string_lossy().as_bytes()).unwrap_or_default();
    let parent_ok = unsafe { libc::access(c.as_ptr(), libc::W_OK) } == 0;
    if !parent_ok {
        return true;
    }
    // Moving a directory to another parent also needs write access to it.
    if p.is_dir() {
        let c = std::ffi::CString::new(p.to_string_lossy().as_bytes()).unwrap_or_default();
        return unsafe { libc::access(c.as_ptr(), libc::W_OK) } != 0;
    }
    false
}

pub fn list_apps() -> Vec<AppInfo> {
    let ctx = Ctx::current();
    let paths = app_paths();
    let brew = brew_apps();
    let used = mac::spotlight_dates(&paths, "kMDItemLastUsedDate");
    let running: HashSet<String> = mac::running_apps().into_iter().map(|a| a.bundle_id).collect();
    let mut apps: Vec<AppInfo> = paths
        .into_par_iter()
        .filter_map(|p| {
            let info = read_info(&p)?;
            let fname = file_name(&p);
            let source = if p.join("Contents/_MASReceipt/receipt").exists() {
                "appStore"
            } else if brew.contains_key(&fname) {
                "brew"
            } else {
                "manual"
            };
            let blocked = if p.join("Contents/Library/SystemExtensions").is_dir() {
                Some("systemExtension".to_string())
            } else if info.id == crate::APP_ID {
                Some("self".to_string())
            } else {
                safety::check_deletable(&p, &ctx).err().map(|c| c.to_string())
            };
            Some(AppInfo {
                path: util::path_str(&p),
                name: fname.trim_end_matches(".app").to_string(),
                version: info.version,
                size: path_size(&p),
                source: source.into(),
                brew_token: brew.get(&fname).cloned(),
                last_used: used.get(&p).copied(),
                running: running.contains(&info.id),
                blocked,
                needs_admin: needs_admin(&p),
                bundle_id: info.id,
            })
        })
        .collect();
    apps.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
    apps
}

static ICONS: Mutex<Option<HashMap<String, String>>> = Mutex::new(None);

/// App icon as a data URL, cached for the session.
pub fn icon(path: &str) -> Option<String> {
    if let Some(hit) = ICONS.lock().unwrap().as_ref().and_then(|m| m.get(path).cloned()) {
        return Some(hit);
    }
    let png = mac::icon_png(Path::new(path), 128.0)?;
    let url = format!("data:image/png;base64,{}", base64(&png));
    ICONS.lock().unwrap().get_or_insert_with(HashMap::new).insert(path.into(), url.clone());
    Some(url)
}

fn base64(data: &[u8]) -> String {
    const T: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut s = String::with_capacity(data.len().div_ceil(3) * 4);
    for c in data.chunks(3) {
        let n = (c[0] as u32) << 16 | (*c.get(1).unwrap_or(&0) as u32) << 8 | *c.get(2).unwrap_or(&0) as u32;
        for i in 0..4 {
            if i <= c.len() {
                s.push(T[(n >> (18 - 6 * i) & 63) as usize] as char);
            } else {
                s.push('=');
            }
        }
    }
    s
}

/// Files that belong to an app: named exactly after its bundle id or app
/// name, plus bundle-id-prefixed launch agents/helpers/ByHost prefs and
/// group containers ending in the bundle id.
pub fn related(bundle_id: &str, app_name: &str, app_path: &str) -> Vec<Item> {
    let ctx = Ctx::current();
    let lib = home().join("Library");
    let sys = PathBuf::from("/Library");
    let id = bundle_id;
    let mut names: Vec<String> = vec![app_name.to_string()];
    if let Some(info) = read_info(Path::new(app_path)) {
        if let Some(n) = info.name.filter(|n| n.chars().count() >= 3) {
            names.push(n);
        }
    }
    names.sort();
    names.dedup();

    let mut cands: Vec<(PathBuf, &str)> = Vec::new();
    let exact_id = [
        ("Application Support", ""), ("Caches", ""), ("Containers", ""), ("WebKit", ""),
        ("HTTPStorages", ""), ("HTTPStorages", ".binarycookies"), ("Cookies", ".binarycookies"),
        ("Preferences", ".plist"), ("Saved Application State", ".savedState"),
        ("Logs", ""), ("Application Scripts", ""),
    ];
    for (dir, suffix) in exact_id {
        cands.push((lib.join(dir).join(format!("{id}{suffix}")), dir));
    }
    for n in &names {
        for dir in ["Application Support", "Caches", "Logs"] {
            cands.push((lib.join(dir).join(n), dir));
        }
        cands.push((sys.join("Application Support").join(n), "system"));
        cands.push((sys.join("Logs").join(n), "system"));
    }
    for dir in ["Application Support", "Caches", "Preferences"] {
        let suffix = if dir == "Preferences" { ".plist" } else { "" };
        cands.push((sys.join(dir).join(format!("{id}{suffix}")), "system"));
    }
    let prefixed = [
        (lib.join("Preferences/ByHost"), "Preferences"),
        (lib.join("LaunchAgents"), "LaunchAgents"),
        (sys.join("LaunchAgents"), "system"),
        (sys.join("LaunchDaemons"), "system"),
        (sys.join("PrivilegedHelperTools"), "system"),
    ];
    let idl = id.to_lowercase();
    for (dir, g) in prefixed {
        for p in list_dir(&dir) {
            let n = file_name(&p).to_lowercase();
            if n.starts_with(&format!("{idl}.")) || n == idl {
                cands.push((p, g));
            }
        }
    }
    let groups = if mac::has_full_disk_access() { list_dir(&lib.join("Group Containers")) } else { vec![] };
    for p in groups {
        let n = file_name(&p).to_lowercase();
        if n.ends_with(&format!(".{idl}")) || n == idl || n.contains(&format!(".{idl}.")) {
            cands.push((p, "Group Containers"));
        }
    }

    let fda = mac::has_full_disk_access();
    let mut seen = HashSet::new();
    let mut items: Vec<Item> = cands
        .into_iter()
        .filter(|(_, g)| fda || !is_tcc_guarded(g))
        .filter(|(p, _)| fs::symlink_metadata(p).is_ok() && seen.insert(p.clone()))
        .collect::<Vec<_>>()
        .into_par_iter()
        .filter(|(p, _)| safety::check_deletable(p, &ctx).is_ok())
        .map(|(p, g)| {
            let s = path_size(&p);
            let mut it = Item::path(&p, g, s, true);
            it.needs_admin = needs_admin(&p);
            it
        })
        .collect();
    // Another copy of this app shares these files; keep them unless asked.
    if !other_copies(Path::new(app_path), &copies_of(bundle_id), &home()).is_empty() {
        for it in items.iter_mut() {
            it.selected = false;
            it.note = Some("sharedWithCopy".into());
        }
    }
    crate::inuse::InUse::current().mark(&mut items);
    items.sort_by(|a, b| b.size.cmp(&a.size));
    items
}

/// Other apps' containers: reading them without Full Disk Access makes macOS
/// show a blocking "access data from other apps" prompt.
fn is_tcc_guarded(dir: &str) -> bool {
    matches!(dir, "Containers" | "Group Containers" | "Application Scripts")
}

/// Other installed copies of the same app. Mounted installer images and
/// copies in the Trash don't count.
pub fn other_copies(me: &Path, cands: &[PathBuf], home: &Path) -> Vec<PathBuf> {
    let trash = home.join(".Trash");
    let mut v: Vec<PathBuf> = cands
        .iter()
        .filter(|c| c.as_path() != me && is_app(c))
        .filter(|c| !c.starts_with("/Volumes") && !c.starts_with(&trash))
        // Unpacked installers and updater downloads in temp folders.
        .filter(|c| !c.starts_with("/private/var/folders") && !c.starts_with("/private/tmp") && !c.starts_with("/var/folders"))
        .cloned()
        .collect();
    v.sort();
    v.dedup();
    v
}

/// Every app bundle on this Mac with the given bundle id.
fn copies_of(bundle_id: &str) -> Vec<PathBuf> {
    let q = format!(
        "kMDItemCFBundleIdentifier == \"{}\"",
        bundle_id.replace('\\', "\\\\").replace('"', "\\\"")
    );
    let mut v: Vec<PathBuf> = std::process::Command::new("/usr/bin/mdfind")
        .arg(q)
        .output()
        .map(|o| String::from_utf8_lossy(&o.stdout).lines().map(PathBuf::from).collect())
        .unwrap_or_default();
    v.extend(app_paths().into_iter().filter(|p| read_info(p).map(|i| i.id == bundle_id).unwrap_or(false)));
    v
}

/// Bundle ids of every installed app and its embedded helpers/extensions.
fn installed_ids() -> Vec<String> {
    let out = std::process::Command::new("/usr/bin/mdfind")
        .arg("kMDItemContentType == com.apple.application-bundle")
        .output();
    let mut apps: Vec<PathBuf> = out
        .map(|o| String::from_utf8_lossy(&o.stdout).lines().map(PathBuf::from).collect())
        .unwrap_or_default();
    apps.extend(app_paths());
    apps.sort();
    apps.dedup();
    let mut ids: Vec<String> = apps
        .par_iter()
        .flat_map(|a| {
            let mut v = Vec::new();
            if let Some(i) = read_info(a) {
                v.push(i.id);
            }
            if !a.starts_with("/System") {
                nested_ids(&a.join("Contents"), 0, &mut v);
            }
            v
        })
        .collect();
    for r in mac::running_apps() {
        ids.push(r.bundle_id);
    }
    ids.iter_mut().for_each(|i| *i = i.to_lowercase());
    ids.sort();
    ids.dedup();
    ids
}

fn nested_ids(dir: &Path, depth: usize, out: &mut Vec<String>) {
    if depth > 4 {
        return;
    }
    for p in list_dir(dir) {
        let n = file_name(&p);
        let bundle = [".app", ".appex", ".xpc", ".systemextension", ".plugin", ".bundle"]
            .iter()
            .any(|e| n.ends_with(e));
        if bundle {
            if let Some(i) = read_info(&p) {
                out.push(i.id);
            }
            nested_ids(&p.join("Contents"), depth + 1, out);
        } else if p.is_dir() && !n.ends_with(".framework") && !n.ends_with(".lproj") {
            nested_ids(&p, depth + 1, out);
        }
    }
}

/// Leftovers that apps rebuild on their own are preselected; anything that
/// may hold settings or data is left for the user to decide.
pub fn preselect_leftover(dir: &str) -> bool {
    matches!(dir, "Caches" | "Logs" | "Saved Application State" | "HTTPStorages" | "WebKit" | "Cookies")
}

/// ~/Library folders searched for leftovers.
/// Caches and Logs are left to System Junk, which lists every entry there.
pub const LEFTOVER_DIRS: &[&str] = &[
    "Application Support", "Preferences", "Containers", "Group Containers",
    "Saved Application State", "HTTPStorages", "WebKit", "Cookies",
    "Application Scripts", "LaunchAgents",
];

/// Identifiers of shared system components, not apps.
const SYSTEM_COMPONENT_PREFIXES: &[&str] = &["com.apple.", "is.workflow.", "org.cups.", "org.sparkle-project."];

/// Extract a bundle-id-looking token from a Library entry name.
pub fn id_from_entry(_dir: &str, name: &str) -> Option<String> {
    let mut n = name.to_string();
    for suf in [".plist", ".savedState", ".binarycookies"] {
        if let Some(s) = n.strip_suffix(suf) {
            n = s.to_string();
        }
    }
    // Sandboxed apps use "TEAMID1234.<id>" and "group.<id>" in several
    // Library folders, not only Group Containers.
    if let Some((head, rest)) = n.split_once('.') {
        if head.len() == 10 && head.chars().all(|c| c.is_ascii_uppercase() || c.is_ascii_digit()) {
            n = rest.to_string();
        }
    }
    if let Some(rest) = n.strip_prefix("group.") {
        n = rest.to_string();
    }
    let parts: Vec<&str> = n.split('.').collect();
    let tld_ok = parts
        .first()
        .map(|t| (2..=6).contains(&t.len()) && t.chars().all(|c| c.is_ascii_lowercase()))
        .unwrap_or(false);
    let valid = parts.len() >= 3
        && tld_ok
        && parts.iter().all(|p| !p.is_empty() && p.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_'));
    let lower = n.to_lowercase();
    if !valid
        || SYSTEM_COMPONENT_PREFIXES.iter().any(|p| lower.starts_with(p))
        || lower.starts_with(&crate::APP_ID.to_lowercase())
    {
        return None;
    }
    Some(n)
}

pub fn is_installed(cand: &str, installed: &[String]) -> bool {
    let c = cand.to_lowercase();
    installed.iter().any(|i| {
        c == *i || c.starts_with(&format!("{i}.")) || i.starts_with(&format!("{c}."))
    })
}

fn vendor(id: &str) -> String {
    id.to_lowercase().split('.').take(2).collect::<Vec<_>>().join(".")
}

/// LaunchAgent whose program still exists is in use, not a leftover.
fn agent_program_exists(p: &Path) -> bool {
    let Ok(v) = plist::Value::from_file(p) else { return true };
    let Some(d) = v.as_dictionary() else { return true };
    let prog = d
        .get("Program")
        .and_then(|x| x.as_string())
        .or_else(|| d.get("ProgramArguments")?.as_array()?.first()?.as_string());
    match prog {
        Some(pr) => Path::new(pr).exists() || !pr.starts_with('/'),
        None => true,
    }
}

pub fn leftovers() -> Vec<Item> {
    let ctx = Ctx::current();
    let installed = installed_ids();
    let vendors: HashSet<String> = installed.iter().map(|i| vendor(i)).collect();
    let lib = home().join("Library");
    let fda = mac::has_full_disk_access();
    let dirs = LEFTOVER_DIRS.iter().copied();
    let mut cands: Vec<(PathBuf, String, &str)> = Vec::new();
    for d in dirs {
        if !fda && is_tcc_guarded(d) {
            continue;
        }
        for p in list_dir(&lib.join(d)) {
            if fs::symlink_metadata(&p).map(|m| util::is_special(&m)).unwrap_or(true) {
                continue; // sockets, pipes: in use by something, never "leftovers"
            }
            let Some(id) = id_from_entry(d, &file_name(&p)) else { continue };
            if is_installed(&id, &installed) || ctx.is_excluded(&p) {
                continue;
            }
            if d == "LaunchAgents" && agent_program_exists(&p) {
                continue;
            }
            cands.push((p, id, d));
        }
    }
    let recent = now_ms() - 30 * 86_400_000;
    let mut items: Vec<Item> = cands
        .into_par_iter()
        .filter(|(p, _, _)| safety::check_deletable(p, &ctx).is_ok())
        .map(|(p, id, d)| {
            let size = path_size(&p);
            let mut it = Item::path(&p, &id, size, preselect_leftover(d));
            it.note = Some(d.to_string());
            it.modified = fs::symlink_metadata(&p).ok().and_then(|m| mtime_ms(&m));
            if vendors.contains(&vendor(&id)) {
                it.selected = false;
                it.note = Some("sameVendor".into());
            } else if it.modified.map(|t| t > recent).unwrap_or(false) {
                it.selected = false;
                it.note = Some("recentlyUsed".into());
            }
            it
        })
        .collect();
    crate::inuse::InUse::current().mark(&mut items);
    items.sort_by(|a, b| b.size.cmp(&a.size));
    items
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extracts_ids() {
        assert_eq!(id_from_entry("Preferences", "com.foo.Bar.plist").as_deref(), Some("com.foo.Bar"));
        assert_eq!(id_from_entry("Saved Application State", "com.foo.Bar.savedState").as_deref(), Some("com.foo.Bar"));
        assert_eq!(id_from_entry("Group Containers", "UBF8T346G9.com.microsoft.teams").as_deref(), Some("com.microsoft.teams"));
        assert_eq!(id_from_entry("Group Containers", "group.com.foo.shared").as_deref(), Some("com.foo.shared"));
        assert_eq!(id_from_entry("Group Containers", "UBF8T346G9.Office"), None);
        assert_eq!(id_from_entry("Application Support", "Google"), None);
        assert_eq!(id_from_entry("Caches", "com.apple.Safari"), None);
        assert_eq!(id_from_entry("Caches", "My App 2.0"), None);
        assert_eq!(id_from_entry("Caches", "com.example"), None);
    }

    #[test]
    fn apple_and_system_components_are_not_leftovers() {
        for (dir, name) in [
            ("Application Scripts", "group.com.apple.notes"),
            ("Application Scripts", "group.is.workflow.shortcuts"),
            ("Group Containers", "group.is.workflow.my.app"),
            ("Application Scripts", "UBF8T346G9.com.apple.x"),
            ("Preferences", "org.cups.PrintingPrefs.plist"),
            ("Preferences", "org.sparkle-project.Sparkle.Autoupdate.plist"),
            ("Caches", "com.apple.nsurlsessiond"),
        ] {
            assert_eq!(id_from_entry(dir, name), None, "{dir}/{name}");
        }
        assert_eq!(id_from_entry("Application Scripts", "group.com.foo.app").as_deref(), Some("com.foo.app"));
    }

    #[test]
    fn leftovers_skip_folders_system_junk_already_covers() {
        // System Junk lists every entry of ~/Library/Caches and ~/Library/Logs;
        // scanning them here too would list the same folder twice.
        assert!(!LEFTOVER_DIRS.contains(&"Caches"));
        assert!(!LEFTOVER_DIRS.contains(&"Logs"));
        assert!(LEFTOVER_DIRS.contains(&"Preferences"));
    }

    #[test]
    fn only_regenerable_leftovers_are_preselected() {
        for d in ["Caches", "Logs", "Saved Application State", "HTTPStorages", "WebKit", "Cookies"] {
            assert!(preselect_leftover(d), "{d}");
        }
        for d in ["Application Support", "Containers", "Group Containers", "Preferences", "LaunchAgents", "Application Scripts"] {
            assert!(!preselect_leftover(d), "{d}");
        }
    }

    #[test]
    fn installed_matching_covers_helpers() {
        let inst = vec!["com.hnc.discord".to_string()];
        assert!(is_installed("com.hnc.Discord", &inst));
        assert!(is_installed("com.hnc.Discord.helper", &inst));
        assert!(!is_installed("com.hnc.Discordia", &inst));
        let inst2 = vec!["com.foo.app.helper".to_string()];
        assert!(is_installed("com.foo.app", &inst2));
    }

    #[test]
    fn detects_other_installed_copies_only() {
        let home = Path::new("/Users/x");
        let me = Path::new("/Applications/Foo.app");
        let cands = vec![
            PathBuf::from("/Applications/Foo.app"),
            PathBuf::from("/Volumes/Foo Installer/Foo.app"),
            PathBuf::from("/Users/x/.Trash/Foo.app"),
            PathBuf::from("/private/var/folders/h2/abc/T/foo-download/Foo.app"),
            PathBuf::from("/Users/x/Applications/Foo.app"),
        ];
        assert_eq!(other_copies(me, &cands, home), vec![PathBuf::from("/Users/x/Applications/Foo.app")]);
        assert!(other_copies(me, &cands[..4], home).is_empty());
    }

    #[test]
    fn base64_matches_reference() {
        assert_eq!(base64(b""), "");
        assert_eq!(base64(b"f"), "Zg==");
        assert_eq!(base64(b"fo"), "Zm8=");
        assert_eq!(base64(b"foo"), "Zm9v");
        assert_eq!(base64(b"foobar"), "Zm9vYmFy");
    }
}
