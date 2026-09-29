mod apps;
mod cleaner;
mod disk;
mod history;
mod inuse;
mod installer;
mod junk;
mod mac;
mod model;
mod safety;
mod settings;
mod util;

use model::{CleanResult, CleanTarget, Item};
use serde::Serialize;
use settings::Settings;
use std::path::Path;
use std::sync::atomic::Ordering;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tauri::menu::{Menu, PredefinedMenuItem, Submenu};
use tauri::{AppHandle, Emitter, Runtime};

type Res<T> = Result<T, String>;

/// Must match `identifier` in tauri.conf.json.
pub const APP_ID: &str = "app.maccleaner.MacCleaner";

async fn blocking<T: Send + 'static>(f: impl FnOnce() -> T + Send + 'static) -> Res<T> {
    tauri::async_runtime::spawn_blocking(f).await.map_err(|e| e.to_string())
}

/// Module scans share one cancel flag; reset it when a new scan starts.
async fn module_scan<T: Send + 'static>(f: impl FnOnce() -> T + Send + 'static) -> Res<T> {
    util::CANCEL.store(false, Ordering::Relaxed);
    let r = blocking(f).await?;
    if util::cancelled() {
        return Err("cancelled".into());
    }
    Ok(r)
}

#[tauri::command]
fn get_settings() -> Settings {
    settings::get()
}

#[tauri::command]
fn save_settings<R: Runtime>(app: AppHandle<R>, settings: Settings) -> Res<Settings> {
    let lang_changed = settings.language != settings::get().language;
    let s = settings::save(settings)?;
    if lang_changed {
        let _ = set_menu(&app, &s.language);
    }
    Ok(s)
}

#[tauri::command]
fn add_whitelist(path: String) -> Res<Settings> {
    let mut s = settings::get();
    s.whitelist.push(path);
    settings::save(s)
}

