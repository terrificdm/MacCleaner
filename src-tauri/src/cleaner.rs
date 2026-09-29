use crate::mac;
use crate::model::{CleanResult, CleanTarget, Done, Failed};
use crate::safety::{self, Ctx};
use crate::util::{self, path_size};
use serde::Deserialize;
use std::path::{Path, PathBuf};
use std::time::Duration;

fn fail(path: &str, code: &str, detail: Option<String>) -> Failed {
    Failed { path: path.into(), code: code.into(), detail }
}

/// Size changed noticeably since the scan (10% and at least 1 MB).
fn changed(expected: u64, now: u64) -> bool {
    let diff = expected.abs_diff(now);
    diff > (expected / 10).max(1 << 20)
}

/// Top-level folder name of a per-app cache/state location, used to refuse
/// touching it while the owning app runs.
fn app_folder(p: &Path) -> Option<String> {
    let lib = util::home().join("Library");
    for d in ["Caches", "Saved Application State", "HTTPStorages", "WebKit"] {
        if let Ok(rest) = p.strip_prefix(lib.join(d)) {
            return Some(rest.components().next()?.as_os_str().to_string_lossy().into_owned());
        }
    }
    None
}

pub fn clean(targets: Vec<CleanTarget>, dry: bool) -> CleanResult {
    clean_except(targets, dry, &[])
}

/// Like `clean`, but `exempt` paths skip the in-use check (the app bundle
/// during uninstall, which was just asked to quit).
fn clean_except(targets: Vec<CleanTarget>, dry: bool, exempt: &[PathBuf]) -> CleanResult {
    let ctx = Ctx::current();
    let running = mac::running_matcher();
    let in_use = crate::inuse::InUse::current();
    let mut res = CleanResult { dry_run: dry, ..Default::default() };
    let mut finder: Vec<(PathBuf, u64)> = Vec::new();
    let mut seen = std::collections::HashSet::new();

    for t in targets {
        if !seen.insert(t.path.clone()) {
            continue; // the same path listed twice in one batch
        }
        if t.kind == "command" {
            run_command(&t, dry, &mut res);
            continue;
        }
        let p = PathBuf::from(&t.path);
        if let Err(code) = safety::check_deletable(&p, &ctx) {
            res.failed.push(fail(&t.path, code, None));
            continue;
        }
        if std::fs::symlink_metadata(&p).map(|m| util::is_locked(&m)).unwrap_or(false) {
            // Respect Finder's "Locked"; don't ask Finder to override it.
            res.failed.push(fail(&t.path, "locked", None));
            continue;
        }
        if !exempt.contains(&p) && in_use.contains_under(&p) {
            res.failed.push(fail(&t.path, "inUse", None));
            continue;
        }
        if let Some(folder) = app_folder(&p) {
            if running(&folder) {
                res.failed.push(fail(&t.path, "appRunning", Some(folder)));
                continue;
            }
        }
        let now = path_size(&p);
        if t.size > 0 && changed(t.size, now) {
            res.failed.push(fail(&t.path, "changed", Some(format!("{} -> {}", t.size, now))));
            continue;
        }
        if dry {
            res.total += now;
            res.done.push(Done { path: t.path, size: now, trash_path: None, method: "dryRun".into() });
            continue;
        }
        match mac::trash_item(&p) {
            Ok(tp) => {
                res.total += now;
                res.done.push(Done { path: t.path, size: now, trash_path: tp, method: "trash".into() });
            }
            // Usually a permission problem (root-owned); Finder can ask for a password.
            Err(_) => finder.push((p, now)),
        }
    }

    if !finder.is_empty() {
        let paths: Vec<PathBuf> = finder.iter().map(|(p, _)| p.clone()).collect();
        let r = mac::finder_trash(&paths);
        for (p, size) in finder {
            let s = util::path_str(&p);
            if std::fs::symlink_metadata(&p).is_err() {
                res.total += size;
                res.done.push(Done { path: s, size, trash_path: None, method: "finder".into() });
            } else {
                let detail = r.as_ref().err().map(|e| util::tail(e, 200));
                res.failed.push(fail(&s, "finderFailed", detail));
            }
        }
    }
    res
}

