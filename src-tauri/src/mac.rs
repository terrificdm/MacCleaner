//! macOS-specific operations: Trash, running apps, icons, Finder, Spotlight.

use objc2::rc::Retained;
use objc2::AnyThread;
use objc2_app_kit::{NSBitmapImageFileType, NSBitmapImageRep, NSRunningApplication, NSWorkspace};
use objc2_foundation::{NSDictionary, NSFileManager, NSPoint, NSRect, NSSize, NSString, NSURL};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{Duration, Instant};

/// Move one item to the user's Trash. Returns the path inside the Trash.
pub fn trash_item(p: &Path) -> Result<Option<String>, String> {
    let fm = NSFileManager::defaultManager();
    let url = NSURL::fileURLWithPath(&NSString::from_str(&p.to_string_lossy()));
    let mut out: Option<Retained<NSURL>> = None;
    fm.trashItemAtURL_resultingItemURL_error(&url, Some(&mut out))
        .map_err(|e| e.localizedDescription().to_string())?;
    Ok(out.and_then(|u| u.path()).map(|s| s.to_string()))
}

/// Ask Finder to move items to the Trash. Finder shows the system admin
/// password prompt for root-owned files. Paths are passed as argv, never
/// interpolated into the script.
pub fn finder_trash(paths: &[PathBuf]) -> Result<(), String> {
    if paths.is_empty() {
        return Ok(());
    }
    let script = [
        "on run argv",
        "set l to {}",
        "repeat with p in argv",
        "set end of l to ((POSIX file (p as text)) as alias)",
        "end repeat",
        "tell application \"Finder\" to delete l",
        "end run",
    ];
    let mut cmd = Command::new("/usr/bin/osascript");
    for line in script {
        cmd.arg("-e").arg(line);
    }
    cmd.arg("--");
    for p in paths {
        cmd.arg(p);
    }
    let out = cmd.output().map_err(|e| e.to_string())?;
    if out.status.success() {
        Ok(())
    } else {
        Err(String::from_utf8_lossy(&out.stderr).trim().to_string())
    }
}

pub fn finder_empty_trash() -> Result<(), String> {
    let out = Command::new("/usr/bin/osascript")
        .args(["-e", "tell application \"Finder\" to empty trash"])
        .output()
        .map_err(|e| e.to_string())?;
    if out.status.success() {
        Ok(())
    } else {
        Err(String::from_utf8_lossy(&out.stderr).trim().to_string())
    }
}

#[derive(Clone, Debug)]
pub struct RunningApp {
    pub bundle_id: String,
    pub name: String,
}

pub fn running_apps() -> Vec<RunningApp> {
    let ws = NSWorkspace::sharedWorkspace();
    let apps = ws.runningApplications();
    apps.iter()
        .filter_map(|a| {
            let id = a.bundleIdentifier()?.to_string();
            let name = a.localizedName().map(|n| n.to_string()).unwrap_or_default();
            Some(RunningApp { bundle_id: id, name })
        })
        .collect()
}

/// Returns a predicate telling whether a Library folder name (e.g. a cache
/// folder) probably belongs to a running app. Errs on the side of "running":
/// matches the bundle id, the app name as a prefix, or the vendor/product
/// components of the bundle id.
pub fn running_matcher() -> impl Fn(&str) -> bool {
    let apps = running_apps();
    let mut ids = Vec::new();
    let mut prefixes = Vec::new();
    let mut words = Vec::new();
    for a in apps {
        let id = a.bundle_id.to_lowercase();
        for part in id.split('.').skip(1) {
            if part.len() >= 4 && !matches!(part, "apple" | "app" | "mac" | "desktop" | "electron" | "helper") {
                words.push(part.to_string());
            }
        }
        ids.push(id);
        let n = a.name.to_lowercase();
        if n.chars().count() >= 4 {
            prefixes.push(n);
        }
    }
    move |folder: &str| {
        let f = folder.to_lowercase();
        let f = f.trim_end_matches(".savedstate");
        ids.iter().any(|i| f == i || f.starts_with(&format!("{i}.")))
            || prefixes.iter().any(|p| f.starts_with(p.as_str()))
            || words.iter().any(|w| f.starts_with(w.as_str()))
    }
}

pub fn is_running(bundle_id: &str) -> bool {
    !NSRunningApplication::runningApplicationsWithBundleIdentifier(&NSString::from_str(bundle_id))
        .is_empty()
}

/// Politely ask every instance of the app to quit and wait up to `timeout`.
/// Returns false if the app is still running (e.g. unsaved documents).
pub fn quit_app(bundle_id: &str, timeout: Duration) -> bool {
    let id = NSString::from_str(bundle_id);
    let apps = NSRunningApplication::runningApplicationsWithBundleIdentifier(&id);
    for a in apps.iter() {
        a.terminate();
    }
    let start = Instant::now();
    while start.elapsed() < timeout {
        let left = NSRunningApplication::runningApplicationsWithBundleIdentifier(&id);
        if left.iter().all(|a| a.isTerminated()) {
            return true;
        }
        std::thread::sleep(Duration::from_millis(250));
    }
    false
}

