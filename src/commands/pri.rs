use anyhow::{Result, bail};

use super::{Command, Context, Output, parse_numbers};
use crate::parser;

pub struct Pri;

const USAGE_ERR: &str = "Nummer und Priorität angeben – z. B. /pri 3 A oder /pri 3 -";

impl Command for Pri {
    fn name(&self) -> &'static str {
        "pri"
    }
    fn aliases(&self) -> &'static [&'static str] {
        &["p"]
    }
    fn usage(&self) -> &'static str {
        "Nr Prio"
    }
    fn summary(&self) -> &'static str {
        "Priorität setzen; - entfernt sie"
    }
    fn help(&self) -> &'static str {
        "/pri Nr Prio  ·  Kurz: /p

Setzt die Priorität einer offenen Aufgabe (A–Z) oder entfernt sie mit -.
Die Zeile bleibt an ihrer Stelle, der übrige Text unverändert.
Erledigte Aufgaben behalten ihr pri:X; zum Ändern /edit verwenden.

Beispiele:
  /pri 3 A
  /pri 3 -"
    }
    fn uses_numbers(&self) -> bool {
        true
    }

    fn run(&self, args: &str, ctx: &mut Context) -> Result<Output> {
        let parts: Vec<&str> = args.split_whitespace().collect();
        let [num, prio] = parts[..] else { bail!(USAGE_ERR) };
        if num.contains('-') {
            bail!(USAGE_ERR);
        }
        let prio = match prio {
            "-" => None,
            p if p.len() == 1 && p.as_bytes()[0].is_ascii_alphabetic() => Some(p.as_bytes()[0].to_ascii_uppercase() as char),
            _ => bail!("Priorität muss ein Buchstabe A–Z sein oder - zum Entfernen"),
        };

        let state = &mut *ctx.state;
        let n = parse_numbers(num, &state.doc)?[0];
        let raw = state.doc.lines[n - 1].text.clone();
        let task = parser::parse(&raw);
        if task.done {
            bail!("{n} ist erledigt – Priorität nur bei offenen Aufgaben (sonst /edit {n})");
        }
        if task.priority == prio {
            return Ok(Output::Text(match prio {
                Some(p) => format!("{n} hat bereits Priorität {p}"),
                None => format!("{n} hat keine Priorität"),
            }));
        }
        let new = parser::set_priority(&raw, prio);
        let mut doc = state.doc.clone();
        doc.set_text(n, new.clone());
        let label = format!("/pri {n} {}", prio.map_or('-', |p| p));
        state.save(doc, label)?;
        let today = state.today();
        Ok(Output::Text(format!("~ {n} {}", state.renderer.raw_line(&new, today))))
    }
}
