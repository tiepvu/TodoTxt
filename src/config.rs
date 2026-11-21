//! Konfiguration und Pfade (SPEC 9).

use std::env;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SortKey {
    Priority,
    Due,
    Created,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Config {
    pub file: Option<PathBuf>,
    pub date_on_add: bool,
    pub color: bool,
    pub default_sort: Vec<SortKey>,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            file: None,
            date_on_add: true,
            color: true,
            default_sort: vec![SortKey::Priority, SortKey::Due, SortKey::Created],
        }
    }
}

/// `~` bzw. `~/…` zum Home-Verzeichnis erweitern.
pub fn expand_tilde(s: &str) -> PathBuf {
    let home = dirs::home_dir();
    match (s, home) {
        ("~", Some(h)) => h,
        (s, Some(h)) if s.starts_with("~/") || s.starts_with("~\\") => h.join(&s[2..]),
        (s, _) => PathBuf::from(s),
    }
}

#[cfg(not(windows))]
fn xdg(var: &str, fallback: &str) -> Option<PathBuf> {
    match env::var_os(var) {
        Some(v) if Path::new(&v).is_absolute() => Some(PathBuf::from(v)),
        _ => dirs::home_dir().map(|h| h.join(fallback)),
    }
}

/// Linux/macOS: `$XDG_CONFIG_HOME/todo/config.toml` bzw. `~/.config/…`;
/// Windows: `%APPDATA%\todo\config.toml`.
pub fn config_path() -> Option<PathBuf> {
    #[cfg(windows)]
    {
        env::var_os("APPDATA").map(|d| PathBuf::from(d).join("todo").join("config.toml"))
    }
    #[cfg(not(windows))]
    {
        xdg("XDG_CONFIG_HOME", ".config").map(|d| d.join("todo").join("config.toml"))
    }
}

/// Linux/macOS: `$XDG_STATE_HOME/todo/history` bzw. `~/.local/state/…`;
/// Windows: `%LOCALAPPDATA%\todo\history`.
pub fn history_path() -> Option<PathBuf> {
    #[cfg(windows)]
    {
        env::var_os("LOCALAPPDATA").map(|d| PathBuf::from(d).join("todo").join("history"))
    }
    #[cfg(not(windows))]
    {
        xdg("XDG_STATE_HOME", ".local/state").map(|d| d.join("todo").join("history"))
    }
}

/// Liest die Konfiguration. `Err` nur bei Fehlern, die den Start verhindern
/// (relativer Pfad in `file`); alles andere sind Warnungen mit Standardwert.
pub fn parse(text: &str) -> Result<(Config, Vec<String>), String> {
    let mut cfg = Config::default();
    let mut warnings = Vec::new();
    let table: toml::Table = match text.parse() {
        Ok(t) => t,
        Err(e) => {
            warnings.push(format!("ungültiges TOML, es gelten die Standardwerte: {}", e.to_string().trim()));
            return Ok((cfg, warnings));
        }
    };
    for (key, value) in &table {
        match key.as_str() {
            "file" => match value.as_str() {
                Some(s) => {
                    let p = expand_tilde(s);
                    if !p.is_absolute() {
                        return Err(format!(
                            "`file = \"{s}\"`: Pfad muss absolut sein oder mit ~ beginnen"
                        ));
                    }
                    cfg.file = Some(p);
                }
                None => warnings.push("`file` muss ein Text sein – wird ignoriert".into()),
            },
            "date_on_add" => match value.as_bool() {
                Some(b) => cfg.date_on_add = b,
                None => warnings.push("`date_on_add` muss true oder false sein – Standard true".into()),
            },
            "color" => match value.as_bool() {
                Some(b) => cfg.color = b,
                None => warnings.push("`color` muss true oder false sein – Standard true".into()),
            },
            "default_sort" => match parse_sort(value) {
                Some(keys) => cfg.default_sort = keys,
                None => warnings.push(
                    "`default_sort` erlaubt nur \"priority\", \"due\", \"created\" – Standard wird verwendet".into(),
                ),
            },
            other => warnings.push(format!("unbekannter Eintrag `{other}` wird ignoriert")),
        }
    }
    Ok((cfg, warnings))
}

fn parse_sort(value: &toml::Value) -> Option<Vec<SortKey>> {
    value
        .as_array()?
        .iter()
        .map(|v| match v.as_str()? {
            "priority" => Some(SortKey::Priority),
            "due" => Some(SortKey::Due),
            "created" => Some(SortKey::Created),
            _ => None,
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_and_values() {
        let (c, w) = parse("").unwrap();
        assert_eq!(c, Config::default());
        assert!(w.is_empty());

        let (c, w) = parse("date_on_add = false\ncolor = false\ndefault_sort = [\"due\"]\n").unwrap();
        assert!(!c.date_on_add && !c.color);
        assert_eq!(c.default_sort, vec![SortKey::Due]);
        assert!(w.is_empty());
    }

    #[test]
    fn warnings_keep_defaults() {
        let (c, w) = parse("color = 1\nfoo = 2\ndefault_sort = [\"name\"]\n").unwrap();
        assert_eq!(c, Config::default());
        assert_eq!(w.len(), 3);
        let (_, w) = parse("das ist kein toml").unwrap();
        assert_eq!(w.len(), 1);
    }

    #[test]
    fn relative_file_is_fatal() {
        assert!(parse("file = \"todo.txt\"").is_err());
        let (c, _) = parse("file = \"~/x/todo.txt\"").unwrap();
        assert!(c.file.unwrap().is_absolute());
    }
}
