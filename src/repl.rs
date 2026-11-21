//! Eingabeschleife mit History und Tab-Vervollständigung (SPEC 5.2).

use std::collections::BTreeSet;
use std::path::PathBuf;

use rustyline::completion::{Completer, Pair};
use rustyline::error::ReadlineError;
use rustyline::highlight::Highlighter;
use rustyline::hint::Hinter;
use rustyline::history::DefaultHistory;
use rustyline::validate::Validator;
use rustyline::{CompletionType, Config, Editor, Helper};

use crate::app::State;
use crate::commands::{Output, Prompter, Registry, execute};

/// Vervollständigt Befehle (am Zeilenanfang), `+projekte` und `@kontexte`.
#[derive(Default)]
pub struct ReplHelper {
    commands: Vec<String>,
    tokens: Vec<String>,
}

impl ReplHelper {
    fn refresh(&mut self, registry: &Registry, state: &State) {
        self.commands = registry.completion_words();
        let mut set = BTreeSet::new();
        for (_, t) in state.tasks() {
            set.extend(t.projects);
            set.extend(t.contexts);
        }
        self.tokens = set.into_iter().collect();
    }
}

impl Completer for ReplHelper {
    type Candidate = Pair;

    fn complete(&self, line: &str, pos: usize, _ctx: &rustyline::Context<'_>) -> rustyline::Result<(usize, Vec<Pair>)> {
        let before = &line[..pos];
        let start = before.rfind(char::is_whitespace).map_or(0, |i| i + before[i..].chars().next().map_or(1, char::len_utf8));
        let word = &before[start..];
        if word.is_empty() {
            return Ok((start, Vec::new()));
        }
        let lower = word.to_lowercase();
        let pool: &[String] = if start == 0 && word.starts_with('/') { &self.commands } else { &self.tokens };
        let candidates = pool
            .iter()
            .filter(|c| c.to_lowercase().starts_with(&lower))
            .map(|c| Pair { display: c.clone(), replacement: c.clone() })
            .collect();
        Ok((start, candidates))
    }
}

impl Hinter for ReplHelper {
    type Hint = String;
}
impl Highlighter for ReplHelper {}
impl Validator for ReplHelper {}
impl Helper for ReplHelper {}

type ReplEditor = Editor<ReplHelper, DefaultHistory>;

struct EditorPrompter<'a> {
    editor: &'a mut ReplEditor,
}

impl Prompter for EditorPrompter<'_> {
    fn show(&mut self, text: &str) {
        println!("{text}");
    }

    fn confirm(&mut self, question: &str) -> bool {
        match self.editor.readline(question) {
            Ok(answer) => matches!(answer.trim().to_lowercase().as_str(), "j" | "ja" | "y" | "yes"),
            Err(_) => false,
        }
    }

    fn edit_line(&mut self, initial: &str) -> Option<String> {
        self.editor.readline_with_initial("bearbeiten> ", (initial, "")).ok()
    }
}

pub fn run(mut state: State, history: Option<PathBuf>) -> anyhow::Result<()> {
    let registry = Registry::new();
    let config = Config::builder()
        .auto_add_history(false)
        .completion_type(CompletionType::List)
        .history_ignore_dups(true)?
        .max_history_size(1000)?
        .build();
    let mut editor: ReplEditor = Editor::with_config(config)?;
    let mut helper = ReplHelper::default();
    helper.refresh(&registry, &state);
    editor.set_helper(Some(helper));
    if let Some(path) = &history {
        let _ = editor.load_history(path);
    }

    loop {
        let line = match editor.readline(&state.prompt()) {
            Ok(line) => line,
            Err(ReadlineError::Interrupted) => continue,
            Err(ReadlineError::Eof) => break,
            Err(e) => return Err(e.into()),
        };
        if !line.trim().is_empty() {
            let _ = editor.add_history_entry(line.as_str());
            save_history(&mut editor, history.as_ref());
        }
        let result = {
            let mut ui = EditorPrompter { editor: &mut editor };
            execute(&registry, &line, &mut state, &mut ui)
        };
        match result {
            Ok(Output::Text(text)) => println!("{text}"),
            Ok(Output::Nothing) => {}
            Ok(Output::Quit) => break,
            Err(e) => println!("{}", state.renderer.error(&format!("Fehler: {e:#}"))),
        }
        if let Some(h) = editor.helper_mut() {
            h.refresh(&registry, &state);
        }
    }
    save_history(&mut editor, history.as_ref());
    Ok(())
}

fn save_history(editor: &mut ReplEditor, path: Option<&PathBuf>) {
    if let Some(path) = path {
        if let Some(dir) = path.parent() {
            let _ = std::fs::create_dir_all(dir);
        }
        let _ = editor.save_history(path);
    }
}
