//! Files currently held open by the user's processes (via lsof).

use crate::model::Item;
use std::path::Path;
use std::process::Command;

pub struct InUse {
    /// Sorted absolute paths of open files.
    paths: Vec<String>,
}

impl InUse {
    /// Parses `lsof -F n` output: only `n` lines holding absolute paths.
    pub fn from_lsof(out: &str) -> InUse {
        let mut paths: Vec<String> = out
            .lines()
            .filter_map(|l| l.strip_prefix('n'))
            .filter(|n| n.starts_with('/'))
            .map(str::to_string)
            .collect();
        paths.sort();
        paths.dedup();
        InUse { paths }
    }

    /// Open files of all processes owned by the current user. If lsof is
    /// unavailable the set is empty, and the other safety checks still apply.
    pub fn current() -> InUse {
        let uid = unsafe { libc::getuid() }.to_string();
        let out = Command::new("/usr/sbin/lsof")
            .args(["-n", "-w", "-u", &uid, "-F", "n"])
            .output();
        match out {
            Ok(o) => InUse::from_lsof(&String::from_utf8_lossy(&o.stdout)),
            Err(_) => InUse { paths: Vec::new() },
        }
    }

    /// True if `p` itself, or anything below it, is open.
    pub fn contains_under(&self, p: &Path) -> bool {
        let base = p.to_string_lossy();
        let base = base.trim_end_matches('/');
        let start = self.paths.partition_point(|x| x.as_str() < base);
        self.paths[start..]
            .iter()
            .take_while(|x| x.starts_with(base))
            .any(|x| x.len() == base.len() || x.as_bytes()[base.len()] == b'/')
    }

    /// Deselect scan results that are in use and say why.
    pub fn mark(&self, items: &mut [Item]) {
        for it in items.iter_mut() {
            let locked = it.path.starts_with('/')
                && std::fs::symlink_metadata(&it.path).map(|m| crate::util::is_locked(&m)).unwrap_or(false);
            if locked {
                it.selected = false;
                it.note = Some("locked".into());
                continue;
            }
            if it.kind != "command" && it.path.starts_with('/') && self.contains_under(Path::new(&it.path)) {
                it.selected = false;
                if it.note.as_deref() != Some("running") {
                    it.note = Some("inUse".into());
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn parses_lsof_names_and_matches_prefixes() {
        let out = "p123\nfcwd\nn/Users/x/Library/Caches/colima/disk.img\nn->0x1234\nnlocalhost:443\nn/Users/x/Library/Caches/colima-other/a\n";
        let u = InUse::from_lsof(out);
        assert!(u.contains_under(Path::new("/Users/x/Library/Caches/colima")));
        assert!(u.contains_under(Path::new("/Users/x/Library/Caches/colima/disk.img")));
        assert!(!u.contains_under(Path::new("/Users/x/Library/Caches/col")));
        assert!(!u.contains_under(Path::new("/Users/x/Library/Caches/colima/other")));
        assert!(u.contains_under(Path::new("/Users/x/Library/Caches/colima-other")));
    }

    #[test]
    fn sees_a_file_this_process_holds_open() {
        let d = tempfile::tempdir().unwrap();
        let dir = std::fs::canonicalize(d.path()).unwrap();
        let f: PathBuf = dir.join("held.bin");
        let _h = std::fs::File::create(&f).unwrap();
        let u = InUse::current();
        assert!(u.contains_under(&f));
        assert!(u.contains_under(&dir));
    }
}