#[tauri::command]
fn default_whitelist() -> Vec<String> {
    settings::DEFAULT_WHITELIST.iter().map(|s| s.to_string()).collect()
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct Perms {
    full_disk_access: bool,
}

#[tauri::command]
fn permissions() -> Perms {
    Perms { full_disk_access: mac::has_full_disk_access() }
}

#[tauri::command]
fn open_fda_settings() {
    mac::open_fda_settings();
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct Volume {
    name: String,
    used: u64,
    total: u64,
    available: u64,
}

#[tauri::command]
fn volume_info() -> Volume {
    let (used, total, available) = disk::volume_usage(disk::DATA_ROOT);
    Volume { name: disk::volume_name(), used, total, available }
}

#[tauri::command]
async fn scan_system_junk() -> Res<Vec<Item>> {
    module_scan(junk::system_junk).await
}

#[tauri::command]
async fn scan_dev_junk() -> Res<Vec<Item>> {
    let s = settings::get();
    module_scan(move || junk::dev_junk(&s.node_modules_roots, s.node_modules_days)).await
}

#[tauri::command]
async fn scan_large_files() -> Res<Vec<Item>> {
    module_scan(junk::large_files).await
}

#[tauri::command]
async fn scan_trash() -> Res<Vec<Item>> {
    module_scan(junk::trash_items).await
}

#[tauri::command]
async fn scan_downloads() -> Res<Vec<Item>> {
    module_scan(junk::downloads).await
}

#[tauri::command]
async fn scan_leftovers() -> Res<Vec<Item>> {
    module_scan(apps::leftovers).await
}

#[tauri::command]
async fn list_apps() -> Res<Vec<apps::AppInfo>> {
    blocking(apps::list_apps).await
}

#[tauri::command]
async fn app_icon(path: String) -> Res<Option<String>> {
    blocking(move || apps::icon(&path)).await
}

#[tauri::command]
async fn app_related(bundle_id: String, name: String, path: String) -> Res<Vec<Item>> {
    blocking(move || apps::related(&bundle_id, &name, &path)).await
}

static DISK_PROGRESS: Mutex<Option<Arc<disk::Progress>>> = Mutex::new(None);

#[tauri::command]
fn cancel_scans() {
    util::CANCEL.store(true, Ordering::Relaxed);
    if let Some(p) = DISK_PROGRESS.lock().unwrap().as_ref() {
        p.cancel.store(true, Ordering::Relaxed);
    }
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
struct DiskProgress {
    files: u64,
    bytes: u64,
    current: String,
    estimate: u64,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct DiskSummary {
    total: u64,
    scanned: u64,
    files: u64,
    denied: u64,
    duration_ms: u64,
}

#[tauri::command]
async fn disk_scan<R: Runtime>(app: AppHandle<R>) -> Res<DiskSummary> {
    let prog = Arc::new(disk::Progress::new());
    *DISK_PROGRESS.lock().unwrap() = Some(prog.clone());
    let (data_used, _, _) = disk::volume_usage(disk::DATA_ROOT);
    let (sys_used, _, _) = disk::volume_usage("/");

    let finished = Arc::new(std::sync::atomic::AtomicBool::new(false));
    let ticker = {
        let prog = prog.clone();
        let app = app.clone();
        let finished = finished.clone();
        std::thread::spawn(move || loop {
            std::thread::sleep(Duration::from_millis(200));
            let done = finished.load(Ordering::Relaxed);
            let _ = app.emit(
                "disk-progress",
                DiskProgress {
                    files: prog.files.load(Ordering::Relaxed),
                    bytes: prog.bytes.load(Ordering::Relaxed),
                    current: prog.current.lock().map(|c| c.clone()).unwrap_or_default(),
                    estimate: data_used,
                },
            );
            if done || prog.cancel.load(Ordering::Relaxed) {
                break;
            }
        })
    };

    let start = Instant::now();
    let p2 = prog.clone();
    let result = blocking(move || {
        let excluded = disk::scan_exclusions();
        disk::scan(Path::new(disk::DATA_ROOT), &p2, &excluded)
    })
    .await?;
    finished.store(true, Ordering::Relaxed);
    *DISK_PROGRESS.lock().unwrap() = None;
    let _ = ticker.join();

    let root = result.ok_or("cancelled")?;
    let scanned = root.size;
    let denied = prog.denied.load(Ordering::Relaxed);
    let summary = DiskSummary {
        total: scanned.max(data_used) + sys_used,
        scanned,
        files: prog.files.load(Ordering::Relaxed),
        denied,
        duration_ms: start.elapsed().as_millis() as u64,
    };
    *disk::SNAPSHOT.lock().unwrap() = Some(Arc::new(disk::Snapshot {
        root,
        display_prefix: disk::DATA_ROOT.into(),
        system_used: sys_used,
        hidden: data_used.saturating_sub(scanned),
    }));
    Ok(summary)
}

#[tauri::command]
fn disk_view(path: String, depth: usize) -> Res<disk::ViewNode> {
    let snap = disk::SNAPSHOT.lock().unwrap().clone().ok_or("noScan")?;
    let rel = path.trim_start_matches('/');
    disk::view(&snap, rel, depth.clamp(1, 5)).ok_or_else(|| "notFound".into())
}

#[tauri::command]
async fn clean(module: String, targets: Vec<CleanTarget>) -> Res<CleanResult> {
    let dry = settings::dry_run();
    let r = blocking(move || cleaner::clean(targets, dry)).await?;
    history::append(&module, &r);
    Ok(r)
}

#[tauri::command]
async fn empty_trash() -> Res<CleanResult> {
    let dry = settings::dry_run();
    let r = blocking(move || cleaner::empty_trash(dry)).await?;
    history::append("emptyTrash", &r);
    Ok(r)
}

#[tauri::command]
async fn uninstall(req: cleaner::UninstallRequest) -> Res<CleanResult> {
    let dry = settings::dry_run();
    let r = blocking(move || cleaner::uninstall(req, dry)).await??;
    history::append("uninstall", &r);
    Ok(r)
}

#[tauri::command]
fn install_status() -> installer::Status {
    installer::status()
}

/// Installs this copy into /Applications, then quits and reopens from there.
#[tauri::command]
async fn install_now<R: Runtime>(app: AppHandle<R>) -> Res<()> {
    let dest = blocking(installer::install).await??;
    installer::relaunch_after_exit(&dest);
    app.exit(0);
    Ok(())
}

/// Put everything a history record moved to the Trash back where it was.
#[tauri::command]
async fn restore(record_id: String) -> Res<CleanResult> {
    let rec = history::list().into_iter().find(|r| r.id == record_id).ok_or("notFound")?;
    let items: Vec<cleaner::RestoreItem> = rec
        .items
        .iter()
        .filter(|d| d.method == "trash" || d.method == "finder")
        // Items already put back or emptied from the Trash are not errors.
        .filter(|d| d.trash_path.as_deref().map(|t| std::fs::symlink_metadata(t).is_ok()).unwrap_or(true))
        .map(|d| cleaner::RestoreItem { path: d.path.clone(), trash_path: d.trash_path.clone() })
        .collect();
    let trash = util::home().join(".Trash");
    let r = blocking(move || cleaner::restore_from(items, &trash)).await?;
    history::append("restore", &r);
    Ok(r)
}

#[tauri::command]
fn history_list() -> Vec<history::Record> {
    history::list()
}

#[tauri::command]
fn history_clear() -> Res<()> {
    history::clear()
}

#[tauri::command]
fn reveal(path: String) {
    mac::reveal(Path::new(&path));
}

#[tauri::command]
fn quick_look(path: String) {
    mac::quick_look(Path::new(&path));
}

fn resolve_lang(lang: &str) -> &'static str {
    match lang {
        "zh-CN" => "zh",
        "en" => "en",
        _ => {
            let sys = std::process::Command::new("/usr/bin/defaults")
                .args(["read", "-g", "AppleLanguages"])
                .output()
                .map(|o| String::from_utf8_lossy(&o.stdout).to_string())
                .unwrap_or_default();
            let first = sys.lines().nth(1).unwrap_or("").trim().trim_matches(|c| c == '"' || c == ',');
            if first.starts_with("zh") { "zh" } else { "en" }
        }
    }
}

/// Menu titles come from the same translation files as the UI.
fn menu_text(zh: bool, key: &str) -> String {
    static ZH: std::sync::OnceLock<serde_json::Value> = std::sync::OnceLock::new();
    static EN: std::sync::OnceLock<serde_json::Value> = std::sync::OnceLock::new();
    let pick = |cell: &'static std::sync::OnceLock<serde_json::Value>, src: &str| {
        cell.get_or_init(|| serde_json::from_str(src).unwrap_or_default())
    };
    let v = if zh {
        pick(&ZH, include_str!("../../src/i18n/zh-CN.json"))
    } else {
        pick(&EN, include_str!("../../src/i18n/en.json"))
    };
    v["menu"][key].as_str().unwrap_or(key).to_string()
}

fn set_menu<R: Runtime>(app: &AppHandle<R>, lang: &str) -> tauri::Result<()> {
    let zh = resolve_lang(lang) == "zh";
    let t = |k: &str| Some(menu_text(zh, k));
    let app_menu = Submenu::with_items(
        app,
        "MacCleaner",
        true,
        &[
            &PredefinedMenuItem::about(app, t("about").as_deref(), None)?,
            &PredefinedMenuItem::separator(app)?,
            &PredefinedMenuItem::hide(app, t("hide").as_deref())?,
            &PredefinedMenuItem::hide_others(app, t("hideOthers").as_deref())?,
            &PredefinedMenuItem::show_all(app, t("showAll").as_deref())?,
            &PredefinedMenuItem::separator(app)?,
            &PredefinedMenuItem::quit(app, t("quit").as_deref())?,
        ],
    )?;
    let edit = Submenu::with_items(
        app,
        menu_text(zh, "edit"),
        true,
        &[
            &PredefinedMenuItem::undo(app, t("undo").as_deref())?,
            &PredefinedMenuItem::redo(app, t("redo").as_deref())?,
            &PredefinedMenuItem::separator(app)?,
            &PredefinedMenuItem::cut(app, t("cut").as_deref())?,
            &PredefinedMenuItem::copy(app, t("copy").as_deref())?,
            &PredefinedMenuItem::paste(app, t("paste").as_deref())?,
            &PredefinedMenuItem::select_all(app, t("selectAll").as_deref())?,
        ],
    )?;
    let window = Submenu::with_items(
        app,
        menu_text(zh, "window"),
        true,
        &[
            &PredefinedMenuItem::minimize(app, t("minimize").as_deref())?,
            &PredefinedMenuItem::maximize(app, t("zoom").as_deref())?,
            &PredefinedMenuItem::separator(app)?,
            &PredefinedMenuItem::close_window(app, t("close").as_deref())?,
        ],
    )?;
    let menu = Menu::with_items(app, &[&app_menu, &edit, &window])?;
    app.set_menu(menu)?;
    Ok(())
}

fn handlers<R: Runtime>() -> impl Fn(tauri::ipc::Invoke<R>) -> bool + Send + Sync + 'static {
    tauri::generate_handler![
        get_settings, save_settings, add_whitelist, default_whitelist, permissions, open_fda_settings,
        volume_info, scan_system_junk, scan_dev_junk, scan_large_files, scan_trash,
        scan_downloads, scan_leftovers, list_apps, app_icon, app_related, cancel_scans,
        disk_scan, disk_view, clean, empty_trash, uninstall, history_list, history_clear,
        reveal, quick_look, install_status, install_now, restore
    ]
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            let lang = settings::get().language;
            set_menu(app.handle(), &lang)?;
            Ok(())
        })
        .invoke_handler(handlers())
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

