//! Store: atomares Schreiben, Symlinks, Rechte, Kodierung (SPEC 10, 12).

use std::fs;

use tempfile::TempDir;
use todo::store::{Document, Store, write_atomic};

#[test]
fn write_replaces_content_and_leaves_no_temp_file() {
    let dir = TempDir::new().unwrap();
    let p = dir.path().join("todo.txt");
    fs::write(&p, "alt\n").unwrap();
    write_atomic(&p, b"neu\n").unwrap();
    assert_eq!(fs::read_to_string(&p).unwrap(), "neu\n");
    let names: Vec<_> = fs::read_dir(dir.path()).unwrap().map(|e| e.unwrap().file_name()).collect();
    assert_eq!(names, ["todo.txt"]);
}

#[test]
fn failed_write_keeps_original() {
    let dir = TempDir::new().unwrap();
    // Ziel ist ein Verzeichnis → Umbenennen scheitert
    let p = dir.path().join("todo.txt");
    fs::create_dir(&p).unwrap();
    assert!(write_atomic(&p, b"neu\n").is_err());
    assert!(p.is_dir());
    assert_eq!(fs::read_dir(dir.path()).unwrap().count(), 1, "temporäre Datei wurde entfernt");
}

#[test]
fn missing_file_reads_as_none() {
    let dir = TempDir::new().unwrap();
    let store = Store::new(dir.path().join("todo.txt"));
    assert!(store.read_raw().unwrap().is_none());
    assert!(store.read_done().unwrap().is_empty());
}

#[test]
fn bom_crlf_and_missing_newline_survive_unrelated_change() {
    let original = "\u{feff}(A) eins\r\nzwei\r\ndrei ohne Umbruch";
    let mut doc = Document::parse(original.as_bytes()).unwrap();
    doc.set_text(2, "ZWEI".into());
    assert_eq!(doc.to_bytes(), "\u{feff}(A) eins\r\nZWEI\r\ndrei ohne Umbruch".as_bytes());
}

#[test]
fn append_done_adds_missing_newline_and_keeps_crlf() {
    let dir = TempDir::new().unwrap();
    let store = Store::new(dir.path().join("todo.txt"));
    fs::write(dir.path().join("done.txt"), "x a\r\nx b").unwrap();
    store.append_done(&["x c".into()]).unwrap();
    assert_eq!(fs::read_to_string(dir.path().join("done.txt")).unwrap(), "x a\r\nx b\r\nx c\r\n");
}

#[cfg(unix)]
#[test]
fn symlink_stays_symlink_and_permissions_are_kept() {
    use std::os::unix::fs::{PermissionsExt, symlink};

    let dir = TempDir::new().unwrap();
    let real_dir = dir.path().join("vault");
    fs::create_dir(&real_dir).unwrap();
    let real = real_dir.join("todo.txt");
    fs::write(&real, "alt\n").unwrap();
    fs::set_permissions(&real, fs::Permissions::from_mode(0o600)).unwrap();
    let link = dir.path().join("todo.txt");
    symlink(&real, &link).unwrap();

    let store = Store::new(link.clone());
    store.write(b"neu\n").unwrap();
    assert!(fs::symlink_metadata(&link).unwrap().file_type().is_symlink());
    assert_eq!(fs::read_to_string(&real).unwrap(), "neu\n");
    assert_eq!(fs::metadata(&real).unwrap().permissions().mode() & 0o777, 0o600);
    // done.txt liegt neben der echten Datei
    assert_eq!(store.done_path(), real_dir.join("done.txt"));
}
