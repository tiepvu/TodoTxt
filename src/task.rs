use chrono::NaiveDate;

/// Geparste Sicht auf eine Zeile.
///
/// Die Zeile selbst bleibt immer als Rohtext erhalten (`store::Line`); `Task`
/// dient nur zum Filtern, Sortieren und Anzeigen. Geändert wird stets der
/// Rohtext, damit unbekannte Tokens und Schreibweisen erhalten bleiben.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Task {
    /// `x ` am Zeilenanfang
    pub done: bool,
    /// `(A)`–`(Z)` am Zeilenanfang einer offenen Aufgabe
    pub priority: Option<char>,
    /// Erledigungsdatum (erstes Datum nach `x `)
    pub completed: Option<NaiveDate>,
    /// Erstellungsdatum
    pub created: Option<NaiveDate>,
    /// Text nach Status, Priorität und Datumsangaben, inkl. aller Tokens
    pub body: String,
    /// `+projekt`-Tokens, wie geschrieben
    pub projects: Vec<String>,
    /// `@kontext`-Tokens, wie geschrieben
    pub contexts: Vec<String>,
    /// `due:JJJJ-MM-TT`
    pub due: Option<NaiveDate>,
    /// `pri:X` erledigter Aufgaben
    pub pri_tag: Option<char>,
}

impl Task {
    /// Priorität für Filter: bei erledigten Aufgaben der Wert von `pri:`.
    pub fn effective_priority(&self) -> Option<char> {
        if self.done { self.pri_tag } else { self.priority }
    }
}