/// Read-only probe of the real machine: `cargo test real_probe -- --ignored --nocapture`.
#[cfg(test)]
mod probe {
    use super::*;

    fn gb(b: u64) -> String {
        format!("{:.2} GB", b as f64 / 1e9)
    }

    fn summarize(label: &str, items: &[Item]) {
        let total: u64 = items.iter().map(|i| i.size).sum();
        let sel: u64 = items.iter().filter(|i| i.selected).map(|i| i.size).sum();
        println!("== {label}: {} items, total {}, preselected {}", items.len(), gb(total), gb(sel));
        let mut groups: std::collections::BTreeMap<&str, (usize, u64)> = Default::default();
        for i in items {
            let e = groups.entry(i.group.as_str()).or_default();
            e.0 += 1;
            e.1 += i.size;
        }
        for (g, (n, s)) in groups.iter().take(12) {
            println!("   [{g}] {n} items {}", gb(*s));
        }
        for i in items.iter().take(6) {
            println!("   {} {} sel={} note={:?}", gb(i.size), i.path, i.selected, i.note);
        }
    }

    #[test]
    #[ignore]
    fn real_probe() {
        println!("FDA: {}", mac::has_full_disk_access());
        let t = Instant::now();
        summarize("system junk", &junk::system_junk());
        summarize("dev junk", &junk::dev_junk(&["~".into()], 90));
        let large = junk::large_files();
        summarize("large (>=50MB floor)", &large);
        let with_used = large.iter().filter(|i| i.last_used.is_some()).count();
        println!("   large files with Spotlight lastUsed: {with_used}/{}", large.len());
        summarize("downloads", &junk::downloads());
        summarize("trash", &junk::trash_items());
        println!("t={:?}", t.elapsed());
        let lo = apps::leftovers();
        summarize("leftovers", &lo);
        println!("t={:?}", t.elapsed());
        let apps = apps::list_apps();
        println!("t={:?} (apps listed)", t.elapsed());
        println!("== apps: {}", apps.len());
        for a in &apps {
            println!(
                "   {:<34} {:<45} {:>9} {:<8} used={} run={} blocked={:?} admin={}",
                a.name, a.bundle_id, gb(a.size), a.source, a.last_used.is_some(), a.running, a.blocked, a.needs_admin
            );
        }
        if let Some(a) = apps.iter().find(|a| a.name == "Discord").or(apps.first()) {
            let rel = apps::related(&a.bundle_id, &a.name, &a.path);
            summarize(&format!("related files of {}", a.name), &rel);
            let icon = apps::icon(&a.path);
            println!("   icon data url len: {:?}", icon.map(|s| s.len()));
        }
        println!("elapsed {:?}", t.elapsed());
    }

