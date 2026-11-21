use anyhow::{Result, bail};

use super::{Command, Context, Output};

pub struct Help;

impl Command for Help {
    fn name(&self) -> &'static str {
        "help"
    }
    fn aliases(&self) -> &'static [&'static str] {
        &["h"]
    }
    fn usage(&self) -> &'static str {
        "[Befehl]"
    }
    fn summary(&self) -> &'static str {
        "Hilfe"
    }
    fn help(&self) -> &'static str {
        "/help [Befehl]  ·  Kurz: /h

Ohne Argument: Übersicht aller Befehle. Mit Befehl: Beschreibung und Beispiele.

Beispiele:
  /help
  /help ls"
    }

    fn run(&self, args: &str, ctx: &mut Context) -> Result<Output> {
        let args = args.trim();
        if !args.is_empty() {
            return match ctx.registry.find(args) {
                Some(c) => Ok(Output::Text(c.help().to_string())),
                None => bail!("Unbekannter Befehl /{} – /help zeigt alle Befehle", args.trim_start_matches('/')),
            };
        }
        let rows: Vec<(String, &str)> = ctx
            .registry
            .commands()
            .iter()
            .map(|c| {
                let mut names = format!("/{}", c.name());
                for a in c.aliases() {
                    names.push_str(&format!(", /{a}"));
                }
                if !c.usage().is_empty() {
                    names.push(' ');
                    names.push_str(c.usage());
                }
                (names, c.summary())
            })
            .collect();
        let width = rows.iter().map(|(n, _)| n.chars().count()).max().unwrap_or(0);
        let mut out = String::from("Freitext ohne / fügt eine Aufgabe hinzu. Nummern = Zeilennummern in todo.txt.\n\n");
        for (names, summary) in rows {
            let pad = width - names.chars().count();
            out.push_str(&format!("  {names}{}  {summary}\n", " ".repeat(pad)));
        }
        out.push_str("\n/help <befehl> zeigt Details, z. B. /help ls · Strg+D beendet");
        Ok(Output::Text(out))
    }
}
