//! Zeile ↔ Task. Gelesen wird verlustfrei; Änderungen erfolgen minimal am Rohtext.

use chrono::NaiveDate;

use crate::task::Task;

pub fn format_date(d: NaiveDate) -> String {
    d.format("%Y-%m-%d").to_string()
}

/// Streng `JJJJ-MM-TT`.
pub fn parse_date(s: &str) -> Option<NaiveDate> {
    let b = s.as_bytes();
    if b.len() != 10 || b[4] != b'-' || b[7] != b'-' {
        return None;
    }
    if !b.iter().enumerate().all(|(i, c)| i == 4 || i == 7 || c.is_ascii_digit()) {
        return None;
    }
    NaiveDate::parse_from_str(s, "%Y-%m-%d").ok()
}

/// Datum am Anfang, gefolgt von Leerzeichen oder Zeilenende.
pub fn take_date(s: &str) -> Option<(NaiveDate, &str)> {
    if s.len() < 10 || !s.is_char_boundary(10) {
        return None;
    }
    let (d, rest) = s.split_at(10);
    let date = parse_date(d)?;
    if rest.is_empty() {
        Some((date, rest))
    } else {
        rest.strip_prefix(' ').map(|r| (date, r))
    }
}

/// `(A) ` am Anfang.
pub fn take_priority(s: &str) -> Option<(char, &str)> {
    let b = s.as_bytes();
    if b.len() >= 4 && b[0] == b'(' && b[1].is_ascii_uppercase() && b[2] == b')' && b[3] == b' ' {
        Some((b[1] as char, &s[4..]))
    } else {
        None
    }
}

pub fn parse(raw: &str) -> Task {
    let mut t = Task::default();
    let mut rest = raw;
    if let Some(r) = rest.strip_prefix("x ") {
        t.done = true;
        rest = r;
        if let Some((d, r)) = take_date(rest) {
            t.completed = Some(d);
            rest = r;
            if let Some((d, r)) = take_date(rest) {
                t.created = Some(d);
                rest = r;
            }
        }
    } else {
        if let Some((p, r)) = take_priority(rest) {
            t.priority = Some(p);
            rest = r;
        }
        if let Some((d, r)) = take_date(rest) {
            t.created = Some(d);
            rest = r;
        }
    }
    t.body = rest.to_string();
    for tok in rest.split_whitespace() {
        if tok.len() > 1 && tok.starts_with('+') {
            t.projects.push(tok.to_string());
        } else if tok.len() > 1 && tok.starts_with('@') {
            t.contexts.push(tok.to_string());
        } else if let Some((key, value)) = tok.split_once(':') {
            match key {
                "due" if t.due.is_none() => t.due = parse_date(value),
                "pri" if t.pri_tag.is_none() => {
                    let b = value.as_bytes();
                    if b.len() == 1 && b[0].is_ascii_uppercase() {
                        t.pri_tag = Some(b[0] as char);
                    }
                }
                _ => {}
            }
        }
    }
    t
}

/// `/done`: `x ` + Datum voran, Priorität wird zu `pri:X` am Ende.
/// Der Rest der Zeile bleibt byte-genau erhalten.
pub fn mark_done(raw: &str, today: NaiveDate) -> String {
    let (prio, rest) = match take_priority(raw) {
        Some((p, r)) => (Some(p), r),
        None => (None, raw),
    };
    let mut out = format!("x {}", format_date(today));
    if !rest.is_empty() {
        out.push(' ');
        out.push_str(rest);
    }
    if let Some(p) = prio {
        if !out.ends_with(char::is_whitespace) {
            out.push(' ');
        }
        out.push_str("pri:");
        out.push(p);
    }
    out
}

pub const ERR_STARTS_WITH_X: &str =
    "Text beginnt mit „x “ und würde als erledigt gelten – bitte umformulieren";

/// `/add`: Priorität übernehmen, Erstellungsdatum ergänzen.
pub fn build_add(text: &str, today: NaiveDate, date_on_add: bool) -> Result<String, &'static str> {
    let text = text.trim();
    if text.starts_with("x ") {
        return Err(ERR_STARTS_WITH_X);
    }
    let (prio, rest) = match take_priority(text) {
        Some((p, r)) => (Some(p), r),
        None => (None, text),
    };
    let mut out = String::new();
    if let Some(p) = prio {
        out.push('(');
        out.push(p);
        out.push_str(") ");
    }
    if date_on_add && take_date(rest).is_none() {
        out.push_str(&format_date(today));
        out.push(' ');
    }
    out.push_str(rest);
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn d(s: &str) -> NaiveDate {
        parse_date(s).unwrap()
    }

    #[test]
    fn parses_open_task() {
        let t = parse("(A) 2026-11-20 Login-Bug fixen +digra @rechner due:2026-11-27 url:http://x");
        assert!(!t.done);
        assert_eq!(t.priority, Some('A'));
        assert_eq!(t.created, Some(d("2026-11-20")));
        assert_eq!(t.projects, vec!["+digra"]);
        assert_eq!(t.contexts, vec!["@rechner"]);
        assert_eq!(t.due, Some(d("2026-11-27")));
        assert_eq!(t.body, "Login-Bug fixen +digra @rechner due:2026-11-27 url:http://x");
    }

    #[test]
    fn parses_done_task() {
        let t = parse("x 2026-11-21 2026-11-19 Keycloak prüfen +keycloak pri:B");
        assert!(t.done);
        assert_eq!(t.completed, Some(d("2026-11-21")));
        assert_eq!(t.created, Some(d("2026-11-19")));
        assert_eq!(t.pri_tag, Some('B'));
        assert_eq!(t.effective_priority(), Some('B'));
    }

    #[test]
    fn ignores_non_tokens() {
        let t = parse("C++ lernen a+b @ + due:morgen");
        assert!(t.projects.is_empty());
        assert!(t.contexts.is_empty());
        assert_eq!(t.due, None);
    }

    #[test]
    fn lowercase_priority_is_text() {
        assert_eq!(parse("(a) Text").priority, None);
        assert_eq!(parse("(A)Text").priority, None);
    }

    #[test]
    fn mark_done_moves_priority() {
        let today = d("2026-11-21");
        assert_eq!(
            mark_done("(A) 2026-11-20 Login  fixen +digra", today),
            "x 2026-11-21 2026-11-20 Login  fixen +digra pri:A"
        );
        assert_eq!(mark_done("Ohne Datum", today), "x 2026-11-21 Ohne Datum");
    }

    #[test]
    fn build_add_rules() {
        let today = d("2026-11-21");
        assert_eq!(build_add("  Bug fixen ", today, true).unwrap(), "2026-11-21 Bug fixen");
        assert_eq!(build_add("(B) Bug", today, true).unwrap(), "(B) 2026-11-21 Bug");
        assert_eq!(build_add("2026-01-01 Alt", today, true).unwrap(), "2026-01-01 Alt");
        assert_eq!(build_add("(C) 2026-01-01 Alt", today, true).unwrap(), "(C) 2026-01-01 Alt");
        assert_eq!(build_add("Ohne", today, false).unwrap(), "Ohne");
        assert!(build_add("x Achse", today, true).is_err());
        assert_eq!(build_add("X Achse", today, false).unwrap(), "X Achse");
    }
}
