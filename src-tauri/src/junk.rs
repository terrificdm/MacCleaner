//! Scanners for the cleaning modules. All are read-only.

use crate::mac;
use crate::model::Item;
use crate::safety::Ctx;
use crate::util::{self, cancelled, file_name, home, list_dir, mtime_ms, now_ms, path_size};
use rayon::prelude::*;
use std::collections::HashSet;
use std::fs;
use std::os::unix::fs::MetadataExt;
use std::path::{Path, PathBuf};

const DAY_MS: i64 = 86_400_000;

/// Cache folders that belong to the developer module instead.
const DEV_CACHE_NAMES: &[&str] = &[
    "Homebrew", "pip", "Yarn", "go-build", "node-gyp", "ms-playwright", "pnpm", "typescript",
];

fn sized(paths: Vec<PathBuf>, ctx: &Ctx) -> Vec<(PathBuf, u64)> {
    paths
        .into_par_iter()
        .filter(|p| !ctx.is_excluded(p))
        .filter(|p| fs::symlink_metadata(p).map(|m| !util::is_special(&m)).unwrap_or(false))
        .map(|p| {
            let s = if cancelled() { 0 } else { path_size(&p) };
            (p, s)
        })
        .filter(|(_, s)| *s > 0)
        .collect()
}

pub fn system_junk() -> Vec<Item> {
    let ctx = Ctx::current();
    let running = mac::running_matcher();
    let lib = home().join("Library");
    let mut items = Vec::new();

    let caches: Vec<PathBuf> = list_dir(&lib.join("Caches"))
        .into_iter()
        .filter(|p| !DEV_CACHE_NAMES.contains(&file_name(p).as_str()))
        .collect();
    for (p, size) in sized(caches, &ctx) {
        let name = file_name(&p);
        let lower = name.to_lowercase();
        let mut it = Item::path(&p, "userCaches", size, true);
        if running(&name) {
            it.selected = false;
            it.note = Some("running".into());
        } else if lower.starts_with("com.apple.") || lower == "cloudkit" || lower == "familycircle" {
            it.selected = false;
            it.note = Some("apple".into());
        }
        items.push(it);
    }

    let logs: Vec<PathBuf> = list_dir(&lib.join("Logs"))
        .into_iter()
        .filter(|p| file_name(p) != "DiagnosticReports")
        .collect();
    for (p, size) in sized(logs, &ctx) {
        items.push(Item::path(&p, "logs", size, true));
    }

    let reports = list_dir(&lib.join("Logs/DiagnosticReports"));
    for (p, size) in sized(reports, &ctx) {
        items.push(Item::path(&p, "crashReports", size, true));
    }

    if let Some(tmp) = std::env::var("TMPDIR").ok().and_then(|t| fs::canonicalize(t).ok()) {
        let cutoff = now_ms() - 3 * DAY_MS;
        let old: Vec<PathBuf> = list_dir(&tmp)
            .into_iter()
            .filter(|p| newest_mtime(p, 20_000).map(|t| t < cutoff).unwrap_or(false))
            .collect();
        for (p, size) in sized(old, &ctx) {
            items.push(Item::path(&p, "tempFiles", size, true));
        }
    }
    crate::inuse::InUse::current().mark(&mut items);
    items.sort_by(|a, b| b.size.cmp(&a.size));
    items
}

