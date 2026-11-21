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
    assert!(env.err("/e 1-1").contains("Genau eine Nummer"));
    assert!(env.err("/e 1-2").contains("Genau eine Nummer"));
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

/// Abbruch zwischen Anhängen an done.txt und Schreiben von todo.txt (SPEC 6.2, 12):
/// schlimmstenfalls Duplikate in done.txt, aber kein Verlust.
#[cfg(unix)]
#[test]
fn archive_failure_after_append_loses_nothing() {
    use std::os::unix::fs::PermissionsExt;

    let mut env = Env::new(Some("a\nx 2026-11-20 erledigt\n"));
    fs::write(env.path("done.txt"), "").unwrap();
    // Verzeichnis schreibgeschützt: done.txt lässt sich anhängen,
    // die temporäre Datei für todo.txt aber nicht anlegen.
    let dir = env.dir.path().to_path_buf();
    fs::set_permissions(&dir, fs::Permissions::from_mode(0o555)).unwrap();
    if fs::write(dir.join("probe"), "").is_ok() {
        // als root greifen die Rechte nicht – Test hier nicht aussagekräftig
        fs::set_permissions(&dir, fs::Permissions::from_mode(0o755)).unwrap();
        return;
    }

    env.ui.confirms.push_back(true);
    let err = env.err("/archive");
    fs::set_permissions(&dir, fs::Permissions::from_mode(0o755)).unwrap();

    assert!(err.contains("kann nicht angelegt werden"), "{err}");
    assert_eq!(env.file(), "a\nx 2026-11-20 erledigt\n");
    assert_eq!(fs::read_to_string(env.path("done.txt")).unwrap(), "x 2026-11-20 erledigt\n");
    assert!(fs::read_dir(&dir).unwrap().all(|e| !e.unwrap().file_name().to_string_lossy().contains(".tmp-")));

    // Erneuter Versuch klappt; die Aufgabe steht dann doppelt in done.txt
    env.ui.confirms.push_back(true);
    env.ok("/archive");
    assert_eq!(env.file(), "a\n");
    assert_eq!(
        fs::read_to_string(env.path("done.txt")).unwrap(),
        "x 2026-11-20 erledigt\nx 2026-11-20 erledigt\n"
    );
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
    assert!(env.err("/help foo").contains("Unbekannter Befehl /foo –"));
    assert!(env.err("/help /foo").contains("Unbekannter Befehl /foo –"));
    assert_eq!(env.run("/q").unwrap(), Output::Quit);
}

// ---------- Version 2: /use ----------

#[test]
fn use_sets_prompt_and_tags_new_tasks() {
    let mut env = Env::new(Some("a +digra\nb +web\n"));
    assert_eq!(env.state.prompt(), "todo> ");
    assert_eq!(env.ok("/use +digra"), "Aktives Projekt: +digra");
    assert_eq!(env.state.prompt(), "todo +digra> ");
    assert_eq!(env.ok("/use neu"), "Aktives Projekt: +neu (noch keine Aufgaben)");
    env.ok("/use +digra");

    // ohne eigenes Projekt: aktives wird angehängt
    assert_eq!(env.ok("Login fixen"), "+ 3 2026-11-21 Login fixen +digra");
    // mit anderem Projekt: unverändert
    assert_eq!(env.ok("Theme +web"), "+ 4 2026-11-21 Theme +web");
    assert_eq!(env.ok("/undo"), "Rückgängig: /add Theme +web");

    assert!(env.err("/use +a +b").contains("Genau ein Projekt"));
    assert!(env.err("/use @ctx").contains("Genau ein Projekt"));
    assert_eq!(env.ok("/use"), "Aktives Projekt +digra aufgehoben");
    assert_eq!(env.state.prompt(), "todo> ");
    assert_eq!(env.ok("/use"), "Kein aktives Projekt");
    assert_eq!(env.ok("ohne"), "+ 4 2026-11-21 ohne");
}