    #[test]
    #[ignore]
    fn large_probe() {
        let t = Instant::now();
        let items = junk::large_files();
        println!("large scan {:?}: {} files >= 50MB, {}", t.elapsed(), items.len(), gb(items.iter().map(|i| i.size).sum()));
        let home = util::path_str(&util::home());
        let mut by: std::collections::BTreeMap<String, (usize, u64)> = Default::default();
        for i in &items {
            let rel = i.path.strip_prefix(&home).unwrap_or(&i.path);
            let top: String = rel.trim_start_matches('/').split('/').take(if rel.starts_with("/Library") { 2 } else { 1 }).collect::<Vec<_>>().join("/");
            let e = by.entry(top).or_default();
            e.0 += 1;
            e.1 += i.size;
        }
        let mut v: Vec<_> = by.into_iter().collect();
        v.sort_by(|a, b| b.1 .1.cmp(&a.1 .1));
        for (k, (n, s)) in v.iter().take(12) {
            println!("  ~/{k:<34} {n:>4} files {}", gb(*s));
        }
        let bad: Vec<&Item> = items.iter().filter(|i| i.path.contains("/.git/") || i.path.contains("/target/") || i.path.contains("/.venv/")).collect();
        println!("  git/target/venv files listed: {}", bad.len());
        for i in bad.iter().take(5) { println!("    {}", i.path); }
        let inuse: Vec<&Item> = items.iter().filter(|i| i.note.as_deref() == Some("inUse")).collect();
        println!("  in use: {}", inuse.len());
        for i in inuse.iter().take(5) { println!("    {} {}", gb(i.size), i.path); }
        let sys = junk::system_junk();
        let su: Vec<&Item> = sys.iter().filter(|i| i.note.as_deref() == Some("inUse")).collect();
        println!("system junk: {} items, {} in use ({}), preselected {}", sys.len(), su.len(), gb(su.iter().map(|i| i.size).sum()), gb(sys.iter().filter(|i| i.selected).map(|i| i.size).sum()));
        for i in su.iter().take(8) { println!("    {} {}", gb(i.size), i.path); }
        let dev = junk::dev_junk(&["~".into()], 90);
        for i in dev.iter().filter(|i| i.group == "nodeModules" || i.note.is_some()) { println!("  dev {} {} sel={} note={:?}", gb(i.size), i.path, i.selected, i.note); }
    }

