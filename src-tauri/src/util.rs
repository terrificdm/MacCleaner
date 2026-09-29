use rayon::prelude::*;
use std::fs::{self, Metadata};
use std::os::unix::fs::MetadataExt;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

/// Global cancel flag shared by the module scans (the disk scan has its own).
pub static CANCEL: AtomicBool = AtomicBool::new(false);

pub fn cancelled() -> bool {
    CANCEL.load(Ordering::Relaxed)
}

pub fn home() -> PathBuf {
    dirs::home_dir().expect("home directory")
}

pub fn expand(p: &str) -> PathBuf {
    if p == "~" {
        home()
    } else if let Some(rest) = p.strip_prefix("~/") {
        home().join(rest)
    } else {
        PathBuf::from(p)
    }
}

pub fn path_str(p: &Path) -> String {
    p.to_string_lossy().into_owned()
}

pub fn file_name(p: &Path) -> String {
    p.file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| path_str(p))
}

pub fn now_ms() -> i64 {
    to_ms(SystemTime::now()).unwrap_or(0)
}

pub fn to_ms(t: SystemTime) -> Option<i64> {
    t.duration_since(UNIX_EPOCH).ok().map(|d| d.as_millis() as i64)
}

pub fn mtime_ms(m: &Metadata) -> Option<i64> {
    m.modified().ok().and_then(to_ms)
}

/// Space actually allocated on disk (not the logical length).
pub fn alloc_size(m: &Metadata) -> u64 {
    m.blocks() * 512
}

/// Allocated size of a file or directory tree. Does not follow symlinks and
/// does not cross into other volumes.
pub fn path_size(p: &Path) -> u64 {
    match fs::symlink_metadata(p) {
        Ok(m) if m.is_dir() => alloc_size(&m) + dir_size(p, m.dev()),
        Ok(m) => alloc_size(&m),
        Err(_) => 0,
    }
}

fn dir_size(p: &Path, dev: u64) -> u64 {
    let entries: Vec<_> = match fs::read_dir(p) {
        Ok(rd) => rd.flatten().collect(),
        Err(_) => return 0,
    };
    entries
        .par_iter()
        .map(|e| match e.metadata() {
            Ok(m) if m.is_dir() => {
                if m.dev() == dev {
                    alloc_size(&m) + dir_size(&e.path(), dev)
                } else {
                    0
                }
            }
            Ok(m) => alloc_size(&m),
            Err(_) => 0,
        })
        .sum()
}

/// Sockets, pipes and device files: never offered for cleaning.
pub fn is_special(m: &Metadata) -> bool {
    use std::os::unix::fs::FileTypeExt;
    let t = m.file_type();
    t.is_socket() || t.is_fifo() || t.is_char_device() || t.is_block_device()
}

/// "Locked" in Finder (user immutable) or system immutable.
pub fn is_locked(m: &Metadata) -> bool {
    use std::os::macos::fs::MetadataExt as _;
    m.st_flags() & (libc::UF_IMMUTABLE as u32 | libc::SF_IMMUTABLE as u32) != 0
}

/// Directory entries sorted by name; unreadable directories yield nothing.
pub fn list_dir(p: &Path) -> Vec<PathBuf> {
    let mut v: Vec<PathBuf> = match fs::read_dir(p) {
        Ok(rd) => rd.flatten().map(|e| e.path()).collect(),
        Err(_) => Vec::new(),
    };
    v.sort();
    v
}

/// GUI apps launched from Finder get a minimal PATH, so resolve CLI tools
/// by absolute path.
pub fn find_tool(name: &str) -> Option<PathBuf> {
    let mut candidates = vec![
        PathBuf::from("/opt/homebrew/bin").join(name),
        PathBuf::from("/usr/local/bin").join(name),
        PathBuf::from("/usr/bin").join(name),
    ];
    if name == "docker" {
        candidates.push(PathBuf::from(
            "/Applications/Docker.app/Contents/Resources/bin/docker",
        ));
        candidates.push(home().join(".orbstack/bin/docker"));
    }
    candidates.into_iter().find(|p| p.is_file())
}

