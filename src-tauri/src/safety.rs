//! Decides whether a path may be moved to the Trash. Every file or folder
//! MacCleaner moves goes through `check_deletable` right before acting.
//! Command items (brew cleanup, docker prune) and Empty Trash don't move
//! paths; they are separate, permanent, and confirmed separately.

use std::fs;
use std::path::{Component, Path, PathBuf};
use std::sync::OnceLock;
use unicode_normalization::UnicodeNormalization;

/// Path components normalised the way APFS compares names by default:
/// case-insensitive and Unicode-normalisation-insensitive.
fn norm(p: &Path) -> Vec<String> {
    p.components()
        .map(|c| c.as_os_str().to_string_lossy().nfc().collect::<String>().to_lowercase())
        .collect()
}

/// `p` equals `base` or lies below it, ignoring case and Unicode form.
fn within(p: &Path, base: &Path) -> bool {
    let (p, b) = (norm(p), norm(base));
    p.len() >= b.len() && p[..b.len()] == b[..]
}

fn inside_unit(p: &Path) -> bool {
    let comps = norm(p);
    let Some((last, parents)) = comps.split_last() else { return false };
    last == ".git" || parents.iter().any(|c| c == ".git" || crate::util::is_package(c))
}

/// The .app bundle containing an executable, if any.
pub fn bundle_of(exe: &Path) -> Option<PathBuf> {
    exe.ancestors()
        .find(|a| a.extension().map(|e| e.eq_ignore_ascii_case("app")).unwrap_or(false))
        .map(Path::to_path_buf)
}

pub fn is_own_bundle(p: &Path, own: Option<&Path>) -> bool {
    own.map(|o| within(p, o) || within(o, p)).unwrap_or(false)
}

fn own_bundle() -> Option<&'static Path> {
    static OWN: OnceLock<Option<PathBuf>> = OnceLock::new();
    OWN.get_or_init(|| std::env::current_exe().ok().and_then(|e| bundle_of(&e)))
        .as_deref()
}

pub struct Ctx {
    pub home: PathBuf,
    pub tmp: Option<PathBuf>,
    pub whitelist: Vec<PathBuf>,
}

impl Ctx {
    pub fn current() -> Ctx {
        let tmp = std::env::var("TMPDIR")
            .ok()
            .and_then(|t| fs::canonicalize(t).ok());
        Ctx {
            home: crate::util::home(),
            tmp,
            whitelist: crate::settings::get()
                .whitelist
                .iter()
                .map(|p| crate::util::expand(p))
                .collect(),
        }
    }

    /// Hard-coded subtrees that are never deleted: the OS itself, keychains,
    /// privacy databases and MacCleaner's own data. Removing anything here
    /// breaks macOS or the app rather than freeing space. Personal data
    /// (Documents, Mail, iCloud...) lives in the editable default whitelist.
    fn protected(&self) -> Vec<PathBuf> {
        let h = &self.home;
        let mut v: Vec<PathBuf> = [
            "/System", "/bin", "/sbin", "/usr", "/cores", "/Library/Apple",
            "/Library/Keychains", "/Library/Application Support/Apple",
            "/Library/Application Support/com.apple.TCC",
        ]
        .iter()
        .map(PathBuf::from)
        .collect();
        for rel in [
            "Library/Keychains",
            "Library/Application Support/MacCleaner",
            "Library/Application Support/com.apple.TCC",
        ] {
            v.push(h.join(rel));
        }
        v
    }

    /// Roots whose *contents* may be cleaned. The roots themselves never can.
    fn allowed_roots(&self) -> Vec<PathBuf> {
        let mut v = vec![self.home.clone(), PathBuf::from("/Applications")];
        for r in [
            "Application Support", "LaunchAgents", "LaunchDaemons",
            "PrivilegedHelperTools", "Preferences", "Caches", "Logs",
        ] {
            v.push(Path::new("/Library").join(r));
        }
        if let Some(t) = &self.tmp {
            v.push(t.clone());
        }
        v
    }

