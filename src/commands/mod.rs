//! Command-Trait, Registry und Ausführung einer Eingabezeile (SPEC 6, 11).

use anyhow::{Result, bail};

use crate::app::State;
use crate::store::Document;

mod add;
mod archive;
mod del;
mod done;
mod edit;
mod help;
mod ls;
mod pri;
mod quit;
mod tags;
mod undo;
mod r#use;

/// Interaktion mit dem Benutzer. Befehle greifen nie direkt aufs Terminal zu.
pub trait Prompter {
    /// Text vor einer Rückfrage anzeigen (z. B. die betroffenen Zeilen)
    fn show(&mut self, text: &str);
    /// Ja/Nein-Rückfrage; Enter, n und Strg+C ergeben false
    fn confirm(&mut self, question: &str) -> bool;
    /// Vorausgefüllte Eingabezeile; None bei Strg+C
    fn edit_line(&mut self, initial: &str) -> Option<String>;
}

pub struct Context<'a> {
    pub state: &'a mut State,
    pub ui: &'a mut dyn Prompter,
    pub registry: &'a Registry,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Output {
    Nothing,
    Text(String),
    Quit,
}

pub trait Command {
    fn name(&self) -> &'static str;
    fn aliases(&self) -> &'static [&'static str] {
        &[]
    }
    /// Argumente für die Übersicht, z. B. `Nr…`
    fn usage(&self) -> &'static str {
        ""
    }
    /// Einzeiler für `/help`
    fn summary(&self) -> &'static str;
    /// Ausführliche Hilfe für `/help <befehl>`
    fn help(&self) -> &'static str;
    /// Verwendet der Befehl Aufgabennummern? Dann wird er nach externer Änderung abgelehnt.
    fn uses_numbers(&self) -> bool {
        false
    }
    fn run(&self, args: &str, ctx: &mut Context) -> Result<Output>;
}

pub struct Registry {
    commands: Vec<Box<dyn Command>>,
}

impl Default for Registry {
    fn default() -> Self {
        Self::new()
    }
}

impl Registry {
    /// Befehle der Versionen 1 und 2, in der Reihenfolge der Hilfe.
    pub fn new() -> Self {
        Registry {
            commands: vec![
                Box::new(add::Add),
                Box::new(ls::Ls),
                Box::new(done::Done),
                Box::new(edit::Edit),
                Box::new(del::Del),
                Box::new(pri::Pri),
                Box::new(undo::Undo),
                Box::new(archive::Archive),
                Box::new(r#use::Use),
                Box::new(tags::Projects),
                Box::new(tags::Contexts),
                Box::new(help::Help),
                Box::new(quit::Quit),
            ],
        }
    }

    pub fn commands(&self) -> &[Box<dyn Command>] {
        &self.commands
    }

    pub fn find(&self, name: &str) -> Option<&dyn Command> {
        let name = name.trim_start_matches('/').to_lowercase();
        self.commands
            .iter()
            .find(|c| c.name() == name || c.aliases().contains(&name.as_str()))
            .map(|c| c.as_ref())
    }

    /// `/name` und `/alias` aller Befehle (für die Vervollständigung).
    pub fn completion_words(&self) -> Vec<String> {
        let mut words = Vec::new();
        for c in &self.commands {
            words.push(format!("/{}", c.name()));
            words.extend(c.aliases().iter().map(|a| format!("/{a}")));
        }
        words
    }
}

/// Führt eine Eingabezeile der REPL aus: Freitext = `/add`, sonst `/befehl args`.
pub fn execute(registry: &Registry, input: &str, state: &mut State, ui: &mut dyn Prompter) -> Result<Output> {
    let input = input.trim();
    if input.is_empty() {
        return Ok(Output::Nothing);
    }
    let (cmd, args) = match input.strip_prefix('/') {
        Some(rest) => {
            let (name, args) = match rest.split_once(char::is_whitespace) {
                Some((n, a)) => (n, a.trim()),
                None => (rest, ""),
            };
            match registry.find(name) {
                Some(c) => (c, args),
                None => bail!("Unbekannter Befehl /{name} – /help zeigt alle Befehle"),
            }
        }
        None => (registry.find("add").expect("add ist registriert"), input),
    };

    // Externe Änderung seit dem letzten Befehl? (SPEC 5.3)
    if state.disk_changed()? {
        state.reload()?;
        if cmd.uses_numbers() {
            bail!(state.external_change_msg());
        }
    }

    let mut ctx = Context { state, ui, registry };
    cmd.run(args, &mut ctx)
}

/// `3 5 7` oder `3-5`; geprüft gegen die Zeilen der Datei (SPEC 5.3).
pub fn parse_numbers(args: &str, doc: &Document) -> Result<Vec<usize>> {
    if args.trim().is_empty() {
        bail!("Nummer fehlt – z. B. 3, 3 5 7 oder 3-5");
    }
    let len = doc.lines.len();
    let is_blank = |n: usize| doc.line(n).is_none_or(|l| l.is_blank());
    let parse_one = |s: &str| -> Result<usize> {
        match s.parse::<usize>() {
            Ok(n) if n >= 1 => Ok(n),
            _ => bail!("„{s}“ ist keine gültige Nummer"),
        }
    };

    let mut out: Vec<usize> = Vec::new();
    for tok in args.split_whitespace() {
        if let Some((a, b)) = tok.split_once('-') {
            let (a, b) = (parse_one(a)?, parse_one(b)?);
            if b < a {
                bail!("Ungültiger Bereich {tok}");
            }
            if b > len {
                bail!("Nr. {b} gibt es nicht (todo.txt hat {len} Zeilen)");
            }
            let found: Vec<usize> = (a..=b).filter(|n| !is_blank(*n)).collect();
            if found.is_empty() {
                bail!("Im Bereich {tok} gibt es keine Aufgaben");
            }
            out.extend(found);
        } else {
            let n = parse_one(tok)?;
            if n > len {
                bail!("Nr. {n} gibt es nicht (todo.txt hat {len} Zeilen)");
            }
            if is_blank(n) {
                bail!("Nr. {n} ist eine Leerzeile (gelöschte Aufgabe)");
            }
            out.push(n);
        }
    }
    let mut seen = std::collections::HashSet::new();
    out.retain(|n| seen.insert(*n));
    Ok(out)
}
