use anyhow::{Result, anyhow};

use super::{Command, Context, Output};
use crate::parser;

pub struct Add;

impl Command for Add {
    fn name(&self) -> &'static str {
        "add"
    }
    fn aliases(&self) -> &'static [&'static str] {
        &["a"]
    }
    fn usage(&self) -> &'static str {
        "Text"
    }
    fn summary(&self) -> &'static str {
        "Aufgabe hinzufügen (auch Freitext ohne /)"
    }
    fn help(&self) -> &'static str {
        "/add Text  ·  Kurz: /a  ·  oder einfach Freitext ohne /

Hängt die Aufgabe als neue Zeile ans Ende von todo.txt und gibt ihre Nummer aus.
- Das Erstellungsdatum wird automatisch gesetzt (abschaltbar mit date_on_add = false).
- (A) am Anfang setzt die Priorität; ein Datum am Anfang gilt als Erstellungsdatum.
- Text, der mit „x “ beginnt, wird abgelehnt (würde als erledigt gelten).
- Aufgaben, die mit / beginnen, nur über /add erfassen.

Beispiele:
  Login-Bug fixen +digra @rechner
  (A) Release vorbereiten due:2026-12-01
  /add /etc/hosts prüfen"
    }

    fn run(&self, args: &str, ctx: &mut Context) -> Result<Output> {
        let text = args.trim();
        if text.is_empty() {
            return Ok(Output::Nothing);
        }
        let state = &mut *ctx.state;
        let today = state.today();
        let raw = parser::build_add(text, today, state.config.date_on_add).map_err(|e| anyhow!(e))?;
        let mut doc = state.doc.clone();
        let n = doc.push_line(raw.clone());
        state.save(doc, format!("/add {text}"))?;
        Ok(Output::Text(format!("+ {n} {}", state.renderer.raw_line(&raw, today))))
    }
}