    /// True if the path falls in a protected subtree or the whitelist. Used
    /// by module scanners to skip paths entirely.
    pub fn is_excluded(&self, p: &Path) -> bool {
        self.protected().iter().any(|x| within(p, x))
            || self.whitelist.iter().any(|w| within(p, w))
            || apple_owned(p, &self.home)
            || has_protected_package(p)
    }
}

/// Apple data inside per-app Library folders (Notes, Reminders, prefs...).
fn apple_owned(p: &Path, home: &Path) -> bool {
    let pc = norm(p);
    for d in [
        "Preferences", "Application Support", "Containers", "Group Containers",
        "Saved Application State", "LaunchAgents", "Application Scripts",
    ] {
        let base = norm(&home.join("Library").join(d));
        if pc.len() > base.len() && pc[..base.len()] == base[..] {
            let name = &pc[base.len()];
            // Shortcuts still uses its pre-Apple "is.workflow" identifiers.
            if name.contains("com.apple.") || name.starts_with("group.com.apple") || name.contains("is.workflow.") {
                return true;
            }
        }
    }
    false
}

fn has_protected_package(p: &Path) -> bool {
    p.components().any(|c| {
        let n = c.as_os_str().to_string_lossy().to_ascii_lowercase();
        n.ends_with(".photoslibrary") || n.ends_with(".photolibrary")
            || n.ends_with(".musiclibrary") || n.ends_with(".tvlibrary")
    })
}

fn is_normalized_absolute(p: &Path) -> bool {
    p.is_absolute()
        && p.components()
            .all(|c| matches!(c, Component::RootDir | Component::Normal(_)))
}

/// Resolve symlinks in the parent while keeping the final component, so a
/// symlink itself is trashed rather than its target.
fn resolve_parent(p: &Path) -> PathBuf {
    match (p.parent(), p.file_name()) {
        (Some(parent), Some(name)) => fs::canonicalize(parent)
            .map(|c| c.join(name))
            .unwrap_or_else(|_| p.to_path_buf()),
        _ => p.to_path_buf(),
    }
}

/// Error codes are translated by the frontend (`errors.<code>`).
pub fn check_deletable(p: &Path, ctx: &Ctx) -> Result<(), &'static str> {
    if !is_normalized_absolute(p) {
        return Err("notAbsolute");
    }
    let meta = fs::symlink_metadata(p).map_err(|_| "notFound")?;
    let resolved = resolve_parent(p);
    for cand in [p, resolved.as_path()] {
        check_one(cand, meta.is_dir(), ctx)?;
    }
    if meta.file_type().is_symlink() {
        if let Ok(target) = fs::canonicalize(p) {
            if ctx.protected().iter().any(|x| target.starts_with(x))
                || target.starts_with("/System")
            {
                return Err("protected");
            }
        }
    }
    Ok(())
}