pub fn dev_junk(nm_roots: &[String], nm_days: u64) -> Vec<Item> {
    let ctx = Ctx::current();
    let h = home();
    let mut items = Vec::new();
    let known: &[(&str, &str, bool)] = &[
        (".npm/_cacache", "npm", true),
        (".npm/_npx", "npm", true),
        (".npm/_logs", "npm", true),
        ("Library/Caches/pip", "pip", true),
        ("Library/Caches/Yarn", "yarn", true),
        ("Library/pnpm/store", "pnpm", true),
        ("Library/Caches/node-gyp", "npm", true),
        ("Library/Caches/typescript", "npm", true),
        ("Library/Caches/ms-playwright", "playwright", false),
        (".bun/install/cache", "bun", true),
        (".cargo/registry/cache", "cargo", true),
        ("Library/Caches/go-build", "go", true),
        (".gradle/caches", "gradle", true),
        (".m2/repository", "maven", false),
        ("Library/Developer/Xcode/DerivedData", "xcode", true),
        ("Library/Developer/Xcode/iOS DeviceSupport", "xcode", true),
        ("Library/Developer/Xcode/Archives", "xcode", false),
        ("Library/Developer/CoreSimulator/Caches", "xcode", true),
    ];
    let paths: Vec<(PathBuf, &str, bool)> = known
        .iter()
        .map(|(rel, g, sel)| (h.join(rel), *g, *sel))
        .filter(|(p, _, _)| p.exists() && !ctx.is_excluded(p))
        .collect();
    let sized: Vec<Item> = paths
        .into_par_iter()
        .map(|(p, g, sel)| {
            let s = path_size(&p);
            Item::path(&p, g, s, sel)
        })
        .filter(|it| it.size > 0)
        .collect();
    items.extend(sized);

    let brew_cache = h.join("Library/Caches/Homebrew");
    if let Some(brew) = util::find_tool("brew") {
        // `--prune=all` removes all cached downloads and old versions of
        // installed formulae; ask brew how much that frees.
        let size = util::run(&brew, &["cleanup", "-n", "--prune=all"])
            .ok()
            .and_then(|o| brew_cleanup_estimate(&o.stdout))
            .unwrap_or_else(|| path_size(&brew_cache));
        if size > 0 && !ctx.is_excluded(&brew_cache) {
            items.push(Item {
                path: util::path_str(&brew_cache),
                name: "brew cleanup --prune=all".into(),
                size,
                group: "homebrew".into(),
                kind: "command".into(),
                // Permanent deletions are never preselected.
                selected: false,
                note: Some("permanent".into()),
                command: Some("brewCleanup".into()),
                ..Default::default()
            });
        }
    }

    if let Some(size) = docker_reclaimable() {
        items.push(Item {
            path: "docker://system".into(),
            name: "docker system prune -f".into(),
            size,
            group: "docker".into(),
            kind: "command".into(),
            // Permanent deletions are never preselected.
            selected: false,
            note: Some("permanent".into()),
            command: Some("dockerPrune".into()),
            ..Default::default()
        });
    }

    items.extend(node_modules(nm_roots, nm_days, &ctx));
    crate::inuse::InUse::current().mark(&mut items);
    items
}

/// Reclaimable bytes reported by `docker system df`, only if the daemon runs.
fn docker_reclaimable() -> Option<u64> {
    let docker = util::find_tool("docker")?;
    let out = util::run(&docker, &["system", "df", "--format", "{{.Type}}\t{{.Reclaimable}}"]).ok()?;
    if !out.ok {
        return None;
    }
    Some(docker_prune_estimate(&out.stdout))
}

/// Bytes `docker system prune -f` would free, from `docker system df` rows
/// of "<Type>\t<Reclaimable>". Without -a/--volumes, prune removes stopped
/// containers and build cache only.
pub fn docker_prune_estimate(df: &str) -> u64 {
    df.lines()
        .filter_map(|l| l.split_once('\t'))
        .filter(|(ty, _)| matches!(ty.trim(), "Containers" | "Build Cache"))
        .filter_map(|(_, r)| parse_human_size(r.split_whitespace().next()?))
        .sum()
}

/// Parses "This operation would free approximately 1.2GB of disk space."
/// from `brew cleanup -n`.
pub fn brew_cleanup_estimate(out: &str) -> Option<u64> {
    let rest = out.split("would free approximately").nth(1)?;
    parse_human_size(rest.split_whitespace().next()?)
}

/// "1.2GB" / "512.3MB" / "0B" -> bytes (docker uses powers of 1000).
pub fn parse_human_size(s: &str) -> Option<u64> {
    let s = s.trim();
    let idx = s.find(|c: char| c.is_ascii_alphabetic())?;
    let (num, unit) = s.split_at(idx);
    let n: f64 = num.parse().ok()?;
    let mul = match unit.to_ascii_uppercase().as_str() {
        "B" => 1.0,
        "KB" | "K" => 1e3,
        "MB" | "M" => 1e6,
        "GB" | "G" => 1e9,
        "TB" | "T" => 1e12,
        _ => return None,
    };
    Some((n * mul).round() as u64)
}