    #[test]
    #[ignore]
    fn smart_probe() {
        let mods: Vec<(&str, Vec<Item>)> = vec![
            ("system", junk::system_junk()),
            ("dev", junk::dev_junk(&["~".into()], 90)),
            ("leftovers", apps::leftovers()),
            ("downloads", junk::downloads()),
        ];
        let mut total = 0;
        for (m, items) in &mods {
            let sel: Vec<&Item> = items.iter().filter(|i| i.selected).collect();
            let s: u64 = sel.iter().map(|i| i.size).sum();
            total += s;
            println!("== {m}: {} selected, {}", sel.len(), gb(s));
            for i in sel.iter().filter(|i| i.size > 20_000_000) {
                println!("   {:>9} [{}] {}", gb(i.size), i.group, i.path);
            }
            let small: Vec<&&Item> = sel.iter().filter(|i| i.size <= 20_000_000).collect();
            println!("   + {} smaller items, {}", small.len(), gb(small.iter().map(|i| i.size).sum()));
        }
        println!("TOTAL preselected {}", gb(total));
    }

    #[test]
    #[ignore]
    fn disk_probe() {
        let t = Instant::now();
        let prog = disk::Progress::new();
        let ex = disk::scan_exclusions();
        let root = disk::scan(Path::new(disk::DATA_ROOT), &prog, &ex).unwrap();
        let (data_used, _, _) = disk::volume_usage(disk::DATA_ROOT);
        println!(
            "scanned {} in {:?}, files {}, denied dirs {}, df data used {}",
            gb(root.size), t.elapsed(), prog.files.load(Ordering::Relaxed),
            prog.denied.load(Ordering::Relaxed), gb(data_used)
        );
        let snap = disk::Snapshot { root, display_prefix: disk::DATA_ROOT.into(), system_used: 0, hidden: 0 };
        let v = disk::view(&snap, "", 2).unwrap();
        for c in &v.children {
            println!("  {:<28} {:>10} del={} kids={}", c.name, gb(c.size), c.deletable, c.children.len());
        }
        let home = util::home();
        let rel = home.strip_prefix("/").unwrap().to_string_lossy().to_string();
        let hv = disk::view(&snap, &rel, 1).unwrap();
        for c in hv.children.iter().take(12) {
            println!("  ~ {:<26} {:>10} del={} path={}", c.name, gb(c.size), c.deletable, c.path);
        }
    }
}

