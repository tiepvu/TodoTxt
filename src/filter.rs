//! Filtersyntax (SPEC 7).

use anyhow::{Result, anyhow};
use chrono::{Days, Months, NaiveDate};

use crate::parser::parse_date;
use crate::task::Task;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Status {
    /// nur offene Aufgaben aus todo.txt
    #[default]
    Open,
    /// `status:all`: alle aus todo.txt und done.txt
    All,
    /// `status:done`: nur erledigte aus todo.txt und done.txt
    Done,
}

impl Status {
    pub fn accepts(self, t: &Task) -> bool {
        match self {
            Status::Open => !t.done,
            Status::All => true,
            Status::Done => t.done,
        }
    }

    pub fn includes_done_file(self) -> bool {
        self != Status::Open
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Cmp {
    Lt,
    Le,
    Gt,
    Ge,
    Eq,
}

impl Cmp {
    fn test(self, value: NaiveDate, reference: NaiveDate) -> bool {
        match self {
            Cmp::Lt => value < reference,
            Cmp::Le => value <= reference,
            Cmp::Gt => value > reference,
            Cmp::Ge => value >= reference,
            Cmp::Eq => value == reference,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Term {
    Word(String),
    NotWord(String),
    Project(String),
    NotProject(String),
    Context(String),
    NotContext(String),
    Pri(char, char),
    PriNone,
    DueNone,
    Due(Cmp, NaiveDate),
    Created(Cmp, NaiveDate),
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Filter {
    pub terms: Vec<Term>,
    pub status: Status,
    /// `*` – aktives Projekt (`/use`) ignorieren; angewendet in `view::listing`
    pub ignore_active: bool,
}

pub fn parse(input: &str, today: NaiveDate) -> Result<Filter> {
    let mut f = Filter::default();
    for tok in input.split_whitespace() {
        let lower = tok.to_lowercase();
        match lower.as_str() {
            "*" => {
                f.ignore_active = true;
                continue;
            }
            "status:all" => {
                f.status = Status::All;
                continue;
            }
            "status:done" => {
                f.status = Status::Done;
                continue;
            }
            "pri:none" => {
                f.terms.push(Term::PriNone);
                continue;
            }
            "due:none" => {
                f.terms.push(Term::DueNone);
                continue;
            }
            _ => {}
        }
        if let Some(range) = lower.strip_prefix("pri:").and_then(parse_pri_range) {
            f.terms.push(Term::Pri(range.0, range.1));
            continue;
        }
        if let Some((field, cmp, value)) = split_comparison(&lower) {
            let date = resolve_date(field, value, today).ok_or_else(|| {
                anyhow!("Ungültige Datumsangabe in „{tok}“ – erlaubt: JJJJ-MM-TT, today oder z. B. 7d, 2w, 1m")
            })?;
            f.terms.push(if field == "due" { Term::Due(cmp, date) } else { Term::Created(cmp, date) });
            continue;
        }
        f.terms.push(word_term(&lower));
    }
    Ok(f)
}

fn word_term(lower: &str) -> Term {
    if let Some(rest) = lower.strip_prefix('-') {
        if rest.len() > 1 && rest.starts_with('+') {
            return Term::NotProject(rest.to_string());
        }
        if rest.len() > 1 && rest.starts_with('@') {
            return Term::NotContext(rest.to_string());
        }
        if !rest.is_empty() {
            return Term::NotWord(rest.to_string());
        }
    }
    if lower.len() > 1 && lower.starts_with('+') {
        return Term::Project(lower.to_string());
    }
    if lower.len() > 1 && lower.starts_with('@') {
        return Term::Context(lower.to_string());
    }
    Term::Word(lower.to_string())
}

/// `a` oder `a-c` (bereits kleingeschrieben).
fn parse_pri_range(v: &str) -> Option<(char, char)> {
    let letter = |s: &str| {
        let mut c = s.chars();
        match (c.next(), c.next()) {
            (Some(ch), None) if ch.is_ascii_alphabetic() => Some(ch.to_ascii_uppercase()),
            _ => None,
        }
    };
    match v.split_once('-') {
        Some((a, b)) => {
            let (a, b) = (letter(a)?, letter(b)?);
            Some(if a <= b { (a, b) } else { (b, a) })
        }
        None => letter(v).map(|a| (a, a)),
    }
}

fn split_comparison(lower: &str) -> Option<(&'static str, Cmp, &str)> {
    for field in ["due", "created"] {
        if let Some(rest) = lower.strip_prefix(field) {
            for (op, cmp) in [("<=", Cmp::Le), (">=", Cmp::Ge), ("<", Cmp::Lt), (">", Cmp::Gt), ("=", Cmp::Eq)] {
                if let Some(value) = rest.strip_prefix(op) {
                    return Some((field, cmp, value));
                }
            }
        }
    }
    None
}

/// Relative Angaben zählen bei `due` in die Zukunft, bei `created` in die Vergangenheit.
fn resolve_date(field: &str, value: &str, today: NaiveDate) -> Option<NaiveDate> {
    if value == "today" {
        return Some(today);
    }
    if let Some(d) = parse_date(value) {
        return Some(d);
    }
    let unit = value.chars().last()?;
    let n: u32 = value[..value.len() - unit.len_utf8()].parse().ok()?;
    let future = field == "due";
    match unit {
        'd' | 'w' => {
            let days = Days::new(u64::from(n) * if unit == 'w' { 7 } else { 1 });
            if future { today.checked_add_days(days) } else { today.checked_sub_days(days) }
        }
        'm' => {
            let months = Months::new(n);
            if future { today.checked_add_months(months) } else { today.checked_sub_months(months) }
        }
        _ => None,
    }
}

fn has_token(tokens: &[String], wanted: &str) -> bool {
    tokens.iter().any(|t| t.to_lowercase() == wanted)
}

impl Filter {
    /// Prüft die Filterbegriffe (ohne Status).
    pub fn matches_terms(&self, t: &Task) -> bool {
        let body = t.body.to_lowercase();
        self.terms.iter().all(|term| match term {
            Term::Word(w) => body.contains(w.as_str()),
            Term::NotWord(w) => !body.contains(w.as_str()),
            Term::Project(p) => has_token(&t.projects, p),
            Term::NotProject(p) => !has_token(&t.projects, p),
            Term::Context(c) => has_token(&t.contexts, c),
            Term::NotContext(c) => !has_token(&t.contexts, c),
            Term::Pri(a, b) => t.effective_priority().is_some_and(|p| *a <= p && p <= *b),
            Term::PriNone => t.effective_priority().is_none(),
            Term::DueNone => t.due.is_none(),
            Term::Due(cmp, d) => t.due.is_some_and(|x| cmp.test(x, *d)),
            Term::Created(cmp, d) => t.created.is_some_and(|x| cmp.test(x, *d)),
        })
    }

    pub fn matches(&self, t: &Task) -> bool {
        self.status.accepts(t) && self.matches_terms(t)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::parse as task;

    fn today() -> NaiveDate {
        parse_date("2026-11-21").unwrap()
    }

    fn hits(filter: &str, line: &str) -> bool {
        parse(filter, today()).unwrap().matches(&task(line))
    }

    #[test]
    fn table() {
        let cases: &[(&str, &str, bool)] = &[
            // Suchwörter
            ("login", "Login-Bug fixen", true),
            ("LOGIN", "login-bug", true),
            ("digra", "Bug +digra", true),
            ("2026", "2026-11-20 Bug", false),
            ("-test", "Bug testen", false),
            ("-test", "Bug", true),
            ("done", "Bug done", true),
            // Projekte und Kontexte
            ("+digra", "Bug +Digra", true),
            ("+dig", "Bug +digra", false),
            ("-+digra", "Bug +digra", false),
            ("-+digra", "Bug +web", true),
            ("@rechner", "Bug @rechner", true),
            ("-@rechner", "Bug @rechner", false),
            // Priorität
            ("pri:A", "(A) Bug", true),
            ("pri:a", "(A) Bug", true),
            ("pri:A", "(B) Bug", false),
            ("pri:A-C", "(B) Bug", true),
            ("pri:A-C", "(D) Bug", false),
            ("pri:none", "Bug", true),
            ("pri:none", "(A) Bug", false),
            ("pri:B status:all", "x 2026-11-21 Bug pri:B", true),
            // Fälligkeit
            ("due<7d", "Bug due:2026-11-27", true),
            ("due<7d", "Bug due:2026-11-28", false),
            ("due<=7d", "Bug due:2026-11-28", true),
            ("due<7d", "Bug due:2026-01-01", true),
            ("due<7d", "Bug", false),
            ("due<2026-12-01", "Bug due:2026-11-30", true),
            ("due<2026-12-01", "Bug due:2026-12-01", false),
            ("due=today", "Bug due:2026-11-21", true),
            ("due>1w", "Bug due:2026-11-29", true),
            ("due:none", "Bug", true),
            ("due:none", "Bug due:2026-11-21", false),
            // Erstellung
            ("created>3d", "2026-11-19 Bug", true),
            ("created>3d", "2026-11-18 Bug", false),
            ("created>=3d", "2026-11-18 Bug", true),
            ("created>3d", "Bug", false),
            // Status
            ("", "x 2026-11-21 Bug", false),
            ("status:all", "x 2026-11-21 Bug", true),
            ("status:done", "Bug", false),
            ("status:done", "x 2026-11-21 Bug", true),
        ];
        for (filter, line, expected) in cases {
            assert_eq!(hits(filter, line), *expected, "Filter {filter:?} auf {line:?}");
        }
    }

    #[test]
    fn months_clamp_to_month_end() {
        let jan31 = parse_date("2026-01-31").unwrap();
        assert_eq!(resolve_date("due", "1m", jan31), parse_date("2026-02-28"));
        let leap = parse_date("2028-01-31").unwrap();
        assert_eq!(resolve_date("due", "1m", leap), parse_date("2028-02-29"));
        assert_eq!(resolve_date("created", "1m", parse_date("2026-03-31").unwrap()), parse_date("2026-02-28"));
    }

    #[test]
    fn invalid_date_is_error() {
        assert!(parse("due<morgen", today()).is_err());
        assert!(parse("created>3x", today()).is_err());
    }

    #[test]
    fn star_and_status() {
        let f = parse("* status:all", today()).unwrap();
        assert!(f.ignore_active);
        assert_eq!(f.status, Status::All);
        assert!(f.terms.is_empty());
    }
}