fn node_modules(roots: &[String], days: u64, ctx: &Ctx) -> Vec<Item> {
    let h = home();
    let skip: Vec<PathBuf> = vec![h.join("Library"), h.join(".Trash"), h.join("Applications")];
    let found: Vec<PathBuf> = roots
        .iter()
        .map(|r| util::expand(r))
        .filter(|r| r.is_dir())
        .flat_map(|r| {
            let mut out = Vec::new();
            find_nm(&r, 0, &skip, ctx, &mut out);
            out
        })
        .collect();
    let mut uniq = HashSet::new();
    let cutoff = now_ms() - days as i64 * DAY_MS;
    let mut items: Vec<Item> = found
        .into_iter()
        .filter(|p| uniq.insert(p.clone()))
        .collect::<Vec<_>>()
        .into_par_iter()
        .filter_map(|nm| {
            let project = nm.parent()?.to_path_buf();
            let last = project_activity(&project);
            if last.map(|t| t >= cutoff).unwrap_or(false) {
                return None;
            }
            let size = path_size(&nm);
            let mut it = Item::path(&nm, "nodeModules", size, true);
            it.name = file_name(&project);
            it.modified = last;
            Some(it)
        })
        .filter(|it| it.size > 0)
        .collect();
    items.sort_by(|a, b| b.size.cmp(&a.size));
    items
}

/// Rust build output (`target` next to Cargo.toml) or a Python virtualenv.
/// These are regenerated as a whole; single files inside must not be offered.
fn is_build_output(dir: &Path, name: &str) -> bool {
    (name == "target" && dir.parent().map(|p| p.join("Cargo.toml").is_file()).unwrap_or(false))
        || dir.join("pyvenv.cfg").is_file()
}

fn find_nm(dir: &Path, depth: usize, skip: &[PathBuf], ctx: &Ctx, out: &mut Vec<PathBuf>) {
    if depth > 10 || cancelled() || skip.iter().any(|s| dir == s) || ctx.is_excluded(dir) {
        return;
    }
    let Ok(rd) = fs::read_dir(dir) else { return };
    for e in rd.flatten() {
        let Ok(ft) = e.file_type() else { continue };
        if !ft.is_dir() {
            continue;
        }
        let name = e.file_name().to_string_lossy().into_owned();
        if name == "node_modules" {
            // Only installed dependencies: a real package sits next to it.
            // Test fixtures and vendored folders have no package.json.
            if dir.join("package.json").is_file() {
                out.push(e.path());
            }
        } else if !name.starts_with('.') && !util::is_package(&name) {
            find_nm(&e.path(), depth + 1, skip, ctx, out);
        }
    }
}

/// Most recent modification among a project's top-level entries, also
/// counting the enclosing git repository's root so sub-packages of an
/// active monorepo are not treated as idle.
fn project_activity(project: &Path) -> Option<i64> {
    let top = |d: &Path| {
        list_dir(d)
            .iter()
            .filter(|p| file_name(p) != "node_modules")
            .filter_map(|p| fs::symlink_metadata(p).ok().and_then(|m| mtime_ms(&m)))
            .max()
    };
    let repo = project.ancestors().skip(1).take(8).find(|a| a.join(".git").exists());
    top(project).max(repo.and_then(top))
}

/// Newest modification time of `p` and everything below it. Visits at most
/// `budget` entries; if the budget runs out the item is reported as modified
/// now, so a huge folder is never assumed to be idle.
pub fn newest_mtime(p: &Path, budget: usize) -> Option<i64> {
    let mut newest = fs::symlink_metadata(p).ok().and_then(|m| mtime_ms(&m))?;
    let mut stack = vec![p.to_path_buf()];
    let mut seen = 0usize;
    while let Some(dir) = stack.pop() {
        let Ok(rd) = fs::read_dir(&dir) else { continue };
        for e in rd.flatten() {
            seen += 1;
            if seen > budget {
                return Some(now_ms());
            }
            let Ok(m) = e.metadata() else { continue };
            if let Some(t) = mtime_ms(&m) {
                newest = newest.max(t);
            }
            if m.is_dir() {
                stack.push(e.path());
            }
        }
    }
    Some(newest)
}

/// Browsers and download tools write to these until the download finishes.
pub fn is_partial_download(name: &str) -> bool {
    let n = name.to_ascii_lowercase();
    [".crdownload", ".part", ".partial", ".download", ".opdownload", ".aria2"]
        .iter()
        .any(|e| n.ends_with(e))
}