pub fn tool_path_env() -> String {
    "/opt/homebrew/bin:/opt/homebrew/sbin:/usr/local/bin:/usr/bin:/bin:/usr/sbin:/sbin".into()
}

pub struct CmdOut {
    pub ok: bool,
    pub stdout: String,
    pub stderr: String,
}

pub fn run(program: &Path, args: &[&str]) -> std::io::Result<CmdOut> {
    let out = Command::new(program)
        .args(args)
        .env("PATH", tool_path_env())
        .env("HOMEBREW_NO_AUTO_UPDATE", "1")
        .env("HOMEBREW_NO_ENV_HINTS", "1")
        .output()?;
    Ok(CmdOut {
        ok: out.status.success(),
        stdout: String::from_utf8_lossy(&out.stdout).into_owned(),
        stderr: String::from_utf8_lossy(&out.stderr).into_owned(),
    })
}

/// Last `n` characters of a string, for error details.
pub fn tail(s: &str, n: usize) -> String {
    let chars: Vec<char> = s.trim().chars().collect();
    let start = chars.len().saturating_sub(n);
    chars[start..].iter().collect()
}

/// Directories that are one unit: deleting a file inside corrupts the app,
/// library, document or project. Module scanners don't descend into them;
/// Space Lens counts their size; the safety check refuses anything inside.
pub const PACKAGE_EXT: &[&str] = &[
    ".app", ".framework", ".bundle", ".appex", ".xpc", ".plugin", ".kext",
    ".systemextension", ".xcarchive", ".photoslibrary", ".photolibrary",
    ".migratedphotolibrary", ".aplibrary", ".musiclibrary", ".tvlibrary",
    ".imovielibrary", ".fcpbundle", ".logicx", ".band", ".sparsebundle",
    ".rtfd", ".pages", ".key", ".numbers", ".xcodeproj", ".xcworkspace",
    ".playground",
];

pub fn is_package(name: &str) -> bool {
    let lower = name.to_lowercase();
    PACKAGE_EXT.iter().any(|e| lower.len() > e.len() && lower.ends_with(e))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn size_counts_nested_files() {
        let dir = tempfile::tempdir().unwrap();
        fs::create_dir_all(dir.path().join("a/b")).unwrap();
        let mut f = fs::File::create(dir.path().join("a/b/x.bin")).unwrap();
        f.write_all(&vec![1u8; 100_000]).unwrap();
        drop(f);
        let s = path_size(dir.path());
        assert!(s >= 100_000, "size {s}");
    }

    #[test]
    fn recognises_special_and_locked_files() {
        let d = tempfile::tempdir().unwrap();
        let sock = d.path().join("s.sock");
        let _l = std::os::unix::net::UnixListener::bind(&sock).unwrap();
        assert!(is_special(&fs::symlink_metadata(&sock).unwrap()));
        let f = d.path().join("f.txt");
        fs::write(&f, b"x").unwrap();
        assert!(!is_special(&fs::symlink_metadata(&f).unwrap()));
        assert!(!is_locked(&fs::symlink_metadata(&f).unwrap()));
        let c = std::ffi::CString::new(f.to_string_lossy().as_bytes()).unwrap();
        unsafe { libc::chflags(c.as_ptr(), libc::UF_IMMUTABLE as _) };
        assert!(is_locked(&fs::symlink_metadata(&f).unwrap()));
        unsafe { libc::chflags(c.as_ptr(), 0) };
    }

    #[test]
    fn expand_tilde() {
        assert_eq!(expand("~"), home());
        assert_eq!(expand("~/x"), home().join("x"));
        assert_eq!(expand("/tmp"), PathBuf::from("/tmp"));
    }
}
