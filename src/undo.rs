//! Undo-Stapel der REPL-Sitzung (SPEC 6.2 `/undo`).

/// Mindestens 20 Schritte laut Spec.
pub const MAX_STEPS: usize = 100;

#[derive(Debug, Clone)]
pub struct UndoStep {
    /// z. B. `/done 7`
    pub label: String,
    /// Inhalt von `todo.txt` vor der Änderung; `None` = Datei existierte nicht
    pub before: Option<Vec<u8>>,
    /// Fingerabdruck nach der Änderung
    pub after: Option<u64>,
}

#[derive(Debug, Default)]
pub struct UndoStack {
    steps: Vec<UndoStep>,
    cleared_by_archive: bool,
}

impl UndoStack {
    pub fn push(&mut self, step: UndoStep) {
        if self.steps.len() >= MAX_STEPS {
            self.steps.remove(0);
        }
        self.steps.push(step);
        self.cleared_by_archive = false;
    }

    pub fn pop(&mut self) -> Option<UndoStep> {
        self.steps.pop()
    }

    pub fn clear(&mut self, by_archive: bool) {
        self.steps.clear();
        self.cleared_by_archive = by_archive;
    }

    pub fn cleared_by_archive(&self) -> bool {
        self.cleared_by_archive
    }

    pub fn len(&self) -> usize {
        self.steps.len()
    }

    pub fn is_empty(&self) -> bool {
        self.steps.is_empty()
    }
}
