use anyhow::{Result, bail};

use super::{Command, Context, Output, parse_numbers};

pub struct Edit;

impl Command for Edit {
    fn name(&self) -> &'static str {
        "edit"
    }
    fn aliases(&self) -> &'static [&'static str] {
        &["e"]
    }
    fn usage(&self) -> &'static str {
        "Nr"
    }
    fn summary(&self) -> &'static str {
        "Aufgabe mit vorausgefülltem Text bearbeiten"
    }
    fn help(&self) -> &'static str {
        "/edit Nr  ·  Kurz: /e

Zeigt die vollständige Zeile – inkl. x, Priorität und Datumsangaben – zum Bearbeiten.
Enter speichert, Strg+C bricht ab. Der neue Text ersetzt die Zeile unverändert.
Um eine Erledigung zurückzunehmen, „x Datum “ am Anfang entfernen.

Beispiel:
  /edit 7"
    }
    fn uses_numbers(&self) -> bool {
        true
    }

    fn run(&self, args: &str, ctx: &mut Context) -> Result<Output> {
        let nums = parse_numbers(args, &ctx.state.doc)?;
        if nums.len() != 1 || args.split_whitespace().count() != 1 {
            bail!("Genau eine Nummer angeben – z. B. /edit 3");
        }
        let n = nums[0];
        let old = ctx.state.doc.lines[n - 1].text.clone();
        let Some(new) = ctx.ui.edit_line(&old) else {
            return Ok(Output::Text("Abgebrochen".into()));
        };
        if new == old {
            return Ok(Output::Text("Keine Änderung".into()));
        }
        if new.trim().is_empty() {
            bail!("Leerer Text – zum Löschen /del verwenden");
        }
        let state = &mut *ctx.state;
        let mut doc = state.doc.clone();
        doc.set_text(n, new.clone());
        state.save(doc, format!("/edit {n}"))?;
        let today = state.today();
        Ok(Output::Text(format!("~ {n} {}", state.renderer.raw_line(&new, today))))
    }
}
