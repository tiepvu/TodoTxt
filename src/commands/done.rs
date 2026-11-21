use anyhow::Result;

use super::{Command, Context, Output, parse_numbers};
use crate::parser;

pub struct Done;

impl Command for Done {
    fn name(&self) -> &'static str {
        "done"
    }
    fn aliases(&self) -> &'static [&'static str] {
        &["d"]
    }
    fn usage(&self) -> &'static str {
        "Nr…"
    }
    fn summary(&self) -> &'static str {
        "als erledigt markieren"
    }
    fn help(&self) -> &'static str {
        "/done Nr…  ·  Kurz: /d

Setzt „x “ und das heutige Erledigungsdatum. Eine Priorität wird zu pri:X am
Zeilenende. Die Zeile bleibt an ihrer Stelle; erst /archive verschiebt sie nach done.txt.

Beispiele:
  /done 7
  /done 3 5 7
  /done 3-5"
    }
    fn uses_numbers(&self) -> bool {
        true
    }

    fn run(&self, args: &str, ctx: &mut Context) -> Result<Output> {
        let state = &mut *ctx.state;
        let nums = parse_numbers(args, &state.doc)?;
        let today = state.today();
        let mut doc = state.doc.clone();
        let mut lines = Vec::new();
        let mut changed = Vec::new();
        for n in nums {
            let raw = doc.lines[n - 1].text.clone();
            if parser::parse(&raw).done {
                lines.push(format!("{n} ist bereits erledigt"));
                continue;
            }
            let new = parser::mark_done(&raw, today);
            lines.push(format!("✓ {n} {}", state.renderer.raw_line(&new, today)));
            doc.set_text(n, new);
            changed.push(n.to_string());
        }
        if !changed.is_empty() {
            state.save(doc, format!("/done {}", changed.join(" ")))?;
        }
        Ok(Output::Text(lines.join("\n")))
    }
}
