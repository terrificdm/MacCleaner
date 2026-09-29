//! Space Lens: scans the data volume into an in-memory tree.

use crate::safety::{self, Ctx};
use crate::util::alloc_size;
use rayon::prelude::*;
use serde::Serialize;
use std::collections::HashSet;
use std::fs;
use std::os::unix::fs::MetadataExt;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

pub const DATA_ROOT: &str = "/System/Volumes/Data";
/// Files smaller than this are folded into their directory's "other" bucket.
const KEEP_FILE: u64 = 2 << 20;
/// Directories smaller than this are folded as well.
const KEEP_DIR: u64 = 1 << 20;

#[derive(Debug)]
pub struct Node {
    pub name: String,
    pub size: u64,
    pub is_dir: bool,
    /// Size of folded small files and directories.
    pub other: u64,
    pub children: Vec<Node>,
}

pub struct Progress {
    pub files: AtomicU64,
    pub bytes: AtomicU64,
    pub denied: AtomicU64,
    pub current: Mutex<String>,
    pub cancel: AtomicBool,
}

impl Progress {
    pub fn new() -> Progress {
        Progress {
            files: AtomicU64::new(0),
            bytes: AtomicU64::new(0),
            denied: AtomicU64::new(0),
            current: Mutex::new(String::new()),
            cancel: AtomicBool::new(false),
        }
    }
}

struct Walk<'a> {
    dev: u64,
    seen: Mutex<HashSet<(u64, u64)>>,
    prog: &'a Progress,
    excluded: &'a (dyn Fn(&Path) -> bool + Sync),
}

pub fn scan(root: &Path, prog: &Progress, excluded: &(dyn Fn(&Path) -> bool + Sync)) -> Option<Node> {
    let meta = fs::symlink_metadata(root).ok()?;
    let w = Walk { dev: meta.dev(), seen: Mutex::new(HashSet::new()), prog, excluded };
    let pool = rayon::ThreadPoolBuilder::new().stack_size(16 << 20).build().ok()?;
    let node = pool.install(|| scan_dir(root, crate::util::file_name(root), &w));
    if prog.cancel.load(Ordering::Relaxed) {
        None
    } else {
        Some(node)
    }
}

fn scan_dir(p: &Path, name: String, w: &Walk) -> Node {
    let mut node = Node { name, size: 0, is_dir: true, other: 0, children: Vec::new() };
    if w.prog.cancel.load(Ordering::Relaxed) {
        return node;
    }
    let rd = match fs::read_dir(p) {
        Ok(rd) => rd,
        Err(_) => {
            w.prog.denied.fetch_add(1, Ordering::Relaxed);
            return node;
        }
    };
    let mut subdirs: Vec<(PathBuf, String)> = Vec::new();
    let mut count = 0u64;
    for e in rd.flatten() {
        let Ok(m) = e.metadata() else { continue };
        let fname = e.file_name().to_string_lossy().into_owned();
        if m.is_dir() {
            if m.dev() == w.dev {
                node.size += alloc_size(&m);
                subdirs.push((e.path(), fname));
            }
            continue;
        }
        count += 1;
        if m.nlink() > 1 && !w.seen.lock().unwrap().insert((m.dev(), m.ino())) {
            continue; // Hard link already counted.
        }
        let s = alloc_size(&m);
        node.size += s;
        if s >= KEEP_FILE {
            node.children.push(Node { name: fname, size: s, is_dir: false, other: 0, children: vec![] });
        } else {
            node.other += s;
        }
    }
    let total_files = w.prog.files.fetch_add(count, Ordering::Relaxed) + count;
    w.prog.bytes.fetch_add(node.size, Ordering::Relaxed);
    if total_files % 512 < count.max(1) {
        if let Ok(mut c) = w.prog.current.try_lock() {
            *c = p.to_string_lossy().into_owned();
        }
    }

    let subs: Vec<Node> = subdirs
        .into_par_iter()
        .filter(|(sp, _)| !(w.excluded)(sp))
        .map(|(sp, n)| scan_dir(&sp, n, w))
        .collect();
    for c in subs {
        node.size += c.size;
        if c.size >= KEEP_DIR {
            node.children.push(c);
        } else {
            node.other += c.size;
        }
    }
    node.children.sort_by(|a, b| b.size.cmp(&a.size));
    node.children.shrink_to_fit();
    node
}

