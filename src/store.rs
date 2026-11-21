//! Datei lesen/schreiben: verlustfrei, atomar, mit Änderungserkennung (SPEC 10).

use std::collections::hash_map::DefaultHasher;
use std::fs::{self, File, OpenOptions};
use std::hash::{Hash, Hasher};
use std::io::{ErrorKind, Write};
use std::path::{Path, PathBuf};
use std::thread::sleep;
use std::time::Duration;

use anyhow::{Context as _, Result, anyhow};

const BOM: &[u8] = b"\xEF\xBB\xBF";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Eol {
    Lf,
    CrLf,
    /// letzte Zeile ohne Zeilenumbruch
    None,
}

impl Eol {
    pub fn as_str(self) -> &'static str {
        match self {
            Eol::Lf => "\n",
            Eol::CrLf => "\r\n",
            Eol::None => "",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Line {
    pub text: String,
    pub eol: Eol,
}

impl Line {
    pub fn is_blank(&self) -> bool {
        self.text.trim().is_empty()
    }
}

/// Inhalt einer Datei: jede Zeile mit ihrem eigenen Zeilenende, dazu ein evtl. BOM.
/// `to_bytes(parse(x)) == x` für jede UTF-8-Datei.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Document {
    pub bom: bool,
    pub lines: Vec<Line>,
}

impl Document {
    pub fn parse(bytes: &[u8]) -> Result<Document> {
        let (bom, body) = match bytes.strip_prefix(BOM) {
            Some(rest) => (true, rest),
            None => (false, bytes),
        };
        let text = std::str::from_utf8(body).map_err(|e| {
            anyhow!("kein gültiges UTF-8 (ab Byte {})", e.valid_up_to() + if bom { 3 } else { 0 })
        })?;
        let lines = text
            .split_inclusive('\n')
            .map(|part| {
                let (text, eol) = if let Some(t) = part.strip_suffix("\r\n") {
                    (t, Eol::CrLf)
                } else if let Some(t) = part.strip_suffix('\n') {
                    (t, Eol::Lf)
                } else {
                    (part, Eol::None)
                };
                Line { text: text.to_string(), eol }
            })
            .collect();
        Ok(Document { bom, lines })
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        let mut out = Vec::new();
        if self.bom {
            out.extend_from_slice(BOM);
        }
        for l in &self.lines {
            out.extend_from_slice(l.text.as_bytes());
            out.extend_from_slice(l.eol.as_str().as_bytes());
        }
        out
    }

    /// Zeilenende der Datei; neue Dateien bekommen LF.
    pub fn default_eol(&self) -> Eol {
        self.lines.iter().map(|l| l.eol).find(|e| *e != Eol::None).unwrap_or(Eol::Lf)
    }

    /// Hängt eine Zeile an und liefert ihre Nummer (1-basiert).
    pub fn push_line(&mut self, text: String) -> usize {
        let eol = self.default_eol();
        if let Some(last) = self.lines.last_mut()
            && last.eol == Eol::None {
                last.eol = eol;
            }
        self.lines.push(Line { text, eol });
        self.lines.len()
    }

    /// Zeile per Nummer (1-basiert).
    pub fn line(&self, n: usize) -> Option<&Line> {
        n.checked_sub(1).and_then(|i| self.lines.get(i))
    }

    pub fn set_text(&mut self, n: usize, text: String) {
        self.lines[n - 1].text = text;
    }
}

/// Fingerabdruck des Dateiinhalts; `None` = Datei existiert nicht.
pub fn fingerprint(bytes: Option<&[u8]>) -> Option<u64> {
    bytes.map(|b| {
        let mut h = DefaultHasher::new();
        b.hash(&mut h);
        h.finish()
    })
}

#[derive(Clone, Debug)]
pub struct Store {
    /// Pfad wie angegeben (absolut); kann ein Symlink sein.
    pub path: PathBuf,
}

impl Store {
    pub fn new(path: PathBuf) -> Store {
        Store { path }
    }

    /// Dateiname für Meldungen, z. B. `todo.txt`.
    pub fn name(&self) -> String {
        self.path
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_else(|| self.path.display().to_string())
    }

    /// Zieldatei nach Auflösen von Symlinks (auch wenn das Ziel noch nicht existiert).
    pub fn target(&self) -> PathBuf {
        resolve_symlinks(&self.path)
    }

    /// `done.txt` im Verzeichnis der Zieldatei.
    pub fn done_path(&self) -> PathBuf {
        self.target().with_file_name("done.txt")
    }

    /// Inhalt von `todo.txt`; `None`, wenn die Datei nicht existiert.
    pub fn read_raw(&self) -> Result<Option<Vec<u8>>> {
        read_optional(&self.target())
    }

    pub fn write(&self, bytes: &[u8]) -> Result<()> {
        write_atomic(&self.target(), bytes)
    }

