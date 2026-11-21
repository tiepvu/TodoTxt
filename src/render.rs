//! Ausgabe und Farben (SPEC 8).

use chrono::NaiveDate;
use owo_colors::OwoColorize;

use crate::parser::{parse, parse_date, take_priority};
use crate::view::Entry;

#[derive(Debug, Clone, Copy)]
enum Style {
    Red,
    Yellow,
    Green,
    Blue,
    Cyan,
    Grey,
}

#[derive(Debug, Clone, Copy)]
pub struct Renderer {
    pub color: bool,
}

impl Renderer {
    fn paint(&self, s: &str, style: Style) -> String {
        if !self.color {
            return s.to_string();
        }
        match style {
            Style::Red => s.red().to_string(),
            Style::Yellow => s.yellow().to_string(),
            Style::Green => s.green().to_string(),
            Style::Blue => s.blue().to_string(),
            Style::Cyan => s.cyan().to_string(),
            Style::Grey => s.bright_black().to_string(),
        }
    }

    pub fn error(&self, s: &str) -> String {
        self.paint(s, Style::Red)
    }

    fn priority_style(p: char) -> Option<Style> {
        match p {
            'A' => Some(Style::Red),
            'B' => Some(Style::Yellow),
            'C' => Some(Style::Green),
            _ => None,
        }
    }

    /// `(A)` usw. als Priorität – nur für den Zeilenanfang einer offenen Aufgabe.
    fn priority_marker(&self, p: char) -> String {
        let marker = format!("({p})");
        match Self::priority_style(p) {
            Some(style) => self.paint(&marker, style),
            None => marker,
        }
    }

    /// Färbt ein Wort der Beschreibung; `(A)` mitten im Text ist keine Priorität.
    fn token(&self, tok: &str, today: NaiveDate) -> String {
        if tok.len() > 1 && tok.starts_with('+') {
            return self.paint(tok, Style::Blue);
        } else if tok.len() > 1 && tok.starts_with('@') {
            return self.paint(tok, Style::Cyan);
        } else if let Some(d) = tok.strip_prefix("due:").and_then(parse_date)
            && d < today {
                return self.paint(tok, Style::Red);
            }
        tok.to_string()
    }

    /// Färbt die Tokens eines Textes; Leerraum bleibt unverändert.
    pub fn tokens(&self, text: &str, today: NaiveDate) -> String {
        if !self.color {
            return text.to_string();
        }
        let mut out = String::new();
        let mut start = 0;
        let mut in_ws = None;
        for (i, ch) in text.char_indices() {
            let ws = ch.is_whitespace();
            if in_ws != Some(ws) {
                if let Some(prev_ws) = in_ws {
                    self.push_piece(&mut out, &text[start..i], prev_ws, today);
                }
                start = i;
                in_ws = Some(ws);
            }
        }
        if let Some(prev_ws) = in_ws {
            self.push_piece(&mut out, &text[start..], prev_ws, today);
        }
        out
    }

    fn push_piece(&self, out: &mut String, piece: &str, ws: bool, today: NaiveDate) {
        if ws {
            out.push_str(piece);
        } else {
            out.push_str(&self.token(piece, today));
        }
    }

    /// Eine Rohzeile (z. B. nach `/add`, `/done`); erledigte Zeilen grau.
    pub fn raw_line(&self, raw: &str, today: NaiveDate) -> String {
        if parse(raw).done {
            return self.paint(raw, Style::Grey);
        }
        match take_priority(raw) {
            Some((p, rest)) => format!("{} {}", self.priority_marker(p), self.tokens(rest, today)),
            None => self.tokens(raw, today),
        }
    }

    /// Liste mit rechtsbündigen Nummern und Spalte für Status bzw. Priorität.
    pub fn list(&self, entries: &[Entry], today: NaiveDate) -> String {
        let width = entries.iter().filter_map(|e| e.num).map(|n| n.to_string().len()).max().unwrap_or(1);
        let mut out = String::new();
        for e in entries {
            let num = match e.num {
                Some(n) => format!("{n:>width$}"),
                None => " ".repeat(width),
            };
            if e.task.done {
                let line = format!("{num}   x {}", e.task.body);
                out.push_str(&self.paint(&line, Style::Grey));
            } else {
                let col = match e.task.priority {
                    Some(p) => self.priority_marker(p),
                    None => "   ".to_string(),
                };
                out.push_str(&format!("{num} {col} {}", self.tokens(&e.task.body, today)));
            }
            out.push('\n');
        }
        out
    }

    pub fn footer(&self, shown: usize, total: usize, done_in_todo: usize) -> String {
        let mut s = format!("{shown} von {total} Aufgaben");
        if done_in_todo > 0 {
            s.push_str(&format!(" · {done_in_todo} erledigt – /archive räumt auf"));
        }
        s
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const RED: &str = "\u{1b}[31m";

    fn today() -> NaiveDate {
        parse_date("2026-11-21").unwrap()
    }

    #[test]
    fn priority_colored_only_at_line_start() {
        let r = Renderer { color: true };
        let out = r.raw_line("(A) Siehe (A) Liste", today());
        assert_eq!(out, format!("{RED}(A)\u{1b}[39m Siehe (A) Liste"));
        assert_eq!(r.raw_line("Text (A) mitten", today()), "Text (A) mitten");
    }

    #[test]
    fn overdue_due_is_red() {
        let r = Renderer { color: true };
        assert!(r.raw_line("Bug due:2026-11-20", today()).contains(&format!("{RED}due:2026-11-20")));
        assert_eq!(r.raw_line("Bug due:2026-11-21", today()), "Bug due:2026-11-21");
    }
}