fn check_one(p: &Path, is_dir: bool, ctx: &Ctx) -> Result<(), &'static str> {
    let protected = ctx.protected();
    // Inside, equal to, or an ancestor of a protected path.
    if protected.iter().any(|x| within(p, x) || within(x, p)) {
        return Err("protected");
    }
    if apple_owned(p, &ctx.home) || has_protected_package(p) {
        return Err("protected");
    }
    if is_own_bundle(p, own_bundle()) {
        return Err("self");
    }
    if inside_unit(p) {
        return Err("insidePackage");
    }
    if ctx.whitelist.iter().any(|w| within(p, w) || within(w, p)) {
        return Err("whitelisted");
    }
    if p.starts_with(ctx.home.join(".Trash")) {
        return Err("inTrash");
    }
    let roots = ctx.allowed_roots();
    let root = roots
        .iter()
        .filter(|r| p.starts_with(r) && p != r.as_path())
        .max_by_key(|r| r.components().count())
        .ok_or("outsideAllowed")?;
    let depth = p.strip_prefix(root).map(|r| r.components().count()).unwrap_or(0);
    if root == Path::new("/Applications") {
        // Only app bundles (possibly inside a vendor folder), never folders
        // like /Applications/Utilities themselves.
        let is_app = p
            .strip_prefix(root)
            .map(|r| {
                r.components()
                    .any(|c| c.as_os_str().to_string_lossy().to_ascii_lowercase().ends_with(".app"))
            })
            .unwrap_or(false);
        if !is_app {
            return Err("protected");
        }
    }
    if root == &ctx.home {
        // ~/Library/<standard folder> and ~/<folder> are containers, never targets.
        if p.starts_with(ctx.home.join("Library")) {
            if depth < 3 {
                return Err("tooShallow");
            }
        } else if depth < 2 && is_dir {
            return Err("tooShallow");
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ctx(home: &Path) -> Ctx {
        Ctx { home: home.to_path_buf(), tmp: None, whitelist: vec![] }
    }

    fn mk(p: &Path) {
        fs::create_dir_all(p).unwrap();
    }

    #[test]
    fn rejects_relative_and_dotdot() {
        let d = tempfile::tempdir().unwrap();
        let h = fs::canonicalize(d.path()).unwrap();
        let c = ctx(&h);
        assert_eq!(check_deletable(Path::new("Library/Caches/x"), &c), Err("notAbsolute"));
        mk(&h.join("Library/Caches/x"));
        let dd = h.join("Library/Caches/x/../../../Documents");
        assert_eq!(check_deletable(&dd, &c), Err("notAbsolute"));
    }

    #[test]
    fn protects_keychains_and_system_hard() {
        let d = tempfile::tempdir().unwrap();
        let h = fs::canonicalize(d.path()).unwrap();
        let c = ctx(&h);
        for rel in ["Library/Keychains/login.keychain-db", "Library/Application Support/MacCleaner/settings.json"] {
            let p = h.join(rel);
            mk(&p);
            assert_eq!(check_deletable(&p, &c), Err("protected"), "{rel}");
        }
    }

    #[test]
    fn documents_protected_only_by_default_whitelist() {
        let d = tempfile::tempdir().unwrap();
        let h = fs::canonicalize(d.path()).unwrap();
        let doc = h.join("Documents/old.mov");
        mk(&doc);
        let mut c = ctx(&h);
        c.whitelist = crate::settings::DEFAULT_WHITELIST
            .iter()
            .map(|p| h.join(p.trim_start_matches("~/")))
            .collect();
        assert_eq!(check_deletable(&doc, &c), Err("whitelisted"));
        assert!(c.is_excluded(&doc));
        let ssh = h.join(".ssh/id_rsa");
        mk(&ssh);
        assert_eq!(check_deletable(&ssh, &c), Err("whitelisted"));
        // Once the user removes ~/Documents from the whitelist it is allowed.
        c.whitelist.retain(|w| !w.ends_with("Documents"));
        assert_eq!(check_deletable(&doc, &c), Ok(()));
        assert!(!c.is_excluded(&doc));
    }

    #[test]
    fn rejects_ancestors_of_protected_and_containers() {
        let d = tempfile::tempdir().unwrap();
        let h = fs::canonicalize(d.path()).unwrap();
        let c = ctx(&h);
        mk(&h.join("Library/Caches"));
        mk(&h.join("Library/Keychains"));
        mk(&h.join("Downloads"));
        // ~/Library is an ancestor of ~/Library/Keychains.
        assert_eq!(check_deletable(&h.join("Library"), &c), Err("protected"));
        assert_eq!(check_deletable(&h.join("Library/Caches"), &c), Err("tooShallow"));
        assert_eq!(check_deletable(&h.join("Downloads"), &c), Err("tooShallow"));
    }

    #[test]
    fn allows_normal_targets() {
        let d = tempfile::tempdir().unwrap();
        let h = fs::canonicalize(d.path()).unwrap();
        let c = ctx(&h);
        let cache = h.join("Library/Caches/com.example.app");
        mk(&cache);
        assert_eq!(check_deletable(&cache, &c), Ok(()));
        let dl = h.join("Downloads/big.dmg");
        mk(dl.parent().unwrap());
        fs::write(&dl, b"x").unwrap();
        assert_eq!(check_deletable(&dl, &c), Ok(()));
        let nm = h.join("Projects/web/node_modules");
        mk(&nm);
        assert_eq!(check_deletable(&nm, &c), Ok(()));
        let top_file = h.join("big.iso");
        fs::write(&top_file, b"x").unwrap();
        assert_eq!(check_deletable(&top_file, &c), Ok(()));
    }

    #[test]
    fn rejects_apple_data_and_photos() {
        let d = tempfile::tempdir().unwrap();
        let h = fs::canonicalize(d.path()).unwrap();
        let c = ctx(&h);
        let notes = h.join("Library/Group Containers/group.com.apple.notes");
        mk(&notes);
        assert_eq!(check_deletable(&notes, &c), Err("protected"));
        let pref = h.join("Library/Preferences/com.apple.finder.plist");
        mk(pref.parent().unwrap());
        fs::write(&pref, b"x").unwrap();
        assert_eq!(check_deletable(&pref, &c), Err("protected"));
        let lib = h.join("Pictures/Photos Library.photoslibrary/originals");
        mk(&lib);
        assert_eq!(check_deletable(&lib, &c), Err("protected"));
    }

    #[test]
    fn applications_only_app_bundles() {
        let c = Ctx { home: PathBuf::from("/nonexistent-home"), tmp: None, whitelist: vec![] };
        assert_eq!(check_one(Path::new("/Applications/Utilities"), true, &c), Err("protected"));
        assert_eq!(check_one(Path::new("/Applications/Cisco"), true, &c), Err("protected"));
        assert_eq!(check_one(Path::new("/Applications/Foo.app"), true, &c), Ok(()));
        assert_eq!(check_one(Path::new("/Applications/Cisco/Client.app"), true, &c), Ok(()));
    }

    #[test]
    fn refuses_git_internals_but_allows_whole_project() {
        let d = tempfile::tempdir().unwrap();
        let h = fs::canonicalize(d.path()).unwrap();
        let c = ctx(&h);
        let pack = h.join("temp/repo/.git/objects/pack/pack-1.pack");
        mk(&pack);
        assert_eq!(check_deletable(&pack, &c), Err("insidePackage"));
        assert_eq!(check_deletable(&h.join("temp/repo/.git"), &c), Err("insidePackage"));
        assert_eq!(check_deletable(&h.join("temp/repo"), &c), Ok(()));
    }

    #[test]
    fn refuses_files_inside_app_bundles() {
        let c = Ctx { home: PathBuf::from("/nonexistent-home"), tmp: None, whitelist: vec![] };
        assert_eq!(check_one(Path::new("/Applications/Foo.app/Contents/Frameworks/big.dylib"), false, &c), Err("insidePackage"));
        assert_eq!(check_one(Path::new("/Applications/Foo.app"), true, &c), Ok(()));
        let d = tempfile::tempdir().unwrap();
        let h = fs::canonicalize(d.path()).unwrap();
        let fw = h.join("temp/Lib.framework/Versions/A/Lib");
        mk(&fw);
        assert_eq!(check_deletable(&fw, &ctx(&h)), Err("insidePackage"));
    }

    #[test]
    fn finds_own_bundle_from_executable_path() {
        assert_eq!(
            bundle_of(Path::new("/Applications/MacCleaner.app/Contents/MacOS/maccleaner")),
            Some(PathBuf::from("/Applications/MacCleaner.app"))
        );
        assert_eq!(bundle_of(Path::new("/Users/x/proj/target/debug/maccleaner")), None);
        let own = PathBuf::from("/Applications/MacCleaner.app");
        assert!(is_own_bundle(&own, Some(&own)));
        assert!(is_own_bundle(&own.join("Contents/MacOS/maccleaner"), Some(&own)));
        assert!(!is_own_bundle(Path::new("/Applications/Other.app"), Some(&own)));
    }

    #[test]
    fn whitelist_and_protection_ignore_case_and_unicode_form() {
        let d = tempfile::tempdir().unwrap();
        let h = fs::canonicalize(d.path()).unwrap();
        let doc = h.join("Documents/old.mov");
        mk(&doc);
        let mut c = ctx(&h);
        c.whitelist = vec![h.join("documents")];
        assert_eq!(check_deletable(&doc, &c), Err("whitelisted"));
        assert!(c.is_excluded(&doc));
        // "Café" typed composed (NFC) vs stored decomposed (NFD).
        let nfd = h.join("Projects/Cafe\u{301}/x.bin");
        mk(&nfd);
        c.whitelist = vec![h.join("Projects/Caf\u{e9}")];
        assert_eq!(check_deletable(&nfd, &c), Err("whitelisted"));
        // Hard-coded protection must not be bypassed by case either.
        let kc = h.join("Library/Keychains/x");
        mk(&kc);
        let upper = PathBuf::from(h.to_string_lossy().to_string()).join("LIBRARY/keychains/x");
        c.whitelist = vec![];
        assert_eq!(check_one(&upper, false, &c), Err("protected"));
    }

    #[test]
    fn protects_apple_scripts_and_shortcuts_data() {
        let d = tempfile::tempdir().unwrap();
        let h = fs::canonicalize(d.path()).unwrap();
        let c = ctx(&h);
        for rel in [
            "Library/Application Scripts/group.com.apple.notes",
            "Library/Application Scripts/group.is.workflow.shortcuts",
            "Library/Group Containers/group.is.workflow.my.app",
        ] {
            let p = h.join(rel);
            mk(&p);
            assert_eq!(check_deletable(&p, &c), Err("protected"), "{rel}");
        }
    }

    #[test]
    fn whitelist_blocks_path_and_ancestors() {
        let d = tempfile::tempdir().unwrap();
        let h = fs::canonicalize(d.path()).unwrap();
        let mut c = ctx(&h);
        let keep = h.join("Projects/keep/node_modules");
        mk(&keep);
        c.whitelist = vec![h.join("Projects/keep")];
        assert_eq!(check_deletable(&keep, &c), Err("whitelisted"));
        assert_eq!(check_deletable(&h.join("Projects"), &c), Err("whitelisted"));
    }

    #[test]
    fn rejects_outside_roots_and_trash() {
        let d = tempfile::tempdir().unwrap();
        let h = fs::canonicalize(d.path()).unwrap();
        let c = ctx(&h);
        assert_eq!(check_deletable(Path::new("/etc/hosts"), &c), Err("outsideAllowed"));
        assert_eq!(check_deletable(Path::new("/System/Library"), &c), Err("protected"));
        let t = h.join(".Trash/x");
        mk(&t);
        assert_eq!(check_deletable(&t, &c), Err("inTrash"));
        assert_eq!(check_deletable(&h.join("nope"), &c), Err("notFound"));
    }

    #[test]
    fn symlinked_parent_is_resolved() {
        let d = tempfile::tempdir().unwrap();
        let h = fs::canonicalize(d.path()).unwrap();
        let c = ctx(&h);
        mk(&h.join("Library/Keychains/secret"));
        mk(&h.join("Projects"));
        std::os::unix::fs::symlink(h.join("Library/Keychains"), h.join("Projects/link")).unwrap();
        let via_link = h.join("Projects/link/secret");
        assert_eq!(check_deletable(&via_link, &c), Err("protected"));
    }
}
