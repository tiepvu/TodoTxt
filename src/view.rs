//! Auswahl und Sortierung für `/ls` (SPEC 6.2, 8).

use std::cmp::Ordering;

use anyhow::Result;

use crate::app::State;
use crate::config::SortKey;
use crate::filter::Filter;
use crate::parser;
use crate::task::Task;

#[derive(Debug, Clone)]
pub struct Entry {
    /// Zeilennummer in todo.txt; `None` für Einträge aus done.txt
    pub num: Option<usize>,
    pub task: Task,
}

#[derive(Debug, Clone)]
pub struct Listing {
    pub entries: Vec<Entry>,
    /// Größe der Grundmenge ohne Filterbegriffe (`m` in „n von m“)
    pub total: usize,
    /// erledigte Aufgaben, die noch in todo.txt stehen
    pub done_in_todo: usize,
}

pub fn listing(state: &State, filter: &Filter) -> Result<Listing> {
    let mut pool: Vec<Entry> = state.tasks().map(|(n, task)| Entry { num: Some(n), task }).collect();
    let done_in_todo = pool.iter().filter(|e| e.task.done).count();
    if filter.status.includes_done_file() {
        pool.extend(state.store.read_done()?.iter().map(|raw| Entry { num: None, task: parser::parse(raw) }));
    }
    let base: Vec<Entry> = pool.into_iter().filter(|e| filter.status.accepts(&e.task)).collect();
    let total = base.len();
    let mut entries: Vec<Entry> = base.into_iter().filter(|e| filter.matches_terms(&e.task)).collect();
    sort(&mut entries, &state.config.default_sort);
    Ok(Listing { entries, total, done_in_todo })
}

/// Stabil: bei Gleichstand bleibt die Reihenfolge der Datei (todo.txt vor done.txt).
pub fn sort(entries: &mut [Entry], keys: &[SortKey]) {
    entries.sort_by(|a, b| {
        keys.iter()
            .map(|k| compare(&a.task, &b.task, *k))
            .find(|o| *o != Ordering::Equal)
            .unwrap_or(Ordering::Equal)
    });
}

fn compare(a: &Task, b: &Task, key: SortKey) -> Ordering {
    match key {
        SortKey::Priority => none_last(sort_priority(a), sort_priority(b)),
        SortKey::Due => none_last(a.due, b.due),
        SortKey::Created => none_last(a.created, b.created),
    }
}

/// Erledigte Aufgaben zählen beim Sortieren als „ohne Priorität“.
fn sort_priority(t: &Task) -> Option<char> {
    if t.done { None } else { t.priority }
}

fn none_last<T: Ord>(a: Option<T>, b: Option<T>) -> Ordering {
    match (a, b) {
        (Some(x), Some(y)) => x.cmp(&y),
        (Some(_), None) => Ordering::Less,
        (None, Some(_)) => Ordering::Greater,
        (None, None) => Ordering::Equal,
    }
}