/// Paths the whole-disk scan skips. Without Full Disk Access, other apps'
/// containers are skipped because reading them triggers a blocking macOS
/// privacy prompt; that space shows up as "Other". Whitelisted folders are
/// still counted (so the chart adds up) but are never deletable.
pub fn scan_exclusions() -> impl Fn(&Path) -> bool + Sync {
    let mut skip: Vec<PathBuf> = Vec::new();
    if !crate::mac::has_full_disk_access() {
        let lib = crate::util::home().join("Library");
        skip.push(lib.join("Containers"));
        skip.push(lib.join("Group Containers"));
    }
    // The scan walks /System/Volumes/Data/...; match both spellings.
    let with_data: Vec<PathBuf> = skip
        .iter()
        .map(|p| Path::new(DATA_ROOT).join(p.strip_prefix("/").unwrap_or(p)))
        .collect();
    skip.extend(with_data);
    move |p: &Path| skip.iter().any(|s| p.starts_with(s))
}

/// A scan result plus extra synthetic nodes, kept for the session only.
pub struct Snapshot {
    pub root: Node,
    pub display_prefix: String,
    pub system_used: u64,
    pub hidden: u64,
}

pub static SNAPSHOT: Mutex<Option<Arc<Snapshot>>> = Mutex::new(None);

#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ViewNode {
    pub name: String,
    pub path: String,
    pub size: u64,
    /// "dir" | "file" | "other" | "system" | "hidden"
    pub kind: String,
    pub deletable: bool,
    pub children: Vec<ViewNode>,
}

/// Name of the startup volume as shown in Finder ("Macintosh HD" unless renamed).
pub fn volume_name() -> String {
    static NAME: std::sync::OnceLock<String> = std::sync::OnceLock::new();
    NAME.get_or_init(|| {
        std::process::Command::new("/usr/sbin/diskutil")
            .args(["info", "-plist", "/"])
            .output()
            .ok()
            .and_then(|o| plist::Value::from_reader_xml(&o.stdout[..]).ok())
            .and_then(|v| v.as_dictionary()?.get("VolumeName")?.as_string().map(str::to_string))
            .filter(|n| !n.is_empty())
            .unwrap_or_else(|| "Macintosh HD".into())
    })
    .clone()
}

/// Used/total bytes of the volume containing `p`.
pub fn volume_usage(p: &str) -> (u64, u64, u64) {
    let c = std::ffi::CString::new(p).unwrap();
    let mut s: libc::statfs = unsafe { std::mem::zeroed() };
    if unsafe { libc::statfs(c.as_ptr(), &mut s) } != 0 {
        return (0, 0, 0);
    }
    let bs = s.f_bsize as u64;
    let total = s.f_blocks * bs;
    let free = s.f_bfree * bs;
    let avail = s.f_bavail * bs;
    (total.saturating_sub(free), total, avail)
}

fn display_path(prefix: &str, rel: &str) -> String {
    let joined = format!("{}/{}", prefix.trim_end_matches('/'), rel.trim_start_matches('/'));
    if let Some(stripped) = joined.strip_prefix(DATA_ROOT) {
        if stripped.is_empty() { "/".into() } else { stripped.into() }
    } else {
        joined
    }
}

fn find<'a>(root: &'a Node, rel: &[&str]) -> Option<&'a Node> {
    let mut cur = root;
    for part in rel {
        cur = cur.children.iter().find(|c| c.is_dir && c.name == *part)?;
    }
    Some(cur)
}

/// Build a view of the subtree at `rel_path` (relative to the scan root,
/// "" for the root) limited to `depth` levels. Tiny slices are merged into
/// an "other" node so the chart stays readable.
pub fn view(snap: &Snapshot, rel_path: &str, depth: usize) -> Option<ViewNode> {
    let parts: Vec<&str> = rel_path.split('/').filter(|s| !s.is_empty()).collect();
    let node = find(&snap.root, &parts)?;
    let ctx = Ctx::current();
    let base = parts.join("/");
    let mut v = build(node, &base, &snap.display_prefix, depth, node.size.max(1), &ctx);
    if parts.is_empty() {
        v.name = volume_name();
        v.path = "/".into();
        v.deletable = false;
        for (kind, size) in [("system", snap.system_used), ("hidden", snap.hidden)] {
            if size > 0 {
                v.children.push(ViewNode {
                    name: kind.into(),
                    path: format!("::{kind}"),
                    size,
                    kind: kind.into(),
                    deletable: false,
                    children: vec![],
                });
                v.size += size;
            }
        }
        v.children.sort_by(|a, b| b.size.cmp(&a.size));
    }
    Some(v)
}