/// Ask every other running instance of `bundle_id` (not this process) to
/// quit, and wait up to `timeout`. Returns false if one is still running.
pub fn quit_other_instances(bundle_id: &str, timeout: Duration) -> bool {
    let me = std::process::id() as i32;
    let id = NSString::from_str(bundle_id);
    let others = || {
        NSRunningApplication::runningApplicationsWithBundleIdentifier(&id)
            .iter()
            .filter(|a| a.processIdentifier() != me && !a.isTerminated())
            .count()
    };
    for a in NSRunningApplication::runningApplicationsWithBundleIdentifier(&id).iter() {
        if a.processIdentifier() != me {
            a.terminate();
        }
    }
    let start = Instant::now();
    while start.elapsed() < timeout {
        if others() == 0 {
            return true;
        }
        std::thread::sleep(Duration::from_millis(250));
    }
    others() == 0
}

/// File icon as PNG bytes (works for apps whose icon lives in Assets.car).
pub fn icon_png(p: &Path, px: f64) -> Option<Vec<u8>> {
    let ws = NSWorkspace::sharedWorkspace();
    let img = ws.iconForFile(&NSString::from_str(&p.to_string_lossy()));
    let mut rect = NSRect::new(NSPoint::new(0.0, 0.0), NSSize::new(px, px));
    let cg = unsafe { img.CGImageForProposedRect_context_hints(&mut rect, None, None) }?;
    let rep = NSBitmapImageRep::initWithCGImage(NSBitmapImageRep::alloc(), &cg);
    let props = NSDictionary::new();
    let data = unsafe { rep.representationUsingType_properties(NSBitmapImageFileType::PNG, &props) }?;
    Some(data.to_vec())
}

pub fn reveal(p: &Path) {
    let _ = Command::new("/usr/bin/open").arg("-R").arg(p).spawn();
}

pub fn quick_look(p: &Path) {
    let _ = Command::new("/usr/bin/qlmanage")
        .arg("-p")
        .arg(p)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn();
}

pub fn open_fda_settings() {
    let _ = Command::new("/usr/bin/open")
        .arg("x-apple.systempreferences:com.apple.preference.security?Privacy_AllFiles")
        .spawn();
}

/// Full Disk Access probe: the system TCC database is only readable with FDA.
pub fn has_full_disk_access() -> bool {
    std::fs::File::open("/Library/Application Support/com.apple.TCC/TCC.db").is_ok()
        || std::fs::read_dir(crate::util::home().join("Library/Safari")).is_ok()
}

/// Batch-read a Spotlight date attribute (e.g. kMDItemLastUsedDate).
/// Missing values are absent from the map.
pub fn spotlight_dates(paths: &[PathBuf], attr: &str) -> HashMap<PathBuf, i64> {
    let mut map = HashMap::new();
    for chunk in paths.chunks(200) {
        let out = Command::new("/usr/bin/mdls")
            .args(["-raw", "-nullMarker", "", "-name", attr])
            .args(chunk)
            .output();
        let Ok(out) = out else { continue };
        let text = String::from_utf8_lossy(&out.stdout);
        let values: Vec<&str> = text.split('\0').collect();
        if values.len() < chunk.len() {
            continue;
        }
        for (p, v) in chunk.iter().zip(values) {
            if let Some(ms) = parse_mdls_date(v.trim()) {
                map.insert(p.clone(), ms);
            }
        }
    }
    map
}

/// Parses "2026-09-29 04:03:41 +0000" into Unix milliseconds.
pub fn parse_mdls_date(s: &str) -> Option<i64> {
    let (date, rest) = s.split_once(' ')?;
    let (time, tz) = rest.split_once(' ').unwrap_or((rest, "+0000"));
    let d: Vec<i64> = date.split('-').map(|x| x.parse().ok()).collect::<Option<_>>()?;
    let t: Vec<i64> = time.split(':').map(|x| x.parse().ok()).collect::<Option<_>>()?;
    if d.len() != 3 || t.len() != 3 {
        return None;
    }
    let (y, m, day) = (d[0], d[1], d[2]);
    // Days from civil (Howard Hinnant).
    let y2 = if m <= 2 { y - 1 } else { y };
    let era = if y2 >= 0 { y2 } else { y2 - 399 } / 400;
    let yoe = y2 - era * 400;
    let mp = (m + 9) % 12;
    let doy = (153 * mp + 2) / 5 + day - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    let days = era * 146097 + doe - 719468;
    let mut secs = days * 86400 + t[0] * 3600 + t[1] * 60 + t[2];
    if tz.len() == 5 {
        let sign = if tz.starts_with('-') { -1 } else { 1 };
        let hh: i64 = tz[1..3].parse().ok()?;
        let mm: i64 = tz[3..5].parse().ok()?;
        secs -= sign * (hh * 3600 + mm * 60);
    }
    Some(secs * 1000)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_mdls_dates() {
        assert_eq!(parse_mdls_date("1970-01-01 00:00:00 +0000"), Some(0));
        assert_eq!(parse_mdls_date("2026-09-29 04:03:41 +0000"), Some(1790654621000));
        assert_eq!(parse_mdls_date("1970-01-01 08:00:00 +0800"), Some(0));
        assert_eq!(parse_mdls_date(""), None);
        assert_eq!(parse_mdls_date("(null)"), None);
    }
}