fn run_command(t: &CleanTarget, dry: bool, res: &mut CleanResult) {
    let cmd = t.command.clone().unwrap_or_default();
    if dry {
        res.total += t.size;
        res.done.push(Done { path: t.path.clone(), size: t.size, trash_path: None, method: "dryRun".into() });
        return;
    }
    let outcome = match cmd.as_str() {
        "brewCleanup" => util::find_tool("brew")
            .ok_or_else(|| "brew not found".to_string())
            .and_then(|b| util::run(&b, &["cleanup", "--prune=all"]).map_err(|e| e.to_string())),
        "dockerPrune" => util::find_tool("docker")
            .ok_or_else(|| "docker not found".to_string())
            .and_then(|d| util::run(&d, &["system", "prune", "-f"]).map_err(|e| e.to_string())),
        _ => Err(format!("unknown command {cmd}")),
    };
    match outcome {
        Ok(o) if o.ok => {
            res.total += t.size;
            res.done.push(Done { path: t.path.clone(), size: t.size, trash_path: None, method: "command".into() });
        }
        Ok(o) => res.failed.push(fail(&t.path, "commandFailed", Some(util::tail(&o.stderr, 300)))),
        Err(e) => res.failed.push(fail(&t.path, "commandFailed", Some(e))),
    }
}

