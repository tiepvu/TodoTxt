use anyhow::Result;

use super::{Command, Context, Output};

pub struct Quit;

impl Command for Quit {
    fn name(&self) -> &'static str {
        "quit"
    }
    fn aliases(&self) -> &'static [&'static str] {
        &["q"]
    }
    fn summary(&self) -> &'static str {
        "beenden (auch Strg+D)"
    }
    fn help(&self) -> &'static str {
        "/quit  ·  Kurz: /q  ·  oder Strg+D

Beendet todo."
    }

    fn run(&self, _args: &str, _ctx: &mut Context) -> Result<Output> {
        Ok(Output::Quit)
    }
}
