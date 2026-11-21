//! Befehle gegen eine Datei im temporären Verzeichnis (SPEC 12).

use std::collections::VecDeque;
use std::fs;
use std::path::PathBuf;

use chrono::NaiveDate;
use tempfile::TempDir;
use todo::app::State;
use todo::commands::{Output, Prompter, Registry, execute, parse_numbers};
use todo::config::Config;
use todo::store::{Document, Store};

#[derive(Default)]
struct TestUi {
    confirms: VecDeque<bool>,
    edits: VecDeque<Option<String>>,
    shown: Vec<String>,
}

impl Prompter for TestUi {
    fn show(&mut self, text: &str) {
        self.shown.push(text.to_string());
    }
    fn confirm(&mut self, _question: &str) -> bool {
        self.confirms.pop_front().expect("unerwartete Rückfrage")
    }
    fn edit_line(&mut self, _initial: &str) -> Option<String> {
        self.edits.pop_front().expect("unerwartetes /edit")
    }
}

struct Env {
    dir: TempDir,
    state: State,
    ui: TestUi,
    registry: Registry,
}

impl Env {
    fn new(content: Option<&str>) -> Env {
        let dir = TempDir::new().unwrap();
        if let Some(c) = content {
            fs::write(dir.path().join("todo.txt"), c).unwrap();
        }
        Env::open(dir)
    }

    fn open(dir: TempDir) -> Env {
        let store = Store::new(dir.path().join("todo.txt"));
        let mut state = State::open(store, Config::default(), false).unwrap();
        state.today_override = NaiveDate::from_ymd_opt(2026, 11, 21);
        Env { dir, state, ui: TestUi::default(), registry: Registry::new() }
    }

    fn path(&self, name: &str) -> PathBuf {
        self.dir.path().join(name)
    }

    fn run(&mut self, line: &str) -> anyhow::Result<Output> {
        execute(&self.registry, line, &mut self.state, &mut self.ui)
    }

    fn ok(&mut self, line: &str) -> String {
        match self.run(line) {
            Ok(Output::Text(t)) => t,
            Ok(other) => panic!("{line}: {other:?}"),
            Err(e) => panic!("{line}: {e:#}"),
        }
    }

    fn err(&mut self, line: &str) -> String {
        match self.run(line) {
            Err(e) => format!("{e:#}"),
            Ok(o) => panic!("{line}: Fehler erwartet, bekam {o:?}"),
        }
    }

    fn file(&self) -> String {
        fs::read_to_string(self.path("todo.txt")).unwrap()
    }

    fn write_external(&self, content: &str) {
        fs::write(self.path("todo.txt"), content).unwrap();
    }
}

// ---------- /add ----------

#[test]
fn add_creates_missing_file_with_date_and_number() {
    let mut env = Env::new(None);
    assert!(!env.path("todo.txt").exists());
    assert_eq!(env.ok("Login-Bug fixen +digra"), "+ 1 2026-11-21 Login-Bug fixen +digra");
    assert_eq!(env.ok("/add (A) Release"), "+ 2 (A) 2026-11-21 Release");
    assert_eq!(env.ok("/a 2026-01-01 Alt"), "+ 3 2026-01-01 Alt");
    assert_eq!(env.file(), "2026-11-21 Login-Bug fixen +digra\n(A) 2026-11-21 Release\n2026-01-01 Alt\n");
}

#[test]
fn add_keeps_crlf_and_fixes_missing_newline() {
    let mut env = Env::new(Some("a\r\nb"));
    env.ok("c");
    assert_eq!(env.file(), "a\r\nb\r\n2026-11-21 c\r\n");
}

#[test]
fn add_rejects_x_and_ignores_empty() {
    let mut env = Env::new(Some(""));
    assert!(env.err("x Achse beschriften").contains("„x “"));
    assert_eq!(env.run("   ").unwrap(), Output::Nothing);
    assert_eq!(env.run("/add").unwrap(), Output::Nothing);
    assert_eq!(env.file(), "");
}

#[test]
fn add_without_date_when_configured() {
    let mut env = Env::new(Some(""));
    env.state.config.date_on_add = false;
    env.ok("Ohne Datum");
    assert_eq!(env.file(), "Ohne Datum\n");
}

#[test]
fn unknown_command_is_not_saved() {
    let mut env = Env::new(Some("a\n"));
    assert!(env.err("/foo bar").contains("Unbekannter Befehl /foo"));
    assert_eq!(env.file(), "a\n");
}

// ---------- /done ----------

#[test]
fn done_changes_line_in_place() {
    let mut env = Env::new(Some("(A) 2026-11-20 Login  fixen +digra\nzweite\n"));
    assert_eq!(env.ok("/done 1"), "✓ 1 x 2026-11-21 2026-11-20 Login  fixen +digra pri:A");
    assert_eq!(env.file(), "x 2026-11-21 2026-11-20 Login  fixen +digra pri:A\nzweite\n");
    assert_eq!(env.ok("/d 1 2"), "1 ist bereits erledigt\n✓ 2 x 2026-11-21 zweite");
}

