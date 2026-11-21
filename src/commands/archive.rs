use anyhow::{Result, bail};

use super::ls::render_listing;
use super::{Command, Context, Output};
use crate::parser;
use crate::store::Document;

pub struct Archive;

fn plural(n: usize, one: &str, many: &str) -> String {
    format!("{n} {}", if n == 1 { one } else { many })
}

impl Command for Archive {
    fn name(&self) -> &'static str {
        "archive"
    }
    fn summary(&self) -> &'static str {
        "Erledigte nach done.txt verschieben, Leerzeilen entfernen, neu nummerieren"
    }
    fn help(&self) -> &'static str {
        "/archive

Einziger Befehl, der Aufgaben neu nummeriert. Verschiebt alle erledigten Zeilen ans
Ende von done.txt und entfernt Leerzeilen aus todo.txt. Fragt vorher nach.
Nicht per /undo rückgängig zu machen; der Undo-Stapel wird geleert."
    }

    fn run(&self, _args: &str, ctx: &mut Context) -> Result<Output> {
        let doc = &ctx.state.doc;
        let mut done = Vec::new();
        let mut blanks = 0;
        let mut keep = Document { bom: doc.bom, lines: Vec::new() };
        for line in &doc.lines {
            if line.is_blank() {
                blanks += 1;
            } else if parser::parse(&line.text).done {
                done.push(line.text.clone());
            } else {
                keep.lines.push(line.clone());
            }
        }
        if done.is_empty() && blanks == 0 {
            return Ok(Output::Text("Nichts zu archivieren".into()));
        }
        let question = format!(
            "{} archivieren, {} entfernen – Nummern ändern sich. Fortfahren? [j/N] ",
            plural(done.len(), "erledigte Aufgabe", "erledigte Aufgaben"),
            plural(blanks, "Leerzeile", "Leerzeilen"),
        );
        if !ctx.ui.confirm(&question) {
            return Ok(Output::Text("Abgebrochen".into()));
        }

        let state = &mut *ctx.state;
        // Erst prüfen, dann an done.txt anhängen, dann todo.txt atomar schreiben:
        // ein Abbruch dazwischen führt höchstens zu Duplikaten in done.txt.
        if state.disk_changed()? {
            state.reload()?;
            bail!(state.external_change_msg());
        }
        if !done.is_empty() {
            state.store.append_done(&done)?;
        }
        state.write_checked(&keep)?;
        state.undo.clear(true);

        let mut out = format!(
            "Archiviert: {} nach done.txt, {} entfernt\n",
            plural(done.len(), "Aufgabe", "Aufgaben"),
            plural(blanks, "Leerzeile", "Leerzeilen"),
        );
        out.push_str(&render_listing(state, "")?);
        Ok(Output::Text(out))
    }
}