pub fn empty_trash(dry: bool) -> CleanResult {
    let trash = util::home().join(".Trash");
    let size = path_size(&trash);
    let mut res = CleanResult { dry_run: dry, ..Default::default() };
    let path = util::path_str(&trash);
    if dry {
        res.total = size;
        res.done.push(Done { path, size, trash_path: None, method: "dryRun".into() });
        return res;
    }
    match mac::finder_empty_trash() {
        Ok(()) => {
            res.total = size;
            res.done.push(Done { path, size, trash_path: None, method: "emptyTrash".into() });
        }
        Err(e) => res.failed.push(fail(&path, "finderFailed", Some(util::tail(&e, 200)))),
    }
    res
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RestoreItem {
    pub path: String,
    pub trash_path: Option<String>,
}

/// Moves items MacCleaner put in the Trash back to where they were. Only
/// sources inside `trash` are used; an existing file at the original path
/// is never overwritten.
pub fn restore_from(items: Vec<RestoreItem>, trash: &Path) -> CleanResult {
    let mut res = CleanResult::default();
    for it in items {
        let Some(src) = it.trash_path.as_deref().map(PathBuf::from) else {
            res.failed.push(fail(&it.path, "restoreManual", None));
            continue;
        };
        let dst = PathBuf::from(&it.path);
        if !src.starts_with(trash) || std::fs::symlink_metadata(&src).is_err() {
            res.failed.push(fail(&it.path, "restoreNotInTrash", None));
            continue;
        }
        if std::fs::symlink_metadata(&dst).is_ok() {
            // A newer copy was created there meanwhile; keep it, leave the old one in the Trash.
            res.skipped.push(fail(&it.path, "restoreExists", None));
            continue;
        }
        let moved = dst
            .parent()
            .map(std::fs::create_dir_all)
            .unwrap_or(Ok(()))
            .and_then(|_| std::fs::rename(&src, &dst));
        match moved {
            Ok(()) => {
                let size = path_size(&dst);
                res.total += size;
                res.done.push(Done { path: it.path, size, trash_path: None, method: "restored".into() });
            }
            Err(e) => res.failed.push(fail(&it.path, "restoreFailed", Some(e.to_string()))),
        }
    }
    res
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UninstallRequest {
    pub app_path: String,
    pub bundle_id: String,
    pub app_size: u64,
    pub brew_token: Option<String>,
    pub related: Vec<CleanTarget>,
}

pub fn uninstall(req: UninstallRequest, dry: bool) -> Result<CleanResult, String> {
    let ctx = Ctx::current();
    let app = PathBuf::from(&req.app_path);
    // Validate before quitting anything.
    safety::check_deletable(&app, &ctx).map_err(|c| c.to_string())?;

    if !dry && !req.bundle_id.is_empty() && mac::is_running(&req.bundle_id) {
        if !mac::quit_app(&req.bundle_id, Duration::from_secs(12)) {
            return Err("appRefusedQuit".into());
        }
        // Give the app a moment to flush its files after quitting.
        std::thread::sleep(Duration::from_millis(800));
    }

    let mut res = CleanResult { dry_run: dry, ..Default::default() };
    let mut targets = vec![CleanTarget {
        path: req.app_path.clone(),
        size: req.app_size,
        kind: "dir".into(),
        command: None,
    }];
    targets.extend(req.related);

    // App and related files go through the normal safety checks and into
    // the Trash, so the whitelist applies and everything can be put back.
    let existing: Vec<CleanTarget> = targets
        .into_iter()
        .filter(|t| std::fs::symlink_metadata(&t.path).is_ok())
        .collect();
    let r = clean_except(existing, dry, &[app.clone()]);
    let app_removed = r.done.iter().any(|d| d.path == req.app_path);
    res.total += r.total;
    res.done.extend(r.done);
    res.failed.extend(r.failed);

    // Then let Homebrew forget the cask. No --zap: zap deletes files
    // permanently and ignores MacCleaner's whitelist.
    if let Some(token) = req.brew_token.filter(|t| !t.is_empty()) {
        let label = format!("brew uninstall --cask {token}");
        if !app_removed {
            // Keep Homebrew's record if the app itself stayed.
        } else if dry {
            res.done.push(Done { path: label, size: 0, trash_path: None, method: "dryRun".into() });
        } else if let Some(brew) = util::find_tool("brew") {
            match util::run(&brew, &["uninstall", "--cask", &token]) {
                Ok(o) if o.ok => res.done.push(Done { path: label, size: 0, trash_path: None, method: "command".into() }),
                Ok(o) => res.failed.push(fail(&token, "brewFailed", Some(util::tail(&o.stderr, 300)))),
                Err(e) => res.failed.push(fail(&token, "brewFailed", Some(e.to_string()))),
            }
        }
    }
    Ok(res)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn change_tolerance() {
        assert!(!changed(100 << 20, 105 << 20));
        assert!(changed(100 << 20, 130 << 20));
        assert!(!changed(1000, 500_000)); // under 1 MB difference
        assert!(changed(0, 5 << 20));
    }

    #[test]
    fn dry_run_moves_nothing() {
        let d = tempfile::tempdir().unwrap();
        // Tests run with the real home as context, so use a file under a
        // temp dir inside the allowed TMPDIR root.
        let f = d.path().join("junk.bin");
        std::fs::write(&f, vec![0u8; 4096]).unwrap();
        let f = std::fs::canonicalize(&f).unwrap();
        let r = clean(
            vec![CleanTarget { path: util::path_str(&f), size: 0, kind: "file".into(), command: None }],
            true,
        );
        assert!(f.exists(), "dry run must not touch the file");
        assert!(r.dry_run);
        assert_eq!(r.done.len() + r.failed.len(), 1);
    }

    #[test]
    fn refuses_files_that_are_open() {
        let d = tempfile::tempdir().unwrap();
        let f = std::fs::canonicalize(d.path()).unwrap().join("open.bin");
        std::fs::write(&f, vec![0u8; 4096]).unwrap();
        let _h = std::fs::File::open(&f).unwrap();
        let r = clean(
            vec![CleanTarget { path: util::path_str(&f), size: 0, kind: "file".into(), command: None }],
            true,
        );
        assert!(r.done.is_empty(), "{:?}", r.done);
        assert_eq!(r.failed[0].code, "inUse");
    }

    #[test]
    fn brew_uninstall_never_zaps_and_trashes_the_app_first() {
        let d = tempfile::tempdir().unwrap();
        let app = std::fs::canonicalize(d.path()).unwrap().join("Fake.app");
        std::fs::create_dir_all(app.join("Contents")).unwrap();
        let req = UninstallRequest {
            app_path: util::path_str(&app),
            bundle_id: "com.example.fake-not-running".into(),
            app_size: 0,
            brew_token: Some("fake".into()),
            related: vec![],
        };
        let r = uninstall(req, true).unwrap();
        let paths: Vec<&str> = r.done.iter().map(|d| d.path.as_str()).collect();
        assert!(paths.iter().all(|p| !p.contains("--zap")), "{paths:?}");
        assert_eq!(paths, vec![util::path_str(&app).as_str(), "brew uninstall --cask fake"]);
        assert!(app.exists());
    }

    #[test]
    fn restores_items_from_the_trash_only() {
        let d = tempfile::tempdir().unwrap();
        let root = std::fs::canonicalize(d.path()).unwrap();
        let trash = root.join("Trash");
        std::fs::create_dir_all(&trash).unwrap();
        std::fs::write(trash.join("a.plist"), b"a").unwrap();
        std::fs::create_dir_all(trash.join("group.x")).unwrap();
        std::fs::write(root.join("elsewhere.txt"), b"x").unwrap();
        std::fs::create_dir_all(root.join("Lib/Prefs")).unwrap();
        std::fs::write(root.join("Lib/Prefs/taken.plist"), b"new").unwrap();
        std::fs::write(trash.join("taken.plist"), b"old").unwrap();
        let s = |p: PathBuf| util::path_str(&p);
        let items = vec![
            RestoreItem { path: s(root.join("Lib/Prefs/a.plist")), trash_path: Some(s(trash.join("a.plist"))) },
            RestoreItem { path: s(root.join("Lib/Scripts/group.x")), trash_path: Some(s(trash.join("group.x"))) },
            RestoreItem { path: s(root.join("Lib/Prefs/taken.plist")), trash_path: Some(s(trash.join("taken.plist"))) },
            RestoreItem { path: s(root.join("Lib/x.txt")), trash_path: Some(s(root.join("elsewhere.txt"))) },
            RestoreItem { path: s(root.join("Lib/y")), trash_path: None },
        ];
        let r = restore_from(items, &trash);
        assert_eq!(r.done.len(), 2, "{:?}", r.failed);
        assert!(root.join("Lib/Prefs/a.plist").exists());
        assert!(root.join("Lib/Scripts/group.x").is_dir());
        let codes: Vec<&str> = r.failed.iter().map(|f| f.code.as_str()).collect();
        assert_eq!(codes, vec!["restoreNotInTrash", "restoreManual"]);
        // A newer copy at the original path is expected, not a failure.
        let skipped: Vec<&str> = r.skipped.iter().map(|f| f.code.as_str()).collect();
        assert_eq!(skipped, vec!["restoreExists"]);
        assert_eq!(std::fs::read(root.join("Lib/Prefs/taken.plist")).unwrap(), b"new");
        assert!(root.join("elsewhere.txt").exists());
    }

    #[test]
    fn refuses_locked_files_without_asking_finder() {
        let d = tempfile::tempdir().unwrap();
        let f = std::fs::canonicalize(d.path()).unwrap().join("locked.zip");
        std::fs::write(&f, b"x").unwrap();
        set_locked(&f, true);
        let r = clean(
            vec![CleanTarget { path: util::path_str(&f), size: 0, kind: "file".into(), command: None }],
            false,
        );
        set_locked(&f, false);
        assert!(f.exists());
        assert_eq!(r.failed[0].code, "locked");
    }

    fn set_locked(p: &Path, on: bool) {
        let c = std::ffi::CString::new(p.to_string_lossy().as_bytes()).unwrap();
        assert_eq!(unsafe { libc::chflags(c.as_ptr(), if on { libc::UF_IMMUTABLE as _ } else { 0 }) }, 0);
    }

    #[test]
    fn duplicate_paths_in_one_batch_are_handled_once() {
        let d = tempfile::tempdir().unwrap();
        let f = std::fs::canonicalize(d.path()).unwrap().join("dup.bin");
        std::fs::write(&f, vec![0u8; 4096]).unwrap();
        let t = || CleanTarget { path: util::path_str(&f), size: 0, kind: "file".into(), command: None };
        let r = clean(vec![t(), t()], true);
        assert_eq!(r.done.len(), 1);
        assert!(r.failed.is_empty(), "{:?}", r.failed);
    }

    #[test]
    fn rejects_protected_even_when_not_dry() {
        let r = clean(
            vec![CleanTarget { path: "/System/Library".into(), size: 0, kind: "dir".into(), command: None }],
            false,
        );
        assert!(r.done.is_empty());
        assert_eq!(r.failed[0].code, "protected");
        assert!(Path::new("/System/Library").exists());
    }

    #[test]
    fn real_trash_of_temp_file() {
        let d = tempfile::tempdir().unwrap();
        let f = std::fs::canonicalize(d.path()).unwrap().join("maccleaner-test.bin");
        std::fs::write(&f, vec![7u8; 8192]).unwrap();
        let r = clean(
            vec![CleanTarget { path: util::path_str(&f), size: 8192, kind: "file".into(), command: None }],
            false,
        );
        assert_eq!(r.failed.len(), 0, "{:?}", r.failed);
        assert!(!f.exists());
        // Remove our test file from the Trash again.
        if let Some(tp) = &r.done[0].trash_path {
            let _ = std::fs::remove_file(tp);
        }
    }
}
