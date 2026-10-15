//! Reference default (D17), changed in this project (T25): the page keeps
//! one thing between visits, the save, in `localStorage`, and only
//! `web/store.js` may touch it. This reads every file under web/ (vendor/
//! aside) and fails if any other file names a browser storage API, or if
//! store.js names one other than `localStorage`. The core decides what is
//! stored and when (`Game::take_autosave`, `Game::restore`); an unreadable
//! save is treated as a new one and set aside (`game.rs` tests). From
//! Feedback Drafts and Status Trends.

use std::fs;
use std::path::{Path, PathBuf};

const STORAGE_NAMES: &[&str] = &[
    "localStorage",
    "sessionStorage",
    "indexedDB",
    "IDBFactory",
    "document.cookie",
    "caches",
    "CacheStorage",
    "serviceWorker",
    "navigator.storage",
    "openDatabase",
];

fn storage_names_in(text: &str) -> Vec<&'static str> {
    STORAGE_NAMES
        .iter()
        .copied()
        .filter(|name| text.contains(name))
        .collect()
}

fn files_under(dir: &Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    let mut stack = vec![dir.to_path_buf()];
    while let Some(d) = stack.pop() {
        let Ok(read) = fs::read_dir(&d) else {
            continue;
        };
        for entry in read {
            let path = entry.expect("entry").path();
            if path.is_dir() {
                if path.file_name().map(|n| n != "vendor").unwrap_or(true) {
                    stack.push(path);
                }
            } else {
                out.push(path);
            }
        }
    }
    out.sort();
    out
}

#[test]
fn the_page_names_no_browser_storage() {
    let web = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../web");
    let mut found = Vec::new();
    for path in files_under(&web) {
        let Ok(text) = fs::read_to_string(&path) else {
            continue;
        };
        let store = path.ends_with("web/store.js");
        for name in storage_names_in(&text) {
            if !(store && name == "localStorage") {
                found.push(format!("{}: {name}", path.display()));
            }
        }
    }
    assert!(
        found.is_empty(),
        "browser storage outside web/store.js, or other than localStorage (D17, T25):\n{}",
        found.join("\n")
    );
}

#[test]
fn the_save_store_exists_and_names_only_local_storage() {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../web/store.js");
    let text = fs::read_to_string(&path).expect("web/store.js");
    assert_eq!(storage_names_in(&text), vec!["localStorage"]);
}

#[test]
fn the_scanner_sees_storage() {
    assert_eq!(
        storage_names_in("window.localStorage.setItem('n', name)"),
        vec!["localStorage"]
    );
    assert_eq!(storage_names_in("indexedDB.open('x')"), vec!["indexedDB"]);
    assert!(storage_names_in("el(\"note\").value").is_empty());
}
