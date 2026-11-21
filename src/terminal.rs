//! Terminal-Prüfung beim Start und Farbentscheidung (SPEC 3, 9).

use std::env;
use std::io::{self, IsTerminal};

/// Prüft, ob die REPL laufen kann; sonst Fehlermeldung mit Hinweisen.
pub fn check_interactive() -> Result<(), String> {
    let ok = io::stdin().is_terminal() && io::stdout().is_terminal() && console_mode_available();
    if ok {
        return Ok(());
    }
    let mut msg = String::from("braucht ein interaktives Terminal (keine Pipe oder Umleitung).");
    if is_mintty() {
        msg.push_str(
            "\nIn mintty (Git Bash): `winpty todo` verwenden oder in Git for Windows die \
             Unterstützung für Pseudo-Konsolen (ConPTY) aktivieren – oder Windows Terminal nutzen.",
        );
    }
    Err(msg)
}

fn is_mintty() -> bool {
    env::var("TERM_PROGRAM").is_ok_and(|v| v.eq_ignore_ascii_case("mintty")) || env::var_os("MSYSTEM").is_some()
}

/// Unter Windows meldet `IsTerminal` auch MSYS-Pseudoterminals (mintty ohne ConPTY).
/// Entscheidend ist, ob die Konsolen-Schnittstelle für die Zeilenbearbeitung verfügbar ist.
#[cfg(windows)]
fn console_mode_available() -> bool {
    use windows_sys::Win32::System::Console::{GetConsoleMode, GetStdHandle, STD_INPUT_HANDLE};
    let mut mode = 0;
    // SAFETY: GetStdHandle/GetConsoleMode mit gültigem Zeiger auf eine lokale Variable.
    unsafe {
        let handle = GetStdHandle(STD_INPUT_HANDLE);
        GetConsoleMode(handle, &mut mode) != 0
    }
}

#[cfg(not(windows))]
fn console_mode_available() -> bool {
    true
}

/// Farben laut Vorrang: --plain > NO_COLOR > color = false > Terminal ohne Farben.
pub fn use_color(plain: bool, config_color: bool) -> bool {
    if plain {
        return false;
    }
    if env::var_os("NO_COLOR").is_some_and(|v| !v.is_empty()) {
        return false;
    }
    if !config_color {
        return false;
    }
    if env::var("TERM").is_ok_and(|t| t == "dumb") {
        return false;
    }
    enable_ansi()
}

/// Windows: ANSI-Farben in der Konsole einschalten. Liefert false, wenn das nicht geht.
#[cfg(windows)]
fn enable_ansi() -> bool {
    use windows_sys::Win32::System::Console::{
        ENABLE_VIRTUAL_TERMINAL_PROCESSING, GetConsoleMode, GetStdHandle, STD_OUTPUT_HANDLE, SetConsoleMode,
    };
    let mut mode = 0;
    // SAFETY: Aufrufe mit gültigem Handle bzw. Zeiger auf eine lokale Variable.
    unsafe {
        let handle = GetStdHandle(STD_OUTPUT_HANDLE);
        if GetConsoleMode(handle, &mut mode) == 0 {
            return false;
        }
        mode & ENABLE_VIRTUAL_TERMINAL_PROCESSING != 0
            || SetConsoleMode(handle, mode | ENABLE_VIRTUAL_TERMINAL_PROCESSING) != 0
    }
}

#[cfg(not(windows))]
fn enable_ansi() -> bool {
    true
}
