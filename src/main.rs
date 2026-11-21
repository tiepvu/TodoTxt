use std::env;
use std::fs;
use std::path::{PathBuf, absolute};
use std::process::ExitCode;

use clap::Parser;

use todo::app::State;
use todo::config::{self, Config};
use todo::store::Store;
use todo::{repl, terminal};

/// Aufgaben in todo.txt verwalten – interaktiv mit Freitext und /befehlen.
#[derive(Parser)]
#[command(version, about)]
struct Args {
    /// Pfad zu todo.txt
    #[arg(short = 'f', long = "file", value_name = "PFAD")]
    file: Option<PathBuf>,

    /// ohne Farben
    #[arg(long)]
    plain: bool,

    /// Befehle auf der Kommandozeile sind nicht vorgesehen (SPEC 2)
    #[arg(hide = true, trailing_var_arg = true)]
    rest: Vec<String>,
}

fn fail(msg: impl std::fmt::Display) -> ExitCode {
    eprintln!("todo: {msg}");
    ExitCode::from(1)
}

fn main() -> ExitCode {
    let args = Args::parse();
    if !args.rest.is_empty() {
        eprintln!(
            "todo: keine Befehle auf der Kommandozeile – starte `todo` und gib dort z. B. /{} ein",
            args.rest[0]
        );
        return ExitCode::from(2);
    }

    // Konfiguration
    let mut cfg = Config::default();
    if let Some(path) = config::config_path()
        && let Ok(text) = fs::read_to_string(&path) {
            match config::parse(&text) {
                Ok((c, warnings)) => {
                    for w in warnings {
                        eprintln!("todo: Warnung in {}: {w}", path.display());
                    }
                    cfg = c;
                }
                Err(e) => return fail(format!("{}: {e}", path.display())),
            }
        }

    // Datei: -f, TODO_FILE, Konfiguration, ~/todo.txt
    let file = match args
        .file
        .clone()
        .or_else(|| env::var("TODO_FILE").ok().filter(|s| !s.is_empty()).map(|s| config::expand_tilde(&s)))
        .or_else(|| cfg.file.clone())
        .or_else(|| dirs::home_dir().map(|h| h.join("todo.txt")))
    {
        Some(f) => f,
        None => return fail("kein Home-Verzeichnis gefunden – Datei mit -f angeben"),
    };
    let file = match absolute(&file) {
        Ok(f) => f,
        Err(e) => return fail(format!("{}: {e}", file.display())),
    };
    let store = Store::new(file.clone());
    match store.target().parent() {
        Some(dir) if dir.is_dir() => {}
        Some(dir) => return fail(format!("Verzeichnis {} existiert nicht", dir.display())),
        None => return fail(format!("ungültiger Pfad {}", file.display())),
    }

    if let Err(msg) = terminal::check_interactive() {
        return fail(msg);
    }
    let color = terminal::use_color(args.plain, cfg.color);

    let state = match State::open(store, cfg, color) {
        Ok(s) => s,
        Err(e) => return fail(format!("{e:#}")),
    };

    println!("todo {} · {} · /help zeigt alle Befehle", env!("CARGO_PKG_VERSION"), file.display());
    if !state.file_exists() {
        println!("{} existiert noch nicht – wird beim ersten Speichern angelegt", file.display());
    }

    match repl::run(state, config::history_path()) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => fail(format!("{e:#}")),
    }
}
