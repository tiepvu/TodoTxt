use anyhow::Result;

use super::{Command, Context, Output};
use crate::app::State;
use crate::{filter, view};

pub struct Ls;

/// Liste samt Fußzeile – auch von `/archive` verwendet.
pub fn render_listing(state: &State, args: &str) -> Result<String> {
    let today = state.today();
    let f = filter::parse(args, today)?;
    let listing = view::listing(state, &f)?;
    let r = &state.renderer;
    let mut out = r.list(&listing.entries, today);
    out.push_str(&r.footer(listing.entries.len(), listing.total, listing.done_in_todo));
    Ok(out)
}

impl Command for Ls {
    fn name(&self) -> &'static str {
        "ls"
    }
    fn aliases(&self) -> &'static [&'static str] {
        &["l"]
    }
    fn usage(&self) -> &'static str {
        "[Filter]"
    }
    fn summary(&self) -> &'static str {
        "Aufgaben anzeigen und durchsuchen"
    }
    fn help(&self) -> &'static str {
        "/ls [Filter]  ·  Kurz: /l

Zeigt offene Aufgaben mit ihrer Nummer (= Zeilennummer in todo.txt), sortiert nach
Priorität, Fälligkeit und Erstellungsdatum. Alle Filter werden UND-verknüpft,
Groß-/Kleinschreibung zählt nicht.

  wort  -wort           Text enthält / enthält nicht
  +projekt  -+projekt   hat Projekt / hat es nicht
  @kontext  -@kontext   hat Kontext / hat ihn nicht
  pri:A  pri:A-C  pri:none
  due<7d  due<=2026-12-01  due=today  due:none
  created>3d  created>=1w
  status:all            auch erledigte (inkl. done.txt)
  status:done           nur erledigte (inkl. done.txt)
  *                     aktives Projekt (/use) ignorieren

Datum: JJJJ-MM-TT, today oder N mit d/w/m. Bei due zählt N in die Zukunft,
bei created in die Vergangenheit. < und > ohne Stichtag, <= und >= mit.

Beispiele:
  /ls +digra @rechner
  /ls login -test
  /ls pri:A due<7d
  /ls created>=3d status:all"
    }

    fn run(&self, args: &str, ctx: &mut Context) -> Result<Output> {
        Ok(Output::Text(render_listing(ctx.state, args)?))
    }
}
