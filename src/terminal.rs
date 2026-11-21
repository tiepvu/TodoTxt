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

/// Lässt sich die Eingabe tatsächlich in den Raw Mode schalten? (SPEC 3)
///
/// `IsTerminal` allein reicht nicht: Unter Windows meldet es auch MSYS-Pseudoterminals
/// (mintty ohne ConPTY). Deshalb wird der Modus probeweise gesetzt und sofort
/// wiederhergestellt.
#[cfg(windows)]
fn console_mode_available() -> bool {
    use windows_sys::Win32::System::Console::{
        ENABLE_ECHO_INPUT, ENABLE_LINE_INPUT, ENABLE_PROCESSED_INPUT, GetConsoleMode, GetStdHandle,
        STD_INPUT_HANDLE, SetConsoleMode,
    };
    let mut mode = 0;
    // SAFETY: GetStdHandle/GetConsoleMode/SetConsoleMode mit gültigem Handle bzw.
    // Zeiger auf eine lokale Variable; der ursprüngliche Modus wird wiederhergestellt.
    unsafe {
        let handle = GetStdHandle(STD_INPUT_HANDLE);
        if GetConsoleMode(handle, &mut mode) == 0 {
            return false;
        }
        let raw = mode & !(ENABLE_LINE_INPUT | ENABLE_ECHO_INPUT | ENABLE_PROCESSED_INPUT);
        if SetConsoleMode(handle, raw) == 0 {
            return false;
        }
        SetConsoleMode(handle, mode);
    }
    true
}

#[cfg(unix)]
fn console_mode_available() -> bool {
    // SAFETY: termios ist ein reines Datenstruktur-Abbild; tcgetattr/tcsetattr erhalten
    // gültige Zeiger auf lokale Variablen, der ursprüngliche Modus wird wiederhergestellt.
    unsafe {
        let fd = libc::STDIN_FILENO;
        let mut original: libc::termios = std::mem::zeroed();
        if libc::tcgetattr(fd, &mut original) != 0 {
            return false;
        }
        let mut raw = original;
        libc::cfmakeraw(&mut raw);
        if libc::tcsetattr(fd, libc::TCSANOW, &raw) != 0 {
            return false;
        }
        libc::tcsetattr(fd, libc::TCSANOW, &original);
    }
    true
}

#[cfg(not(any(unix, windows)))]
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