fn build(n: &Node, rel: &str, prefix: &str, depth: usize, view_total: u64, ctx: &Ctx) -> ViewNode {
    let path = display_path(prefix, rel);
    let mut v = ViewNode {
        name: n.name.clone(),
        path: path.clone(),
        size: n.size,
        kind: if n.is_dir { "dir".into() } else { "file".into() },
        deletable: safety::check_deletable(Path::new(&path), ctx).is_ok(),
        children: vec![],
    };
    if depth == 0 || !n.is_dir {
        return v;
    }
    let min = view_total / 400; // 0.25% of the visible total
    let cap = if depth >= 3 { 28 } else { 14 };
    let mut rest = n.other;
    for (i, c) in n.children.iter().enumerate() {
        if i < cap && c.size >= min {
            let crel = if rel.is_empty() { c.name.clone() } else { format!("{rel}/{}", c.name) };
            v.children.push(build(c, &crel, prefix, depth - 1, view_total, ctx));
        } else {
            rest += c.size;
        }
    }
    if rest > 0 && rest >= min {
        v.children.push(ViewNode {
            name: "other".into(),
            path: format!("{path}/::other"),
            size: rest,
            kind: "other".into(),
            deletable: false,
            children: vec![],
        });
    }
    v
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn write(p: &Path, n: usize) {
        fs::create_dir_all(p.parent().unwrap()).unwrap();
        let mut f = fs::File::create(p).unwrap();
        f.write_all(&vec![1u8; n]).unwrap();
    }

    #[test]
    fn scans_tree_and_dedupes_hardlinks() {
        let d = tempfile::tempdir().unwrap();
        let r = d.path();
        write(&r.join("big/a.bin"), 3 << 20);
        write(&r.join("small/b.txt"), 100);
        fs::hard_link(r.join("big/a.bin"), r.join("big/a-link.bin")).unwrap();
        let prog = Progress::new();
        let n = scan(r, &prog, &|_| false).unwrap();
        let big = n.children.iter().find(|c| c.name == "big").unwrap();
        // One copy of 3 MB, not two.
        assert!(big.size >= 3 << 20 && big.size < 5 << 20, "size {}", big.size);
        assert_eq!(big.children.iter().filter(|c| !c.is_dir).count(), 1);
        // The tiny dir is folded into "other".
        assert!(n.children.iter().all(|c| c.name != "small"));
        assert!(n.other > 0);
        assert_eq!(prog.files.load(Ordering::Relaxed), 3);
    }

    #[test]
    fn cancel_returns_none() {
        let d = tempfile::tempdir().unwrap();
        let prog = Progress::new();
        prog.cancel.store(true, Ordering::Relaxed);
        assert!(scan(d.path(), &prog, &|_| false).is_none());
    }

    #[test]
    fn display_path_strips_data_prefix() {
        assert_eq!(display_path(DATA_ROOT, ""), "/");
        assert_eq!(display_path(DATA_ROOT, "Users/x"), "/Users/x");
        assert_eq!(display_path("/tmp/r", "a"), "/tmp/r/a");
    }

    #[test]
    fn view_limits_depth_and_merges_small() {
        let d = tempfile::tempdir().unwrap();
        let r = d.path();
        write(&r.join("a/b/c/deep.bin"), 3 << 20);
        let prog = Progress::new();
        let root = scan(r, &prog, &|_| false).unwrap();
        let snap = Snapshot {
            root,
            display_prefix: r.to_string_lossy().into(),
            system_used: 0,
            hidden: 0,
        };
        let v = view(&snap, "a", 1).unwrap();
        assert_eq!(v.children.len(), 1);
        assert!(v.children[0].children.is_empty());
        assert!(view(&snap, "missing", 2).is_none());
    }
}
