use anyhow::Result;

use super::{Command, Context, Output, parse_numbers};

pub struct Del;

impl Command for Del {
    fn name(&self) -> &'static str {
        "del"
    }
    fn usage(&self) -> &'static str {
        "Nr…"
    }
    fn summary(&self) -> &'static str {
        "Aufgabe löschen (mit Rückfrage)"
    }
    fn help(&self) -> &'static str {
        "/del Nr…

Zeigt die betroffenen Zeilen und fragt nach. Die Zeile wird durch eine Leerzeile
ersetzt, damit alle anderen Nummern gleich bleiben; /archive entfernt sie später.

Beispiele:
  /del 4
  /del 4 9
  /del 4-6"
    }
    fn uses_numbers(&self) -> bool {
        true
    }

    fn run(&self, args: &str, ctx: &mut Context) -> Result<Output> {
        let nums = parse_numbers(args, &ctx.state.doc)?;
        let today = ctx.state.today();
        let preview: Vec<String> = nums
            .iter()
            .map(|n| format!("  {n} {}", ctx.state.renderer.raw_line(&ctx.state.doc.lines[n - 1].text, today)))
            .collect();
        ctx.ui.show(&preview.join("\n"));
        if !ctx.ui.confirm("Löschen? [j/N] ") {
            return Ok(Output::Text("Abgebrochen".into()));
        }
        let state = &mut *ctx.state;
        let mut doc = state.doc.clone();
        for n in &nums {
            doc.set_text(*n, String::new());
        }
        let list: Vec<String> = nums.iter().map(|n| n.to_string()).collect();
        state.save(doc, format!("/del {}", list.join(" ")))?;
        Ok(Output::Text(format!("Gelöscht: {}", list.join(", "))))
    }
}
