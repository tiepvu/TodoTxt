use anyhow::{Result, bail};

use super::{Command, Context, Output};
use crate::store::fingerprint;

pub struct Undo;

impl Command for Undo {
    fn name(&self) -> &'static str {
        "undo"
    }
    fn aliases(&self) -> &'static [&'static str] {
        &["u"]
    }
    fn summary(&self) -> &'static str {
        "letzte Änderung rückgängig"
    }
    fn help(&self) -> &'static str {
        "/undo  ·  Kurz: /u

Macht die letzte Änderung dieser Sitzung rückgängig; mehrfach aufrufbar.
Abgelehnt, wenn todo.txt inzwischen extern geändert wurde. /archive lässt sich
nicht rückgängig machen und leert den Undo-Stapel."
    }
    // Prüft externe Änderungen selbst gegen den Stand nach dem Undo-Schritt,
    // um den Stapel dabei zu leeren (SPEC 6.2 /undo).

    fn run(&self, _args: &str, ctx: &mut Context) -> Result<Output> {
        let state = &mut *ctx.state;
        let Some(step) = state.undo.pop() else {
            let msg = if state.undo.cleared_by_archive() {
                "Nichts rückgängig zu machen (seit /archive)"
            } else {
                "Nichts rückgängig zu machen"
            };
            return Ok(Output::Text(msg.into()));
        };
        let current = state.store.read_raw()?;
        if fingerprint(current.as_deref()) != step.after {
            state.undo.clear(false);
            state.reload()?;
            bail!("{} wurde extern geändert – Undo nicht möglich", state.store.name());
        }
        match &step.before {
            Some(bytes) => state.store.write(bytes)?,
            None => state.store.remove()?,
        }
        state.reload()?;
        Ok(Output::Text(format!("Rückgängig: {}", step.label)))
    }
}