#[test]
fn done_requires_valid_numbers() {
    let mut env = Env::new(Some("a\n\nc\n"));
    assert!(env.err("/done").contains("Nummer fehlt"));
    assert!(env.err("/done 2").contains("Leerzeile"));
    assert!(env.err("/done 9").contains("gibt es nicht"));
    assert!(env.err("/done abc").contains("keine gültige Nummer"));
    assert_eq!(env.file(), "a\n\nc\n");
}

// ---------- /del ----------

#[test]
fn del_leaves_blank_line_and_keeps_numbers() {
    let mut env = Env::new(Some("a\nb\nc\n"));
    env.ui.confirms.push_back(false);
    assert_eq!(env.ok("/del 2"), "Abgebrochen");
    assert_eq!(env.file(), "a\nb\nc\n");

    env.ui.confirms.push_back(true);
    assert_eq!(env.ok("/del 2"), "Gelöscht: 2");
    assert_eq!(env.ui.shown.last().unwrap(), "  2 b");
    assert_eq!(env.file(), "a\n\nc\n");
    // Nr. 3 ist weiterhin c
    assert_eq!(env.ok("/done 3"), "✓ 3 x 2026-11-21 c");
}

// ---------- /edit ----------

#[test]
fn edit_replaces_whole_line() {
    let mut env = Env::new(Some("x 2026-11-20 2026-11-19 erledigt\nb\n"));
    env.ui.edits.push_back(Some("2026-11-19 wieder offen".into()));
    assert_eq!(env.ok("/edit 1"), "~ 1 2026-11-19 wieder offen");
    assert_eq!(env.file(), "2026-11-19 wieder offen\nb\n");

    env.ui.edits.push_back(None);
    assert_eq!(env.ok("/e 2"), "Abgebrochen");
    env.ui.edits.push_back(Some("b".into()));
    assert_eq!(env.ok("/e 2"), "Keine Änderung");
    env.ui.edits.push_back(Some("   ".into()));
    assert!(env.err("/e 2").contains("Leerer Text"));
    assert!(env.err("/e 1 2").contains("Genau eine Nummer"));
    assert_eq!(env.file(), "2026-11-19 wieder offen\nb\n");
    assert_eq!(env.state.undo.len(), 1);
}

// ---------- /undo ----------

#[test]
fn undo_restores_previous_states() {
    let mut env = Env::new(None);
    env.ok("a");
    env.ok("b");
    env.ok("/done 1");
    assert_eq!(env.ok("/undo"), "Rückgängig: /done 1");
    assert_eq!(env.file(), "2026-11-21 a\n2026-11-21 b\n");
    assert_eq!(env.ok("/u"), "Rückgängig: /add b");
    assert_eq!(env.ok("/u"), "Rückgängig: /add a");
    assert!(!env.path("todo.txt").exists(), "Datei existierte vorher nicht");
    assert_eq!(env.ok("/u"), "Nichts rückgängig zu machen");
}

#[test]
fn undo_after_external_change_is_rejected() {
    let mut env = Env::new(Some("a\n"));
    env.ok("b");
    env.write_external("a\nb\nextern\n");
    assert!(env.err("/undo").contains("Undo nicht möglich"));
    assert_eq!(env.file(), "a\nb\nextern\n");
    assert_eq!(env.ok("/undo"), "Nichts rückgängig zu machen");
}

// ---------- externe Änderungen ----------

#[test]
fn external_change_rejects_number_commands_once() {
    let mut env = Env::new(Some("a\nb\n"));
    env.write_external("neu\na\nb\n");
    assert!(env.err("/done 1").contains("extern geändert"));
    assert_eq!(env.file(), "neu\na\nb\n");
    // Danach gilt der neue Stand
    assert_eq!(env.ok("/done 2"), "✓ 2 x 2026-11-21 a");
}

#[test]
fn external_change_reloads_for_ls_and_add() {
    let mut env = Env::new(Some("a\n"));
    env.write_external("a\nextern\n");
    assert!(env.ok("/ls").contains("2     extern"));
    env.write_external("a\nextern\nnoch eins\n");
    assert_eq!(env.ok("c"), "+ 4 2026-11-21 c");
    assert_eq!(env.file(), "a\nextern\nnoch eins\n2026-11-21 c\n");
}

// ---------- /ls ----------

