use anyhow::{Result, bail};

use super::{Command, Context, Output};

pub struct Use;

impl Command for Use {
    fn name(&self) -> &'static str {
        "use"
    }
    fn usage(&self) -> &'static str {
        "[+projekt]"
    }
    fn summary(&self) -> &'static str {
        "aktives Projekt setzen bzw. aufheben"
    }
    fn help(&self) -> &'static str {
        "/use [+projekt]

Setzt das aktive Projekt; der Prompt zeigt es an (todo +digra>).
- Neue Aufgaben ohne eigenes +projekt bekommen es automatisch.
- /ls zeigt nur noch dessen Aufgaben; /ls * zeigt trotzdem alle.
/use ohne Argument hebt es auf. Das + darf fehlen.

Beispiele:
  /use +digra
  /use"
    }

    fn run(&self, args: &str, ctx: &mut Context) -> Result<Output> {
        let args = args.trim();
        if args.is_empty() {
            return Ok(Output::Text(match ctx.state.active_project.take() {
                Some(p) => format!("Aktives Projekt {p} aufgehoben"),
                None => "Kein aktives Projekt".into(),
            }));
        }
        let name = args.strip_prefix('+').unwrap_or(args);
        if args.split_whitespace().count() != 1 || name.is_empty() || name.starts_with(['+', '@', '-']) {
            bail!("Genau ein Projekt angeben – z. B. /use +digra");
        }
        let project = format!("+{name}");
        let wanted = project.to_lowercase();
        let known = ctx.state.tasks().any(|(_, t)| t.projects.iter().any(|p| p.to_lowercase() == wanted));
        let mut out = format!("Aktives Projekt: {project}");
        if !known {
            out.push_str(" (noch keine Aufgaben)");
        }
        ctx.state.active_project = Some(project);
        Ok(Output::Text(out))
    }
}