/// Decimal megabytes, matching the sizes shown in the UI.
const LARGE_FLOOR: u64 = 50_000_000;

pub fn large_files() -> Vec<Item> {
    let ctx = Ctx::current();
    let h = home();
    // Whole home folder, including hidden folders and ~/Library. Protected
    // paths and the whitelist are skipped via `ctx.is_excluded`.
    let mut skip = vec![h.join(".Trash")];
    if !mac::has_full_disk_access() {
        // Reading other apps' containers without FDA triggers a blocking prompt.
        skip.push(h.join("Library/Containers"));
        skip.push(h.join("Library/Group Containers"));
    }
    let mut found: Vec<(PathBuf, u64, Option<i64>)> = Vec::new();
    walk_large(&h, &skip, &ctx, &mut found);
    let paths: Vec<PathBuf> = found.iter().map(|(p, _, _)| p.clone()).collect();
    let used = mac::spotlight_dates(&paths, "kMDItemLastUsedDate");
    let mut items: Vec<Item> = found
        .into_iter()
        .map(|(p, size, mtime)| {
            let mut it = Item::path(&p, file_kind(&p), size, false);
            it.modified = mtime;
            it.last_used = used.get(&p).copied();
            it
        })
        .collect();
    crate::inuse::InUse::current().mark(&mut items);
    items.sort_by(|a, b| b.size.cmp(&a.size));
    items
}

fn walk_large(dir: &Path, skip: &[PathBuf], ctx: &Ctx, out: &mut Vec<(PathBuf, u64, Option<i64>)>) {
    if cancelled() || skip.iter().any(|s| dir == s) || ctx.is_excluded(dir) {
        return;
    }
    let Ok(dir_meta) = fs::symlink_metadata(dir) else { return };
    let Ok(rd) = fs::read_dir(dir) else { return };
    let mut subdirs = Vec::new();
    for e in rd.flatten() {
        let name = e.file_name().to_string_lossy().into_owned();
        let Ok(m) = e.metadata() else { continue };
        if m.is_dir() {
            let path = e.path();
            let skip_dir = name == "node_modules"
                || name == ".git"
                || util::is_package(&name)
                || m.dev() != dir_meta.dev() // another volume mounted here
                || is_build_output(&path, &name);
            if !skip_dir {
                subdirs.push(path);
            }
        } else if m.is_file() && util::alloc_size(&m) >= LARGE_FLOOR {
            if !ctx.is_excluded(&e.path()) {
                out.push((e.path(), util::alloc_size(&m), mtime_ms(&m)));
            }
        }
    }
    let nested: Vec<Vec<(PathBuf, u64, Option<i64>)>> = subdirs
        .par_iter()
        .map(|d| {
            let mut v = Vec::new();
            walk_large(d, skip, ctx, &mut v);
            v
        })
        .collect();
    out.extend(nested.into_iter().flatten());
}

pub fn file_kind(p: &Path) -> &'static str {
    let ext = p.extension().map(|e| e.to_string_lossy().to_lowercase()).unwrap_or_default();
    match ext.as_str() {
        "mp4" | "mov" | "mkv" | "avi" | "m4v" | "webm" | "flv" | "wmv" => "video",
        "zip" | "rar" | "7z" | "tar" | "gz" | "tgz" | "bz2" | "xz" | "zst" => "archive",
        "dmg" | "iso" | "pkg" | "img" | "xip" | "vmdk" | "qcow2" | "vdi" | "ipsw" => "diskImage",
        "mp3" | "wav" | "flac" | "aac" | "m4a" | "aiff" => "audio",
        "jpg" | "jpeg" | "png" | "heic" | "raw" | "cr2" | "nef" | "tif" | "tiff" | "psd" => "image",
        "pdf" | "doc" | "docx" | "ppt" | "pptx" | "xls" | "xlsx" | "key" | "pages" | "numbers" => "document",
        _ => "other",
    }
}