    pub fn remove(&self) -> Result<()> {
        match fs::remove_file(self.target()) {
            Ok(()) => Ok(()),
            Err(e) if e.kind() == ErrorKind::NotFound => Ok(()),
            Err(e) => Err(e).with_context(|| format!("{} kann nicht gelöscht werden", self.name())),
        }
    }

    /// Nicht-leere Zeilen aus `done.txt` (fehlt die Datei: keine).
    pub fn read_done(&self) -> Result<Vec<String>> {
        let path = self.done_path();
        let Some(bytes) = read_optional(&path)? else {
            return Ok(Vec::new());
        };
        let doc = Document::parse(&bytes).with_context(|| format!("{} kann nicht gelesen werden", path.display()))?;
        Ok(doc.lines.into_iter().filter(|l| !l.is_blank()).map(|l| l.text).collect())
    }

    /// Hängt Zeilen an `done.txt` an (wird nie neu geschrieben).
    pub fn append_done(&self, lines: &[String]) -> Result<()> {
        let path = self.done_path();
        let existing = read_optional(&path)?.unwrap_or_default();
        let eol = if existing.windows(2).any(|w| w == b"\r\n") { "\r\n" } else { "\n" };
        let mut out = String::new();
        if !existing.is_empty() && !existing.ends_with(b"\n") {
            out.push_str(eol);
        }
        for l in lines {
            out.push_str(l);
            out.push_str(eol);
        }
        let mut f = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&path)
            .with_context(|| format!("{} kann nicht geöffnet werden", path.display()))?;
        f.write_all(out.as_bytes())?;
        f.sync_all()?;
        Ok(())
    }
}

fn read_optional(path: &Path) -> Result<Option<Vec<u8>>> {
    match fs::read(path) {
        Ok(b) => Ok(Some(b)),
        Err(e) if e.kind() == ErrorKind::NotFound => Ok(None),
        Err(e) => Err(e).with_context(|| format!("{} kann nicht gelesen werden", path.display())),
    }
}

fn resolve_symlinks(path: &Path) -> PathBuf {
    let mut p = path.to_path_buf();
    for _ in 0..40 {
        match fs::symlink_metadata(&p) {
            Ok(m) if m.file_type().is_symlink() => match fs::read_link(&p) {
                Ok(t) if t.is_absolute() => p = t,
                Ok(t) => p = p.parent().unwrap_or_else(|| Path::new("")).join(t),
                Err(_) => break,
            },
            _ => break,
        }
    }
    p
}

/// Temporäre Datei im selben Verzeichnis schreiben, Rechte übernehmen, umbenennen.
pub fn write_atomic(target: &Path, bytes: &[u8]) -> Result<()> {
    let dir = target.parent().filter(|d| !d.as_os_str().is_empty()).unwrap_or_else(|| Path::new("."));
    let name = target.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
    let tmp = dir.join(format!(".{name}.tmp-{}", std::process::id()));

    let result = (|| -> Result<()> {
        let mut f = File::create(&tmp).with_context(|| format!("{} kann nicht angelegt werden", tmp.display()))?;
        f.write_all(bytes)?;
        f.sync_all()?;
        drop(f);
        if let Ok(meta) = fs::metadata(target) {
            let _ = fs::set_permissions(&tmp, meta.permissions());
        }
        let mut attempt = 0;
        loop {
            match fs::rename(&tmp, target) {
                Ok(()) => return Ok(()),
                Err(_) if attempt < 3 => {
                    attempt += 1;
                    sleep(Duration::from_millis(100));
                }
                Err(e) => {
                    return Err(e).with_context(|| {
                        format!("{} kann nicht gespeichert werden (gesperrt?) – Datei unverändert", target.display())
                    });
                }
            }
        }
    })();
    if result.is_err() {
        let _ = fs::remove_file(&tmp);
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip_is_byte_identical() {
        let corpus: &[&[u8]] = &[
            b"",
            b"a",
            b"a\n",
            b"a\nb",
            b"a\r\nb\r\n",
            b"a\n\r\nb\n",
            b"\n\n\n",
            b"  (A) 2026-01-01   viel   Leerraum  +p \t@k key:value  \n",
            b"\xEF\xBB\xBF(A) mit BOM\r\n",
            "Umlaute äöü ß – ✓\n".as_bytes(),
        ];
        for &c in corpus {
            let doc = Document::parse(c).unwrap();
            assert_eq!(doc.to_bytes(), c, "{:?}", String::from_utf8_lossy(c));
        }
    }

    #[test]
    fn push_line_fixes_missing_newline_and_keeps_crlf() {
        let mut doc = Document::parse(b"a\r\nb").unwrap();
        assert_eq!(doc.push_line("c".into()), 3);
        assert_eq!(doc.to_bytes(), b"a\r\nb\r\nc\r\n");
        let mut empty = Document::default();
        empty.push_line("x".into());
        assert_eq!(empty.to_bytes(), b"x\n");
    }

    #[test]
    fn invalid_utf8_is_error() {
        assert!(Document::parse(b"a\xFFb").is_err());
    }
}
