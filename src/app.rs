//! Zustand einer REPL-Sitzung: geladene Datei, Fingerabdruck, Undo, Konfiguration.

use anyhow::{Context as _, Result, bail};
use chrono::NaiveDate;

use crate::config::Config;
use crate::parser;
use crate::render::Renderer;
use crate::store::{Document, Store, fingerprint};
use crate::task::Task;
use crate::undo::{UndoStack, UndoStep};

pub struct State {
    pub store: Store,
    pub doc: Document,
    /// Fingerabdruck des geladenen Inhalts = Stand, zu dem die angezeigten Nummern passen
    pub fp: Option<u64>,
    pub config: Config,
    pub undo: UndoStack,
    pub renderer: Renderer,
    /// Fest vorgegebenes „heute“ (Tests)
    pub today_override: Option<NaiveDate>,
}

impl State {
    pub fn open(store: Store, config: Config, color: bool) -> Result<State> {
        let mut state = State {
            store,
            doc: Document::default(),
            fp: None,
            config,
            undo: UndoStack::default(),
            renderer: Renderer { color },
            today_override: None,
        };
        state.reload()?;
        Ok(state)
    }

    pub fn today(&self) -> NaiveDate {
        self.today_override.unwrap_or_else(|| chrono::Local::now().date_naive())
    }

    pub fn file_exists(&self) -> bool {
        self.fp.is_some()
    }

    pub fn reload(&mut self) -> Result<()> {
        let raw = self.store.read_raw()?;
        self.set_loaded(raw)
    }

    fn set_loaded(&mut self, raw: Option<Vec<u8>>) -> Result<()> {
        self.doc = match &raw {
            Some(b) => Document::parse(b).with_context(|| format!("{} kann nicht gelesen werden", self.store.name()))?,
            None => Document::default(),
        };
        self.fp = fingerprint(raw.as_deref());
        Ok(())
    }

    /// Wurde die Datei seit dem letzten Lesen geändert?
    pub fn disk_changed(&self) -> Result<bool> {
        Ok(fingerprint(self.store.read_raw()?.as_deref()) != self.fp)
    }

    pub fn external_change_msg(&self) -> String {
        format!("{} wurde extern geändert – Nummern bitte mit /ls prüfen", self.store.name())
    }

    /// Speichert und legt einen Undo-Schritt an.
    pub fn save(&mut self, doc: Document, label: impl Into<String>) -> Result<()> {
        let before = self.write_checked(&doc)?;
        self.undo.push(UndoStep { label: label.into(), before, after: self.fp });
        Ok(())
    }

    /// Schreibt `doc`, wenn die Datei unmittelbar vorher noch dem geladenen Stand
    /// entspricht; liefert den bisherigen Inhalt.
    pub fn write_checked(&mut self, doc: &Document) -> Result<Option<Vec<u8>>> {
        let current = self.store.read_raw()?;
        if fingerprint(current.as_deref()) != self.fp {
            self.set_loaded(current)?;
            bail!("{} wurde extern geändert – nichts gespeichert. Nummern bitte mit /ls prüfen", self.store.name());
        }
        let bytes = doc.to_bytes();
        self.store.write(&bytes)?;
        self.doc = doc.clone();
        self.fp = fingerprint(Some(&bytes));
        Ok(current)
    }

    /// Alle nicht-leeren Zeilen mit Nummer (1-basiert).
    pub fn tasks(&self) -> impl Iterator<Item = (usize, Task)> + '_ {
        self.doc
            .lines
            .iter()
            .enumerate()
            .filter(|(_, l)| !l.is_blank())
            .map(|(i, l)| (i + 1, parser::parse(&l.text)))
    }
}