/// IPC contract tests: send the exact JSON the frontend sends (camelCase
/// argument and field names) through Tauri's mock runtime.
#[cfg(test)]
mod ipc {
    use super::*;
    use serde_json::{json, Value};
    use tauri::test::{get_ipc_response, mock_builder, mock_context, noop_assets, INVOKE_KEY};
    use tauri::webview::InvokeRequest;

    fn call(cmd: &str, body: Value) -> Result<Value, Value> {
        let app = mock_builder()
            .invoke_handler(handlers())
            .build(mock_context(noop_assets()))
            .unwrap();
        let wv = tauri::WebviewWindowBuilder::new(&app, "main", Default::default()).build().unwrap();
        get_ipc_response(
            &wv,
            InvokeRequest {
                cmd: cmd.into(),
                callback: tauri::ipc::CallbackFn(0),
                error: tauri::ipc::CallbackFn(1),
                url: "tauri://localhost".parse().unwrap(),
                body: tauri::ipc::InvokeBody::Json(body),
                headers: Default::default(),
                invoke_key: INVOKE_KEY.to_string(),
            },
        )
        .map(|b| b.deserialize::<Value>().unwrap())
    }

    #[test]
    fn menu_titles_come_from_translation_files() {
        assert_eq!(menu_text(false, "quit"), "Quit MacCleaner");
        assert_ne!(menu_text(true, "quit"), "quit");
        assert_ne!(menu_text(true, "quit"), menu_text(false, "quit"));
    }

    #[test]
    fn versions_match_across_config_files() {
        let conf: Value = serde_json::from_str(include_str!("../tauri.conf.json")).unwrap();
        let pkg: Value = serde_json::from_str(include_str!("../../package.json")).unwrap();
        assert_eq!(conf["version"], json!(env!("CARGO_PKG_VERSION")));
        assert_eq!(pkg["version"], json!(env!("CARGO_PKG_VERSION")));
    }

    #[test]
    fn app_id_matches_tauri_config() {
        let conf: Value = serde_json::from_str(include_str!("../tauri.conf.json")).unwrap();
        assert_eq!(conf["identifier"], json!(APP_ID));
    }

    #[test]
    fn settings_shape_is_camel_case() {
        let v = call("get_settings", json!({})).unwrap();
        for k in ["dryRun", "largeMinMb", "nodeModulesRoots", "onboardingDone", "theme", "language"] {
            assert!(v.get(k).is_some(), "missing {k} in {v}");
        }
    }

    #[test]
    fn app_related_accepts_frontend_args() {
        let v = call("app_related", json!({"bundleId": "com.example.none", "name": "NoSuchApp", "path": "/Applications/NoSuchApp.app"}));
        assert!(v.unwrap().is_array());
    }

    #[test]
    fn disk_view_before_scan_reports_no_scan() {
        assert_eq!(call("disk_view", json!({"path": "/", "depth": 3})).unwrap_err(), json!("noScan"));
    }

    #[test]
    fn uninstall_request_deserializes_and_refuses_protected() {
        let req = json!({"req": {
            "appPath": "/System/Applications/Calculator.app", "bundleId": "com.apple.calculator",
            "appSize": 1, "brewToken": null,
            "related": [{"path": "/tmp/x", "size": 1, "kind": "file", "command": null}]
        }});
        assert_eq!(call("uninstall", req).unwrap_err(), json!("protected"));
        assert!(Path::new("/System/Applications/Calculator.app").exists());
    }

    #[test]
    fn missing_argument_is_rejected() {
        assert!(call("app_related", json!({"bundle_id": "x"})).is_err());
    }
}