#[test]
fn ls_sorts_and_shows_line_numbers() {
    let content = "\
2026-11-01 Theme-Update einspielen +website
2026-11-20 JSON-Schema-Export testen +batterypass due:2026-11-27
x 2026-11-21 2026-11-19 Login-Timeout erhöhen +digra
(A) 2026-11-20 Login-Bug fixen +digra @rechner
";
    let mut env = Env::new(Some(content));
    assert_eq!(
        env.ok("/ls"),
        "4 (A) Login-Bug fixen +digra @rechner\n\
         2     JSON-Schema-Export testen +batterypass due:2026-11-27\n\
         1     Theme-Update einspielen +website\n\
         3 von 3 Aufgaben · 1 erledigt – /archive räumt auf"
    );
    assert_eq!(env.ok("/l +digra"), "4 (A) Login-Bug fixen +digra @rechner\n1 von 3 Aufgaben · 1 erledigt – /archive räumt auf");
}

#[test]
fn ls_status_all_includes_done_file_without_number() {
    let mut env = Env::new(Some("(A) Login-Bug fixen\nx 2026-11-21 Login-Timeout\n"));
    fs::write(env.path("done.txt"), "x 2026-11-01 Login-Seite stylen\nx 2026-11-02 Anderes\n").unwrap();
    assert_eq!(
        env.ok("/ls login status:all"),
        "1 (A) Login-Bug fixen\n2   x Login-Timeout\n    x Login-Seite stylen\n3 von 4 Aufgaben · 1 erledigt – /archive räumt auf"
    );
    assert!(env.ok("/ls status:done").starts_with("2   x Login-Timeout\n"));
}

#[test]
fn ls_aligns_numbers() {
    let content: String = (1..=12).map(|i| format!("Aufgabe {i}\n")).collect();
    let mut env = Env::new(Some(&content));
    let out = env.ok("/ls Aufgabe 1");
    assert!(out.starts_with(" 1     Aufgabe 1\n10     Aufgabe 10\n"), "{out}");
}

#[test]
fn ls_invalid_date_is_error() {
    let mut env = Env::new(Some("a\n"));
    assert!(env.err("/ls due<morgen").contains("Ungültige Datumsangabe"));
}

// ---------- /archive ----------

#[test]
fn archive_moves_done_and_removes_blanks() {
    let mut env = Env::new(Some("\u{feff}a\r\nx 2026-11-20 erledigt\r\n\r\nb\r\nx 2026-11-21 auch\r\n"));
    fs::write(env.path("done.txt"), "x 2026-01-01 alt").unwrap();
    env.ok("c");

    env.ui.confirms.push_back(false);
    assert_eq!(env.ok("/archive"), "Abgebrochen");

    env.ui.confirms.push_back(true);
    let out = env.ok("/archive");
    assert!(out.starts_with("Archiviert: 2 Aufgaben nach done.txt, 1 Leerzeile entfernt\n"), "{out}");
    assert!(out.ends_with("3 von 3 Aufgaben"), "{out}");
    assert_eq!(env.file(), "\u{feff}a\r\nb\r\n2026-11-21 c\r\n");
    assert_eq!(
        fs::read_to_string(env.path("done.txt")).unwrap(),
        "x 2026-01-01 alt\nx 2026-11-20 erledigt\nx 2026-11-21 auch\n"
    );
    assert_eq!(env.ok("/undo"), "Nichts rückgängig zu machen (seit /archive)");
    assert_eq!(env.ok("/archive"), "Nichts zu archivieren");
}

// ---------- Nummern ----------

#[test]
fn numbers_stay_stable_until_archive() {
    let mut env = Env::new(Some("a\nb\nc\nd\n"));
    env.ok("/done 1");
    env.ui.confirms.push_back(true);
    env.ok("/del 2");
    env.ui.edits.push_back(Some("C neu".into()));
    env.ok("/edit 3");
    env.ok("e");
    let lines: Vec<String> = env.file().lines().map(String::from).collect();
    assert_eq!(lines, ["x 2026-11-21 a", "", "C neu", "d", "2026-11-21 e"]);
}

#[test]
fn parse_numbers_ranges() {
    let doc = Document::parse(b"a\n\nc\nd\n").unwrap();
    assert_eq!(parse_numbers("1-4", &doc).unwrap(), vec![1, 3, 4]);
    assert_eq!(parse_numbers("4 1 4", &doc).unwrap(), vec![4, 1]);
    assert!(parse_numbers("2-2", &doc).is_err());
    assert!(parse_numbers("3-9", &doc).is_err());
    assert!(parse_numbers("4-3", &doc).is_err());
    assert!(parse_numbers("0", &doc).is_err());
}

// ---------- /help ----------

#[test]
fn help_lists_commands() {
    let mut env = Env::new(Some(""));
    let out = env.ok("/help");
    for c in ["/add, /a Text", "/ls, /l [Filter]", "/done, /d Nr…", "/archive", "/quit, /q"] {
        assert!(out.contains(c), "{c} fehlt in:\n{out}");
    }
    assert!(env.ok("/h ls").contains("status:all"));
    assert!(env.err("/help foo").contains("Unbekannter Befehl"));
    assert_eq!(env.run("/q").unwrap(), Output::Quit);
}
