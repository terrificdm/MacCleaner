//! "Install to Applications": when MacCleaner runs from anywhere else (a dmg,
//! Downloads, a zip), offer to copy itself into /Applications, replacing an
//! older copy after the user confirms. The replaced copy goes to the Trash.

use serde::Serialize;
use std::cmp::Ordering;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum Action {
    /// Already running from an Applications folder, or not an app bundle (dev).
    None,
    /// Nothing installed yet.
    Fresh,
    Upgrade,
    Reinstall,
    Downgrade,
    /// Something else called MacCleaner.app is in /Applications.
    Conflict,
}

/// Compares dotted numeric versions ("0.10.0" > "0.9.0"; "1.0" == "1.0.0").
pub fn compare_versions(a: &str, b: &str) -> Ordering {
    let parse = |v: &str| -> Vec<u64> { v.split('.').map(|x| x.trim().parse().unwrap_or(0)).collect() };
    let (mut x, mut y) = (parse(a), parse(b));
    let n = x.len().max(y.len());
    x.resize(n, 0);
    y.resize(n, 0);
    x.cmp(&y)
}

/// `installed`: (version, has our bundle id) of /Applications/MacCleaner.app.
pub fn plan(running_from_applications: bool, current: &str, installed: Option<(&str, bool)>) -> Action {
    if running_from_applications {
        return Action::None;
    }
    match installed {
        None => Action::Fresh,
        Some((_, false)) => Action::Conflict,
        Some((v, true)) => match compare_versions(current, v) {
            Ordering::Greater => Action::Upgrade,
            Ordering::Equal => Action::Reinstall,
            Ordering::Less => Action::Downgrade,
        },
    }
}

/// macOS runs quarantined apps opened from a dmg or Downloads from a random
/// read-only copy under .../AppTranslocation/...
pub fn is_translocated(bundle: &Path) -> bool {
    bundle.components().any(|c| c.as_os_str() == "AppTranslocation")
}

/// Copy `src` to `dest`. An existing `dest` is first handed to `move_old`
/// (the Trash in production); if that fails nothing else happens.
pub fn install_bundle(
    src: &Path,
    dest: &Path,
    move_old: &dyn Fn(&Path) -> Result<(), String>,
) -> Result<(), String> {
    if std::fs::symlink_metadata(dest).is_ok() {
        move_old(dest)?;
    }
    let out = std::process::Command::new("/usr/bin/ditto")
        .arg(src)
        .arg(dest)
        .output()
        .map_err(|e| e.to_string())?;
    if out.status.success() {
        Ok(())
    } else {
        Err(String::from_utf8_lossy(&out.stderr).trim().to_string())
    }
}