pub fn trash_items() -> Vec<Item> {
    let ctx = Ctx::current();
    let entries = list_dir(&home().join(".Trash"));
    let mut items: Vec<Item> = entries
        .into_par_iter()
        .filter(|p| file_name(p) != ".DS_Store" && !ctx.is_excluded(p))
        .map(|p| {
            let s = path_size(&p);
            let mut it = Item::path(&p, "trash", s, false);
            it.modified = fs::symlink_metadata(&p).ok().and_then(|m| mtime_ms(&m));
            it
        })
        .collect();
    items.sort_by(|a, b| b.size.cmp(&a.size));
    items
}

pub fn downloads() -> Vec<Item> {
    let ctx = Ctx::current();
    let entries: Vec<PathBuf> = list_dir(&home().join("Downloads"))
        .into_iter()
        .filter(|p| {
            let n = file_name(p);
            n != ".DS_Store" && n != ".localized" && !is_partial_download(&n)
                && fs::symlink_metadata(p).map(|m| !util::is_special(&m)).unwrap_or(false)
        })
        .collect();
    let added = mac::spotlight_dates(&entries, "kMDItemDateAdded");
    let used = mac::spotlight_dates(&entries, "kMDItemLastUsedDate");
    let cutoff = now_ms() - 30 * DAY_MS;
    let mut items: Vec<Item> = entries
        .into_par_iter()
        .filter(|p| !ctx.is_excluded(p))
        .map(|p| {
            let s = path_size(&p);
            let meta = fs::symlink_metadata(&p).ok();
            let ext = p.extension().map(|e| e.to_string_lossy().to_lowercase()).unwrap_or_default();
            let installer = matches!(ext.as_str(), "dmg" | "pkg" | "mpkg" | "xip" | "iso");
            let group = if installer { "installers" } else if file_kind(&p) == "archive" { "archives" } else { "downloads" };
            let mut it = Item::path(&p, group, s, false);
            it.modified = added.get(&p).copied().or_else(|| meta.as_ref().and_then(mtime_ms));
            it.last_used = used.get(&p).copied();
            // Suggest installers that have been sitting around for a month.
            it.selected = installer && it.modified.map(|t| t < cutoff).unwrap_or(false);
            it
        })
        .filter(|it| it.size > 0)
        .collect();
    crate::inuse::InUse::current().mark(&mut items);
    items.sort_by(|a, b| b.size.cmp(&a.size));
    items
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn docker_estimate_counts_only_what_prune_removes() {
        // `docker system prune -f` removes stopped containers and build cache,
        // not unused (non-dangling) images or volumes.
        let df = "Images\t12.5GB (80%)\nContainers\t1.2GB (100%)\nLocal Volumes\t3GB (60%)\nBuild Cache\t800MB\n";
        assert_eq!(docker_prune_estimate(df), 2_000_000_000);
    }

    #[test]
    fn large_floor_is_decimal_megabytes() {
        let d = tempfile::tempdir().unwrap();
        let h = fs::canonicalize(d.path()).unwrap();
        fs::write(h.join("clip.mov"), vec![1u8; 51_000_000]).unwrap();
        let ctx = Ctx { home: h.clone(), tmp: None, whitelist: vec![] };
        let mut out = Vec::new();
        walk_large(&h, &[], &ctx, &mut out);
        assert_eq!(out.len(), 1, "a 51 MB file is above the 50 MB floor shown in the UI");
    }

    #[test]
    fn parses_brew_cleanup_estimate() {
        let out = "Would remove: /Users/x/Library/Caches/Homebrew/foo--1.0.tar.gz (12.3MB)\n==> This operation would free approximately 1.2GB of disk space.\n";
        assert_eq!(brew_cleanup_estimate(out), Some(1_200_000_000));
        assert_eq!(brew_cleanup_estimate("nothing to do"), None);
    }

    #[test]
    fn parses_docker_sizes() {
        assert_eq!(parse_human_size("0B"), Some(0));
        assert_eq!(parse_human_size("1.5GB"), Some(1_500_000_000));
        assert_eq!(parse_human_size("512.3kB"), Some(512_300));
        assert_eq!(parse_human_size("abc"), None);
    }

    #[test]
    fn finds_stale_node_modules_only() {
        let d = tempfile::tempdir().unwrap();
        let r = fs::canonicalize(d.path()).unwrap();
        let old = r.join("old-proj");
        let fresh = r.join("fresh-proj");
        for p in [&old, &fresh] {
            fs::create_dir_all(p.join("node_modules/pkg/node_modules/inner")).unwrap();
            fs::write(p.join("node_modules/pkg/index.js"), vec![1u8; 5000]).unwrap();
            fs::write(p.join("package.json"), b"{}").unwrap();
        }
        // Make old-proj look untouched for 200 days.
        let t = std::time::SystemTime::now() - std::time::Duration::from_secs(200 * 86400);
        let f = fs::File::options().write(true).open(old.join("package.json")).unwrap();
        f.set_modified(t).unwrap();
        let ctx = Ctx { home: r.clone(), tmp: None, whitelist: vec![] };
        let items = node_modules(&[util::path_str(&r)], 90, &ctx);
        assert_eq!(items.len(), 1, "{items:?}");
        assert_eq!(items[0].name, "old-proj");
        assert!(items[0].path.ends_with("old-proj/node_modules"));
    }

    #[test]
    fn large_scan_includes_hidden_and_library_but_not_protected() {
        let d = tempfile::tempdir().unwrap();
        let h = fs::canonicalize(d.path()).unwrap();
        let big = vec![1u8; (LARGE_FLOOR + 4096) as usize];
        for rel in [".colima/disk.img", "Library/Caches/app/blob.bin", "Documents/keep.mov", ".Trash/old.iso", "small/ok.txt"] {
            let p = h.join(rel);
            fs::create_dir_all(p.parent().unwrap()).unwrap();
            fs::write(&p, if rel.ends_with("ok.txt") { &big[..10] } else { &big[..] }).unwrap();
        }
        let whitelist = crate::settings::DEFAULT_WHITELIST.iter().map(|p| h.join(p.trim_start_matches("~/"))).collect();
        let ctx = Ctx { home: h.clone(), tmp: None, whitelist };
        let mut out = Vec::new();
        walk_large(&h, &[h.join(".Trash")], &ctx, &mut out);
        let mut got: Vec<String> = out.iter().map(|(p, _, _)| p.strip_prefix(&h).unwrap().to_string_lossy().into_owned()).collect();
        got.sort();
        assert_eq!(got, vec![".colima/disk.img", "Library/Caches/app/blob.bin"]);
    }

    #[test]
    fn large_scan_skips_git_build_output_and_venvs() {
        let d = tempfile::tempdir().unwrap();
        let h = fs::canonicalize(d.path()).unwrap();
        let big = vec![1u8; (LARGE_FLOOR + 4096) as usize];
        let put = |rel: &str| {
            let p = h.join(rel);
            fs::create_dir_all(p.parent().unwrap()).unwrap();
            fs::write(&p, &big).unwrap();
        };
        put("temp/repo/.git/objects/pack/pack-1.pack");
        put("temp/rust/target/debug/libx.a");
        fs::write(h.join("temp/rust/Cargo.toml"), b"[package]").unwrap();
        put("temp/py/.venv/lib/node");
        fs::write(h.join("temp/py/.venv/pyvenv.cfg"), b"home = /usr").unwrap();
        put("temp/py/env2/lib/big.so");
        fs::write(h.join("temp/py/env2/pyvenv.cfg"), b"home = /usr").unwrap();
        // A plain folder that happens to be called "target" is still scanned.
        put("Movies/target/holiday.mov");
        let ctx = Ctx { home: h.clone(), tmp: None, whitelist: vec![] };
        let mut out = Vec::new();
        walk_large(&h, &[], &ctx, &mut out);
        let got: Vec<String> = out.iter().map(|(p, _, _)| p.strip_prefix(&h).unwrap().to_string_lossy().into_owned()).collect();
        assert_eq!(got, vec!["Movies/target/holiday.mov"]);
    }

    #[test]
    fn large_scan_never_lists_what_safety_refuses() {
        let d = tempfile::tempdir().unwrap();
        let h = fs::canonicalize(d.path()).unwrap();
        let big = vec![1u8; (LARGE_FLOOR + 4096) as usize];
        for rel in ["Desktop/Talk.key/Data/movie.mov", "VMs/disk.sparsebundle/bands/0", "Code/App.xcodeproj/big.bin", "Movies/clip.mov"] {
            let p = h.join(rel);
            fs::create_dir_all(p.parent().unwrap()).unwrap();
            fs::write(&p, &big).unwrap();
        }
        let ctx = Ctx { home: h.clone(), tmp: None, whitelist: vec![] };
        let mut out = Vec::new();
        walk_large(&h, &[], &ctx, &mut out);
        for (p, _, _) in &out {
            assert_eq!(crate::safety::check_deletable(p, &ctx), Ok(()), "{}", p.display());
        }
        assert_eq!(out.len(), 1);
    }

    #[test]
    fn node_modules_needs_package_json() {
        let d = tempfile::tempdir().unwrap();
        let r = fs::canonicalize(d.path()).unwrap();
        let old = std::time::SystemTime::now() - std::time::Duration::from_secs(200 * 86400);
        // A test fixture: node_modules without a package.json next to it.
        let fx = r.join("repo/test/fixtures");
        fs::create_dir_all(fx.join("node_modules/foo")).unwrap();
        fs::write(fx.join("node_modules/foo/package.json"), vec![1u8; 5000]).unwrap();
        fs::File::options().write(true).open(fx.join("node_modules/foo/package.json")).unwrap().set_modified(old).unwrap();
        let ctx = Ctx { home: r.clone(), tmp: None, whitelist: vec![] };
        set_tree_mtime(&r, old);
        assert!(node_modules(&[util::path_str(&r)], 90, &ctx).is_empty());
    }

    #[test]
    fn node_modules_in_active_monorepo_is_not_idle() {
        let d = tempfile::tempdir().unwrap();
        let r = fs::canonicalize(d.path()).unwrap();
        let old = std::time::SystemTime::now() - std::time::Duration::from_secs(200 * 86400);
        let pkg = r.join("mono/packages/ui");
        fs::create_dir_all(pkg.join("node_modules/dep")).unwrap();
        fs::write(pkg.join("node_modules/dep/index.js"), vec![1u8; 5000]).unwrap();
        fs::write(pkg.join("package.json"), b"{}").unwrap();
        fs::create_dir_all(r.join("mono/.git")).unwrap();
        set_tree_mtime(&r, old);
        // Someone edits the repo root today; the sub-package is part of an active project.
        fs::write(r.join("mono/README.md"), b"edited").unwrap();
        let ctx = Ctx { home: r.clone(), tmp: None, whitelist: vec![] };
        assert!(node_modules(&[util::path_str(&r)], 90, &ctx).is_empty());
        // Once the whole repo is idle, it is listed.
        set_tree_mtime(&r, old);
        assert_eq!(node_modules(&[util::path_str(&r)], 90, &ctx).len(), 1);
    }

    fn set_tree_mtime(dir: &Path, t: std::time::SystemTime) {
        for e in fs::read_dir(dir).unwrap().flatten() {
            let p = e.path();
            if p.is_dir() {
                set_tree_mtime(&p, t);
            }
            if let Ok(f) = fs::File::open(&p) {
                let _ = f.set_modified(t);
            }
        }
    }

    #[test]
    fn newest_mtime_looks_inside_folders() {
        let d = tempfile::tempdir().unwrap();
        let r = fs::canonicalize(d.path()).unwrap();
        let old = std::time::SystemTime::now() - std::time::Duration::from_secs(30 * 86400);
        fs::create_dir_all(r.join("job/sub")).unwrap();
        fs::write(r.join("job/sub/live.sock"), b"x").unwrap();
        fs::File::open(r.join("job")).unwrap().set_modified(old).unwrap();
        let newest = newest_mtime(&r.join("job"), 10_000).unwrap();
        assert!(now_ms() - newest < 60_000, "nested recent file must count");
    }

    #[test]
    fn recognises_partial_downloads() {
        for n in ["video.mp4.crdownload", "big.iso.part", "a.zip.download", "x.opdownload", "y.partial", "z.aria2"] {
            assert!(is_partial_download(n), "{n}");
        }
        for n in ["report.pdf", "part.txt", "download.dmg"] {
            assert!(!is_partial_download(n), "{n}");
        }
    }

    #[test]
    fn classifies_files() {
        assert_eq!(file_kind(Path::new("/a/b.MOV")), "video");
        assert_eq!(file_kind(Path::new("/a/b.dmg")), "diskImage");
        assert_eq!(file_kind(Path::new("/a/b")), "other");
    }
}