#[test]
fn ls_filters_on_active_project_unless_star() {
    let mut env = Env::new(Some("a +Digra\nb +web\nc +digra @rechner\n"));
    env.ok("/use +digra");
    assert_eq!(env.ok("/ls"), "1     a +Digra\n3     c +digra @rechner\n2 von 3 Aufgaben");
    assert_eq!(env.ok("/ls @rechner"), "3     c +digra @rechner\n1 von 3 Aufgaben");
    assert_eq!(env.ok("/ls *"), "1     a +Digra\n2     b +web\n3     c +digra @rechner\n3 von 3 Aufgaben");
    env.ok("/use");
    assert!(env.ok("/ls").ends_with("3 von 3 Aufgaben"));
}

// ---------- Version 2: /pri ----------

#[test]
fn pri_sets_replaces_and_removes_in_place() {
    let mut env = Env::new(Some("2026-11-20 Bug  fixen +p\nx 2026-11-21 fertig pri:B\n\n"));
    assert_eq!(env.ok("/pri 1 a"), "~ 1 (A) 2026-11-20 Bug  fixen +p");
    assert_eq!(env.ok("/p 1 C"), "~ 1 (C) 2026-11-20 Bug  fixen +p");
    assert_eq!(env.ok("/pri 1 C"), "1 hat bereits Priorität C");
    assert_eq!(env.file(), "(C) 2026-11-20 Bug  fixen +p\nx 2026-11-21 fertig pri:B\n\n");
    assert_eq!(env.ok("/pri 1 -"), "~ 1 2026-11-20 Bug  fixen +p");
    assert_eq!(env.ok("/pri 1 -"), "1 hat keine Priorität");
    assert_eq!(env.file(), "2026-11-20 Bug  fixen +p\nx 2026-11-21 fertig pri:B\n\n");

    assert_eq!(env.ok("/undo"), "Rückgängig: /pri 1 -");
    assert_eq!(env.file(), "(C) 2026-11-20 Bug  fixen +p\nx 2026-11-21 fertig pri:B\n\n");

    assert!(env.err("/pri 2 A").contains("ist erledigt"));
    assert!(env.err("/pri 3 A").contains("Leerzeile"));
    assert!(env.err("/pri 9 A").contains("gibt es nicht"));
    assert!(env.err("/pri 1").contains("Nummer und Priorität"));
    assert!(env.err("/pri 1-2 A").contains("Nummer und Priorität"));
    assert!(env.err("/pri 1 AB").contains("Buchstabe A–Z"));
    assert!(env.err("/pri 1 7").contains("Buchstabe A–Z"));
}

#[test]
fn pri_is_rejected_after_external_change() {
    let mut env = Env::new(Some("a\n"));
    env.write_external("neu\na\n");
    assert!(env.err("/pri 1 A").contains("extern geändert"));
    assert_eq!(env.file(), "neu\na\n");
}

// ---------- Version 2: /projects, /contexts ----------

#[test]
fn projects_and_contexts_count_open_tasks() {
    let mut env = Env::new(Some(
        "a +digra @rechner\nb +Digra +digra\nx 2026-11-21 c +keycloak +digra\nd +batterypass @telefon\n\ne +keycloak @rechner\nf +digra\n",
    ));
    assert_eq!(env.ok("/projects"), "+digra        3\n+batterypass  1\n+keycloak     1");
    assert_eq!(env.ok("/cx"), "@rechner  2\n@telefon  1");

    let mut empty = Env::new(Some("nur text\n"));
    assert_eq!(empty.ok("/pj"), "Keine Projekte in offenen Aufgaben");
    assert_eq!(empty.ok("/contexts"), "Keine Kontexte in offenen Aufgaben");
}

#[test]
fn help_lists_version_2_commands() {
    let mut env = Env::new(Some(""));
    let out = env.ok("/help");
    for c in ["/use [+projekt]", "/pri, /p Nr Prio", "/projects, /pj", "/contexts, /cx"] {
        assert!(out.contains(c), "{c} fehlt in:\n{out}");
    }
}