pub fn applications_dir() -> PathBuf {
    PathBuf::from("/Applications")
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Status {
    pub action: Action,
    pub current_version: String,
    pub installed_version: Option<String>,
    pub running_from: Option<String>,
    pub translocated: bool,
}

fn bundle_info(bundle: &Path) -> Option<(String, String)> {
    let v = plist::Value::from_file(bundle.join("Contents/Info.plist")).ok()?;
    let d = v.as_dictionary()?;
    let get = |k: &str| d.get(k).and_then(|x| x.as_string()).map(str::to_string);
    Some((get("CFBundleShortVersionString").unwrap_or_default(), get("CFBundleIdentifier").unwrap_or_default()))
}

fn current_bundle() -> Option<PathBuf> {
    std::env::current_exe().ok().and_then(|e| crate::safety::bundle_of(&e))
}

fn dest_path() -> PathBuf {
    applications_dir().join("MacCleaner.app")
}

pub fn status() -> Status {
    let Some(bundle) = current_bundle() else {
        // `tauri dev`: plain binary, not an app bundle.
        return Status { action: Action::None, current_version: String::new(), installed_version: None, running_from: None, translocated: false };
    };
    let current_version = bundle_info(&bundle).map(|(v, _)| v).unwrap_or_default();
    let home_apps = crate::util::home().join("Applications");
    let parent = bundle.parent().map(Path::to_path_buf).unwrap_or_default();
    let in_apps = parent == applications_dir() || parent == home_apps;
    let installed = bundle_info(&dest_path());
    let action = plan(
        in_apps,
        &current_version,
        installed.as_ref().map(|(v, id)| (v.as_str(), id == crate::APP_ID)),
    );
    Status {
        action,
        current_version,
        installed_version: installed.map(|(v, _)| v),
        running_from: Some(crate::util::path_str(&bundle)),
        translocated: is_translocated(&bundle),
    }
}

fn writable(dir: &Path) -> bool {
    let c = std::ffi::CString::new(dir.to_string_lossy().as_bytes()).unwrap_or_default();
    unsafe { libc::access(c.as_ptr(), libc::W_OK) == 0 }
}

/// Copy with administrator rights (macOS shows its password prompt). Paths
/// are passed as argv and quoted by AppleScript, never spliced into a string.
fn admin_copy(src: &Path, dest: &Path) -> Result<(), String> {
    let script = [
        "on run argv",
        "do shell script (\"/usr/bin/ditto \" & quoted form of (item 1 of argv) & \" \" & quoted form of (item 2 of argv) & \" && /usr/bin/xattr -dr com.apple.quarantine \" & quoted form of (item 2 of argv)) with administrator privileges",
        "end run",
    ];
    let mut cmd = std::process::Command::new("/usr/bin/osascript");
    for l in script {
        cmd.arg("-e").arg(l);
    }
    let out = cmd.arg("--").arg(src).arg(dest).output().map_err(|e| e.to_string())?;
    if out.status.success() {
        Ok(())
    } else {
        Err(String::from_utf8_lossy(&out.stderr).trim().to_string())
    }
}

/// Error codes are translated by the frontend (errors.<code>).
pub fn install() -> Result<PathBuf, String> {
    let st = status();
    if matches!(st.action, Action::None | Action::Conflict) {
        return Err("installNotAllowed".into());
    }
    let src = current_bundle().ok_or("installNotAllowed")?;
    let dest = dest_path();

    // Ask an older copy that is running to quit first.
    if !crate::mac::quit_other_instances(crate::APP_ID, std::time::Duration::from_secs(10)) {
        return Err("oldRunning".into());
    }
    let move_old = |p: &Path| -> Result<(), String> {
        crate::mac::trash_item(p)
            .map(|_| ())
            .or_else(|_| crate::mac::finder_trash(&[p.to_path_buf()]))
            .map_err(|_| "oldNotMoved".to_string())
    };
    if writable(&applications_dir()) {
        install_bundle(&src, &dest, &move_old).map_err(|e| if e == "oldNotMoved" { e } else { "copyFailed".into() })?;
        let _ = std::process::Command::new("/usr/bin/xattr").args(["-dr", "com.apple.quarantine"]).arg(&dest).status();
    } else {
        if std::fs::symlink_metadata(&dest).is_ok() {
            move_old(&dest)?;
        }
        admin_copy(&src, &dest).map_err(|_| "copyFailed".to_string())?;
    }
    Ok(dest)
}

/// Opens `app` once this process has exited (so macOS starts a new instance
/// instead of activating the current one).
pub fn relaunch_after_exit(app: &Path) {
    let _ = std::process::Command::new("/bin/sh")
        .args(["-c", "sleep 1.5; /usr/bin/open \"$1\"", "sh"])
        .arg(app)
        .spawn();
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn compares_dotted_versions_numerically() {
        assert_eq!(compare_versions("0.2.0", "0.1.9"), Ordering::Greater);
        assert_eq!(compare_versions("0.10.0", "0.9.0"), Ordering::Greater);
        assert_eq!(compare_versions("1.0", "1.0.0"), Ordering::Equal);
        assert_eq!(compare_versions("0.1.0", "0.1.1"), Ordering::Less);
    }

    #[test]
    fn plans_the_right_action() {
        assert_eq!(plan(true, "0.2.0", Some(("0.1.0", true))), Action::None);
        assert_eq!(plan(false, "0.2.0", None), Action::Fresh);
        assert_eq!(plan(false, "0.2.0", Some(("0.1.0", true))), Action::Upgrade);
        assert_eq!(plan(false, "0.2.0", Some(("0.2.0", true))), Action::Reinstall);
        assert_eq!(plan(false, "0.1.0", Some(("0.2.0", true))), Action::Downgrade);
        assert_eq!(plan(false, "0.2.0", Some(("3.0", false))), Action::Conflict);
    }

    #[test]
    fn detects_app_translocation() {
        assert!(is_translocated(Path::new("/private/var/folders/ab/xyz/T/AppTranslocation/1234-ABCD/d/MacCleaner.app")));
        assert!(!is_translocated(Path::new("/Volumes/MacCleaner 0.2.0/MacCleaner.app")));
    }

    fn fake_app(dir: &Path, version: &str) -> PathBuf {
        let app = dir.join("MacCleaner.app");
        fs::create_dir_all(app.join("Contents/MacOS")).unwrap();
        fs::write(app.join("Contents/version.txt"), version).unwrap();
        app
    }

    #[test]
    fn fresh_install_copies_the_bundle() {
        let d = tempfile::tempdir().unwrap();
        let src = fake_app(&d.path().join("dmg"), "0.2.0");
        let apps = d.path().join("Applications");
        fs::create_dir_all(&apps).unwrap();
        let dest = apps.join("MacCleaner.app");
        install_bundle(&src, &dest, &|_| panic!("nothing to move")).unwrap();
        assert_eq!(fs::read_to_string(dest.join("Contents/version.txt")).unwrap(), "0.2.0");
        assert!(src.exists(), "source must be left alone");
    }

    #[test]
    fn upgrade_moves_old_copy_away_first() {
        let d = tempfile::tempdir().unwrap();
        let src = fake_app(&d.path().join("dmg"), "0.2.0");
        let apps = d.path().join("Applications");
        let dest = fake_app(&apps, "0.1.0");
        let trash = d.path().join("Trash");
        fs::create_dir_all(&trash).unwrap();
        let t2 = trash.clone();
        install_bundle(&src, &dest, &move |p: &Path| {
            fs::rename(p, t2.join("MacCleaner.app")).map_err(|e| e.to_string())
        })
        .unwrap();
        assert_eq!(fs::read_to_string(dest.join("Contents/version.txt")).unwrap(), "0.2.0");
        assert_eq!(fs::read_to_string(trash.join("MacCleaner.app/Contents/version.txt")).unwrap(), "0.1.0");
    }

    #[test]
    fn failed_move_leaves_old_copy_untouched() {
        let d = tempfile::tempdir().unwrap();
        let src = fake_app(&d.path().join("dmg"), "0.2.0");
        let dest = fake_app(&d.path().join("Applications"), "0.1.0");
        let r = install_bundle(&src, &dest, &|_| Err("denied".into()));
        assert!(r.is_err());
        assert_eq!(fs::read_to_string(dest.join("Contents/version.txt")).unwrap(), "0.1.0");
    }
}
