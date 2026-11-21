//! `/projects` und `/contexts`: Tokens mit Anzahl offener Aufgaben (SPEC 6.2).

use anyhow::Result;

use super::{Command, Context, Output};
use crate::app::State;
use crate::task::Task;

/// Zählt offene Aufgaben je Token; Groß-/Kleinschreibung zählt nicht (wie beim Filter).
/// Angezeigt wird die erste Schreibweise in der Datei. Sortiert nach Anzahl, dann Name.
fn counts(state: &State, tokens: fn(&Task) -> &[String]) -> Vec<(String, usize)> {
    let mut rows: Vec<(String, String, usize)> = Vec::new(); // (klein, Anzeige, Anzahl)
    for (_, task) in state.tasks().filter(|(_, t)| !t.done) {
        let mut seen: Vec<String> = Vec::new();
        for tok in tokens(&task) {
            let key = tok.to_lowercase();
            if seen.contains(&key) {
                continue;
            }
            match rows.iter_mut().find(|r| r.0 == key) {
                Some(row) => row.2 += 1,
                None => rows.push((key.clone(), tok.clone(), 1)),
            }
            seen.push(key);
        }
    }
    rows.sort_by(|a, b| b.2.cmp(&a.2).then_with(|| a.0.cmp(&b.0)));
    rows.into_iter().map(|(_, name, n)| (name, n)).collect()
}

fn table(rows: &[(String, usize)], empty: &str) -> String {
    if rows.is_empty() {
        return empty.to_string();
    }
    let width = rows.iter().map(|(name, _)| name.chars().count()).max().unwrap_or(0);
    let num_width = rows.iter().map(|(_, n)| n.to_string().len()).max().unwrap_or(1);
    rows.iter()
        .map(|(name, n)| format!("{name}{}  {n:>num_width$}", " ".repeat(width - name.chars().count())))
        .collect::<Vec<_>>()
        .join("\n")
}

pub struct Projects;

impl Command for Projects {
    fn name(&self) -> &'static str {
        "projects"
    }
    fn aliases(&self) -> &'static [&'static str] {
        &["pj"]
    }
    fn summary(&self) -> &'static str {
        "alle Projekte mit Anzahl offener Aufgaben"
    }
    fn help(&self) -> &'static str {
        "/projects  ·  Kurz: /pj

Listet alle +projekte aus todo.txt mit der Anzahl offener Aufgaben,
die meisten zuerst. Erledigte Aufgaben zählen nicht mit."
    }

    fn run(&self, _args: &str, ctx: &mut Context) -> Result<Output> {
        let r = &ctx.state.renderer;
        let rows = counts(ctx.state, |t| &t.projects);
        let out = table(&rows, "Keine Projekte in offenen Aufgaben");
        Ok(Output::Text(if rows.is_empty() { out } else { r.tokens(&out, ctx.state.today()) }))
    }
}

pub struct Contexts;

impl Command for Contexts {
    fn name(&self) -> &'static str {
        "contexts"
    }
    fn aliases(&self) -> &'static [&'static str] {
        &["cx"]
    }
    fn summary(&self) -> &'static str {
        "alle Kontexte mit Anzahl offener Aufgaben"
    }
    fn help(&self) -> &'static str {
        "/contexts  ·  Kurz: /cx

Listet alle @kontexte aus todo.txt mit der Anzahl offener Aufgaben,
die meisten zuerst. Erledigte Aufgaben zählen nicht mit."
    }

    fn run(&self, _args: &str, ctx: &mut Context) -> Result<Output> {
        let r = &ctx.state.renderer;
        let rows = counts(ctx.state, |t| &t.contexts);
        let out = table(&rows, "Keine Kontexte in offenen Aufgaben");
        Ok(Output::Text(if rows.is_empty() { out } else { r.tokens(&out, ctx.state.today()) }))
    }
}
