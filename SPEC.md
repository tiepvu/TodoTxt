# todo – Spezifikation

Name: `todo`
Stand: 21.11.2026 · Version 0.2 (Entwurf)

Änderungen gegenüber 0.1: Aufgabennummer = Zeilennummer (5.3), `/archive` in Version 1 als einziger Neunummerierungs-Schritt, `/del` hinterlässt Leerzeile, `done.txt` nur lesend bzw. anhängend, keine einmaligen Shell-Befehle mehr (nur REPL), keine wiederkehrenden Aufgaben, Filter `done` → `status:all`/`status:done` mit präzisen Datumsvergleichen, Erkennung externer Änderungen per Hash, `/edit` bearbeitet die ganze Zeile, `Prompter` für Rückfragen, Pfade und Vorrang in der Konfiguration.

---

## 1. Zweck

Ein Konsolen-Tool zum schnellen Erfassen und Verwalten von Aufgaben über mehrere Projekte hinweg, gespeichert in **einer** zentralen Textdatei im todo.txt-Format.

Die Bedienung orientiert sich an Claude Code: Man startet das Tool einmal, landet in einer interaktiven Eingabe (REPL) und arbeitet dort mit Freitext und `/befehlen`. Einmalige Aufrufe aus der Shell (`todo ls …`) sind bewusst nicht vorgesehen; dadurch gibt es keine Konflikte zwischen der Befehlssyntax und der Shell – weder mit Sonderzeichen (`<`, `>`, `*`, in PowerShell `@`) noch mit Argumenten, die mit `-` beginnen und als Optionen gelesen würden.

## 2. Ziele und Nicht-Ziele

### Ziele

- Erfassen einer Aufgabe in wenigen Sekunden, ohne Nachdenken über Ablage
- Überblick über alle Projekte in einer Datei, gefiltert per `+projekt`
- Volle Kompatibilität mit dem todo.txt-Format; die Datei bleibt mit jedem Texteditor und anderen todo.txt-Tools nutzbar
- Eine einzelne ausführbare Datei ohne Laufzeitumgebung
- Lauffähig unter Linux, macOS und Windows

### Nicht-Ziele

- Keine Plugins, keine Skript-Schnittstelle nach außen
- Keine einmaligen Shell-Befehle (`todo <befehl> [args]`); Bedienung nur über die REPL
- Keine GUI, keine Vollbild-Oberfläche (TUI) in Version 1
- Keine Synchronisation, kein Server, keine Datenbank
- Keine Unteraufgaben, Abhängigkeiten oder Notizen an Aufgaben
- Keine wiederkehrenden Aufgaben vorerst; ein vorhandenes `rec:` bleibt als normales `key:value` unverändert erhalten
- Kein Wissensmanagement – das bleibt in Obsidian

> Leitlinie: Was nicht in dieser Spezifikation steht, gehört nicht in den Kern.

## 3. Plattformen

| Plattform | Terminal | Status |
|---|---|---|
| Linux | beliebig | Ziel |
| macOS | Terminal, iTerm2 u. a. | Ziel |
| Windows | Windows Terminal / VS Code (PowerShell, cmd, Git Bash) | Ziel |
| Windows | mintty, das Standardfenster von „Git Bash“ | Ziel, braucht ConPTY-Unterstützung. Früh testen |

Da es keine Shell-Befehle als Ausweichlösung gibt, muss die REPL in allen Ziel-Terminals funktionieren. Erkennt `todo` beim Start kein interaktives Terminal (z. B. mintty ohne ConPTY, Pipe, Umleitung), bricht es mit einer verständlichen Meldung ab – in mintty mit dem Hinweis auf `winpty todo` bzw. die ConPTY-Option von Git for Windows.

Erkennung: Eingabe und Ausgabe müssen Terminals sein **und** die Zeilenbearbeitung muss sich tatsächlich in den Eingabemodus (Raw Mode) schalten lassen. Eine reine Prüfung per `IsTerminal` reicht nicht, da sie unter Windows MSYS-Pseudoterminals (mintty) ebenfalls als Terminal melden kann.

## 4. Dateiformat

Grundlage ist die offizielle todo.txt-Spezifikation: https://github.com/todotxt/todo.txt

### 4.1 Aufbau einer Zeile

```
[x ] [(A) ] [Erledigt-Datum ] [Erstellt-Datum ] Beschreibung [+projekt] [@kontext] [key:value]
```

Beispiele:

```
(A) 2026-11-20 Login-Bug fixen +digra @rechner
2026-11-20 JSON-Schema-Export testen +batterypass due:2026-11-27
x 2026-11-21 2026-11-19 Keycloak-Version prüfen +keycloak
```

### 4.2 Unterstützte Elemente

| Element | Bedeutung |
|---|---|
| `x ` | erledigt (Zeilenanfang) |
| `(A)`–`(Z)` | Priorität (nur am Zeilenanfang offener Aufgaben) |
| `JJJJ-MM-TT` | Erstellungs- bzw. Erledigungsdatum |
| `+projekt` | Projekt, beliebig oft, an beliebiger Stelle |
| `@kontext` | Kontext, beliebig oft, an beliebiger Stelle |
| `due:JJJJ-MM-TT` | Fälligkeitsdatum |
| `pri:A` | Priorität erledigter Aufgaben (wird beim Abhaken gesetzt) |
| `key:value` | beliebige weitere Paare, werden unverändert erhalten |

### 4.3 Dateien

| Datei | Inhalt |
|---|---|
| `todo.txt` | offene und noch nicht archivierte erledigte Aufgaben, ggf. Leerzeilen gelöschter Aufgaben |
| `done.txt` | archivierte erledigte Aufgaben (gleiches Verzeichnis) |

`done.txt` wird vom Tool nur auf zwei Arten berührt: `/archive` hängt Zeilen ans Ende an, `/ls status:all` bzw. `/ls status:done` liest sie. Alle anderen Befehle wirken ausschließlich auf `todo.txt`. Fehlt `done.txt`, legt `/archive` sie an.

## 5. Bedienkonzept

### 5.1 Start

```
todo                    # REPL mit Standarddatei
todo -f pfad/todo.txt   # REPL mit bestimmter Datei
todo --plain            # REPL ohne Farben
todo --help | --version
```

Weitere Argumente werden nicht akzeptiert; `todo ls` o. Ä. führt zu einer Fehlermeldung mit Hinweis auf die REPL.

Exit-Codes des Programms: `0` normales Beenden, `1` Datei-/Systemfehler oder kein interaktives Terminal, `2` ungültige Startoptionen (Standard von `clap`). Benutzerfehler innerhalb der REPL (z. B. ungültige Nummer) werden nur gemeldet und beenden das Programm nicht.

Existiert die Datei beim Start nicht, startet die REPL mit leerer Liste und dem Hinweis `<Pfad> existiert noch nicht – wird beim ersten Speichern angelegt`. Angelegt wird sie beim ersten ändernden Befehl (UTF-8 ohne BOM, LF). Existiert das Verzeichnis nicht, bricht `todo` beim Start mit Exit-Code `1` ab.

Reihenfolge der Dateisuche:

1. Option `-f`
2. Umgebungsvariable `TODO_FILE`
3. Eintrag `file` in der Konfiguration
4. `~/todo.txt`

### 5.2 REPL

- Prompt: `todo>`; ab Version 2 mit aktivem Projekt `todo +digra>` (siehe `/use`)
- **Freitext ohne `/`** wird als neue Aufgabe hinzugefügt
- Befehle beginnen mit `/`; ein unbekannter Befehl ergibt `Unbekannter Befehl /foo – /help zeigt alle Befehle` und wird **nicht** als Aufgabe gespeichert. Eine Aufgabe, die mit `/` beginnen soll, wird mit `/add` erfasst (`/add /etc/hosts prüfen`)
- History über Sitzungen hinweg (Pfeiltasten, Strg+R)
- Tab-Vervollständigung für Befehle, `+projekte` und `@kontexte` (aus der aktuellen Datei)
- Strg+C bricht die aktuelle Eingabe ab, Strg+D oder `/quit` beendet

### 5.3 Referenzen auf Aufgaben

Wie bei todo.sh ist die **Nummer einer Aufgabe ihre Zeilennummer in `todo.txt`** (1-basiert, Leerzeilen mitgezählt).

- Die Nummer hängt nicht von Filter, Sortierung oder vorheriger Anzeige ab; ein vorheriges `/ls` ist nicht nötig, und die Nummern stimmen mit todo.sh überein
- Nach Sortierung oder Filterung sind die angezeigten Nummern daher nicht fortlaufend (z. B. `7, 2, 12`)
- Nummern bleiben stabil, weil keine Zeile verschoben wird:
  - `/add` hängt am Ende an
  - `/done`, `/edit` und ab Version 2 `/pri` ändern die Zeile an ihrer Stelle
  - `/del` ersetzt die Zeile durch eine Leerzeile
- Nummern ändern sich nur durch `/archive` (Abschnitt 6.2) oder durch externe Bearbeitung der Datei
- Mehrere Nummern: `3 5 7` oder Bereich `3-5`; ein Bereich umfasst nur vorhandene Aufgaben, Leerzeilen darin werden übersprungen
- Eine Nummer, die auf eine Leerzeile oder hinter das Dateiende zeigt, ergibt eine Fehlermeldung; der Befehl wird nicht ausgeführt
- **Externe Änderungen:** Die REPL merkt sich nach jedem Befehl den Stand der Datei (Hash des Inhalts). Das ist der Stand, zu dem die Nummern auf dem Bildschirm passen. Vor jedem Befehl wird verglichen:
  - unverändert: Befehl wird ausgeführt
  - geändert und der Befehl verwendet Nummern (`/done`, `/edit`, `/del`, `/pri`, `/undo`): Datei wird neu eingelesen, der Befehl abgelehnt mit `todo.txt wurde extern geändert – Nummern bitte mit /ls prüfen`
  - geändert und der Befehl verwendet keine Nummern (`/ls`, `/add`, `/archive` usw.): Datei wird neu eingelesen, der Befehl ausgeführt
  - Zusätzlich wird unmittelbar vor dem Schreiben erneut verglichen (Abschnitt 10)
- Aufgaben aus `done.txt` haben keine Nummer und können nicht per Befehl verändert werden

## 6. Befehle

### 6.1 Übersicht

#### Kern (Version 1)

| Befehl | Kurz | Argumente | Funktion |
|---|---|---|---|
| *Freitext* | – | Text | Aufgabe hinzufügen |
| `/add` | `/a` | Text | Aufgabe hinzufügen |
| `/ls` | `/l` | [Filter] | Aufgaben anzeigen und durchsuchen |
| `/done` | `/d` | Nr… | als erledigt markieren |
| `/edit` | `/e` | Nr | Aufgabe mit vorausgefülltem Text bearbeiten |
| `/del` | – | Nr… | Aufgabe löschen (mit Rückfrage) |
| `/undo` | `/u` | – | letzte Änderung rückgängig |
| `/archive` | – | – | erledigte Aufgaben nach `done.txt` verschieben, Leerzeilen entfernen, neu nummerieren |
| `/help` | `/h` | [Befehl] | Hilfe |
| `/quit` | `/q` | – | beenden |

#### Komfort (Version 2)

| Befehl | Kurz | Argumente | Funktion |
|---|---|---|---|
| `/use` | – | [+projekt] | aktives Projekt setzen bzw. aufheben |
| `/pri` | `/p` | Nr Prio | Priorität setzen; `-` entfernt sie |
| `/projects` | `/pj` | – | alle Projekte mit Anzahl offener Aufgaben |
| `/contexts` | `/cx` | – | alle Kontexte mit Anzahl offener Aufgaben |

### 6.2 Details je Befehl

#### Freitext / `/add`

- Hängt die Aufgabe als neue Zeile ans Ende von `todo.txt`
- Setzt automatisch das Erstellungsdatum (abschaltbar per Konfiguration)
- Steht am Anfang `(A)` o. Ä., wird die Priorität übernommen und vor das Datum gesetzt
- Steht am Anfang (ggf. nach der Priorität) bereits ein Datum `JJJJ-MM-TT`, gilt es als Erstellungsdatum; es wird kein weiteres ergänzt
- Beginnt der Text mit `x ` (kleines x und Leerzeichen), würde er als erledigte Aufgabe gespeichert. Dann Fehlermeldung `Text beginnt mit „x “ und würde als erledigt gelten – bitte umformulieren`, keine Änderung
- Ab Version 2: Ist ein Projekt per `/use` aktiv und enthält der Text **gar kein** `+projekt`, wird das aktive Projekt angehängt. Enthält er ein anderes Projekt, bleibt er unverändert
- Leere Eingabe wird ignoriert
- Ausgabe: Nummer und gespeicherte Zeile; die Nummer ist sofort in Befehlen verwendbar

```
todo> Login-Bug fixen +digra
+ 17 2026-11-21 Login-Bug fixen +digra
```

#### `/ls [Filter]`

- Ohne Filter: alle offenen Aufgaben (ab Version 2 bei aktivem Projekt nur dessen Aufgaben)
- Sortierung: Priorität (A zuerst, ohne Priorität zuletzt), dann Fälligkeit, dann Erstellungsdatum
- Filtersyntax siehe Abschnitt 7
- Jede Zeile beginnt mit ihrer Nummer (Zeilennummer, Abschnitt 5.3)
- Am Ende eine Zeile `n von m Aufgaben`:
  - `n` = Anzahl angezeigter Aufgaben
  - `m` = Anzahl der Grundmenge ohne Filter: alle offenen Aufgaben in `todo.txt`; mit `status:all` alle Aufgaben aus `todo.txt` und `done.txt`, mit `status:done` alle erledigten
  - gibt es erledigte Aufgaben in `todo.txt`, folgt ein Hinweis: `· 14 erledigt – /archive räumt auf`

#### `/done Nr…`

- Setzt `x ` und das heutige Erledigungsdatum an den Zeilenanfang; ein vorhandenes Erstellungsdatum bleibt dahinter erhalten
- Vorhandene Priorität wird entfernt und als `pri:X` ans Zeilenende angehängt
- Der Rest der Zeile bleibt byte-genau erhalten (keine Neuformatierung)
- Ohne Erstellungsdatum entsteht `x 2026-11-21 Text` (wie bei todo.sh); das erste Datum nach `x` gilt beim Lesen immer als Erledigungsdatum
- Ist eine Aufgabe bereits erledigt: Hinweis `7 ist bereits erledigt`, keine Änderung; die übrigen Nummern werden trotzdem verarbeitet
- Ausgabe: die erledigten Zeilen mit Nummer

```
todo> /done 7
✓ 7 x 2026-11-21 2026-11-20 Login-Bug fixen +digra @rechner pri:A
```

#### `/edit Nr`

- Zeigt die **vollständige Zeile** genau so, wie sie in der Datei steht, als vorausgefüllte Eingabe an – inkl. `x`, Priorität und Datumsangaben. Damit lassen sich auch Status, Priorität und Daten ändern (z. B. `x …` entfernen, um eine Erledigung zurückzunehmen)
- Enter speichert, Strg+C bricht ab
- Unveränderter Text: keine Änderung, kein Undo-Schritt
- Leerer Text: Fehlermeldung `Leerer Text – zum Löschen /del verwenden`, keine Änderung
- Der neue Text ersetzt die Zeile 1:1; es werden keine Daten automatisch ergänzt oder entfernt
- Ausgabe: die gespeicherte Zeile mit Nummer

#### `/del Nr…`

- Zeigt die betroffenen Zeilen und fragt `Löschen? [j/N]`; Enter, `n` oder Strg+C bedeuten Nein (gilt für alle Rückfragen)
- Die Zeile wird durch eine **Leerzeile** ersetzt, damit alle anderen Nummern gleich bleiben; entfernt wird sie erst durch `/archive`

#### `/undo`

- Macht die letzte ändernde Aktion der aktuellen Sitzung rückgängig
- Mehrfach aufrufbar (Stapel, mindestens 20 Schritte)
- Gilt nur innerhalb einer REPL-Sitzung
- Jeder Schritt speichert den vollständigen Inhalt von `todo.txt` vor der Änderung und den Hash danach. `/undo` schreibt den gespeicherten Inhalt nur zurück, wenn die Datei noch genau dem Stand nach der Änderung entspricht
- Wurde die Datei inzwischen extern geändert: `/undo` wird abgelehnt (`todo.txt wurde extern geändert – Undo nicht möglich`) und der Stapel geleert, damit keine fremden Änderungen überschrieben werden
- Ausgabe: was rückgängig gemacht wurde, z. B. `Rückgängig: /done 7`
- `/archive` ist ausgenommen und leert den Stapel (siehe dort)

#### `/use [+projekt]`

- Setzt das aktive Projekt; Prompt zeigt es an
- Wirkung: neue Aufgaben bekommen es automatisch, `/ls` filtert darauf
- `/use` ohne Argument hebt es auf
- `/ls *` zeigt trotz aktivem Projekt alle Aufgaben
- Das `+` darf fehlen (`/use digra` = `/use +digra`); erlaubt ist genau ein Projekt. Ein Projekt ohne Aufgaben ist erlaubt (Hinweis `noch keine Aufgaben`)
- Groß-/Kleinschreibung zählt beim Filtern wie in Abschnitt 7 nicht
- Das aktive Projekt wirkt in `/ls` wie ein zusätzlicher `+projekt`-Filter: `m` in `n von m Aufgaben` bleibt die Grundmenge ohne Filter, zeigt also auch die Aufgaben anderer Projekte an
- Es gilt nur für die laufende Sitzung und wird nicht gespeichert

#### `/pri Nr Prio`

- `Prio` ist ein Buchstabe A–Z oder `-` zum Entfernen; Kleinbuchstaben werden großgeschrieben
- Genau eine Nummer, kein Bereich
- Ändert nur `(X) ` am Zeilenanfang; der Rest der Zeile bleibt byte-genau erhalten, die Zeile an ihrer Stelle
- Nur für offene Aufgaben; bei erledigten Fehlermeldung mit Hinweis auf `/edit` (deren `pri:X` bleibt unverändert)
- Gleiche Priorität bzw. Entfernen ohne Priorität: Hinweis, keine Änderung, kein Undo-Schritt
- Ausgabe: die gespeicherte Zeile mit Nummer, z. B. `~ 3 (A) 2026-11-20 Text`

#### `/projects`, `/contexts`

```
+digra        5
+keycloak     2
+batterypass  1
```

- Gezählt werden offene Aufgaben in `todo.txt`; eine Aufgabe zählt je Token einmal
- Groß-/Kleinschreibung zählt nicht; angezeigt wird die erste Schreibweise in der Datei
- Sortierung: Anzahl absteigend, dann Name
- Ohne Treffer: `Keine Projekte in offenen Aufgaben` bzw. `Keine Kontexte in offenen Aufgaben`

#### `/archive`

Einziger Befehl, der Aufgaben neu nummeriert. Er wird nie automatisch ausgeführt, damit sich Nummern nur dann ändern, wenn man es selbst auslöst.

- Verschiebt alle erledigten Zeilen ans Ende von `done.txt` (Reihenfolge wie in `todo.txt`)
- Entfernt Leerzeilen aus `todo.txt`; die verbleibenden Aufgaben behalten ihre Reihenfolge und werden dadurch neu nummeriert
- Zeigt vorher, was passiert, und fragt `14 erledigte Aufgaben archivieren, 3 Leerzeilen entfernen – Nummern ändern sich. Fortfahren? [j/N]`
- Gibt es nichts zu archivieren, Ausgabe `Nichts zu archivieren` und keine Änderung
- Reihenfolge beim Schreiben: erst an `done.txt` anhängen, dann `todo.txt` atomar neu schreiben. Bricht der Vorgang dazwischen ab, entstehen schlimmstenfalls doppelte Einträge in `done.txt`, aber kein Verlust
- Ist **nicht** per `/undo` rückgängig zu machen (betrifft zwei Dateien); die Rückfrage ersetzt das. `/archive` leert den Undo-Stapel, da sich ältere Schritte auf alte Nummern beziehen; ein folgendes `/undo` meldet `Nichts rückgängig zu machen (seit /archive)`
- Danach zeigt die REPL die neue Liste wie bei `/ls`

#### `/help [Befehl]`

- Ohne Argument: Kurzübersicht aller Befehle
- Mit Befehl: Beschreibung und Beispiele

## 7. Filtersyntax

Filter sind durch Leerzeichen getrennt und werden UND-verknüpft. Groß-/Kleinschreibung wird ignoriert. Kein Regex, keine Maskierung nötig: Alles, was nicht die Form eines Filters aus der Tabelle hat, ist ein Suchwort.

| Filter | Bedeutung |
|---|---|
| `wort` | Zeile enthält „wort“ als Teilzeichenkette – gesucht wird im angezeigten Text inkl. `+projekt`, `@kontext` und `key:value`, nicht in Status, Priorität und Datumsangaben am Zeilenanfang |
| `-wort` | Zeile enthält „wort“ nicht |
| `+projekt` | hat genau dieses Projekt (ganzes Token, nicht Teilzeichenkette) |
| `-+projekt` | hat dieses Projekt nicht |
| `@kontext` | hat genau diesen Kontext |
| `-@kontext` | hat diesen Kontext nicht |
| `pri:A` | Priorität A; bei erledigten Aufgaben zählt der Wert von `pri:` |
| `pri:A-C` | Priorität A bis C |
| `pri:none` | ohne Priorität |
| `due<7d`, `due<=2026-12-01` usw. | Vergleich mit dem Fälligkeitsdatum, siehe unten |
| `due:none` | ohne Fälligkeit |
| `created>3d` usw. | Vergleich mit dem Erstellungsdatum, siehe unten; Aufgaben ohne Erstellungsdatum passen nie |
| `status:all` | auch erledigte Aufgaben, inkl. `done.txt` |
| `status:done` | nur erledigte Aufgaben, inkl. `done.txt` |
| `*` | aktives Projekt ignorieren (Version 2, siehe `/use`) |

Einträge aus `done.txt` erscheinen ohne Nummer und sind nur lesbar (Abschnitt 5.3).

**Datumsvergleiche**

- Operatoren: `<`, `<=`, `>`, `>=`, `=`; `<` und `>` schließen den Stichtag aus, `<=` und `>=` ein
- Stichtag ist entweder ein Datum `JJJJ-MM-TT`, `today` oder eine relative Angabe `N` + Einheit (`d` Tage, `w` Wochen, `m` Monate)
- Relative Angaben zählen in die natürliche Richtung des Feldes:
  - bei `due` **ab heute in die Zukunft**: `due<7d` = fällig vor heute + 7 Tage, also überfällig, heute und die nächsten 6 Tage
  - bei `created` **ab heute in die Vergangenheit**: `created>3d` = erstellt nach heute − 3 Tage, also in den letzten 2 Tagen und heute; `created>=3d` schließt den Tag vor 3 Tagen ein
- Monate werden kalendarisch gerechnet; gibt es den Tag im Zielmonat nicht, gilt der letzte Tag des Monats (31.01. + 1m = 28./29.02.)
- „Heute“ ist das lokale Datum des Rechners
- Aufgaben ohne Fälligkeit passen auf keinen `due`-Vergleich
- Ungültiger Stichtag (z. B. `due<morgen`) ergibt eine Fehlermeldung statt einer leeren Liste

Beispiele:

```
/ls +digra @rechner
/ls login -test
/ls pri:A due<7d
/ls created>=3d status:all
/ls due=today
```

## 8. Ausgabe

- Farben, wenn das Terminal sie unterstützt und `NO_COLOR` nicht gesetzt ist (ein leerer Wert gilt wie bei https://no-color.org als nicht gesetzt)
  - Priorität A rot, B gelb, C grün
  - `+projekt` blau, `@kontext` cyan
  - überfällige `due:` rot
  - erledigte Aufgaben grau
- Option `--plain` bzw. Konfiguration schaltet Farben ab

Beispiel `/ls`:

```
 7 (A) Login-Bug fixen +digra @rechner
 2     JSON-Schema-Export testen +batterypass due:2026-11-27
12     Theme-Update einspielen +website
3 von 3 Aufgaben · 4 erledigt – /archive räumt auf
```

Darstellung einer Zeile:

- Nummer, rechtsbündig auf die Breite der größten angezeigten Nummer
- Spalte für Status bzw. Priorität: `(A)`, `x` für erledigt oder leer
- Danach die Beschreibung mit allen Tokens (`+projekt`, `@kontext`, `due:`, sonstige `key:value`) in Originalreihenfolge
- Erstellungs- und Erledigungsdatum werden in Listen **nicht** angezeigt; sie sind in `/edit` sichtbar
- Bei gleicher Sortierposition gilt die Reihenfolge in der Datei; Einträge aus `done.txt` stehen nach denen aus `todo.txt`

Beispiel `/ls login status:all` (Eintrag aus `done.txt` ohne Nummer):

```
 7 (A) Login-Bug fixen +digra @rechner
 9   x Login-Timeout erhöhen +digra
     x Login-Seite stylen +website
3 von 61 Aufgaben · 4 erledigt – /archive räumt auf
```

## 9. Konfiguration

| Datei | Linux / macOS | Windows |
|---|---|---|
| Konfiguration | `$XDG_CONFIG_HOME/todo/config.toml`, sonst `~/.config/todo/config.toml` | `%APPDATA%\todo\config.toml` |
| History der REPL | `$XDG_STATE_HOME/todo/history`, sonst `~/.local/state/todo/history` | `%LOCALAPPDATA%\todo\history` |

Auch unter macOS gilt `~/.config` (nicht `~/Library/Application Support`, das `dirs::config_dir()` liefern würde). Fehlende Verzeichnisse für die History werden angelegt.

```toml
file = "~/todo.txt"
date_on_add = true
color = true
default_sort = ["priority", "due", "created"]
```

| Eintrag | Standard | Bedeutung |
|---|---|---|
| `file` | `~/todo.txt` | Pfad zu `todo.txt`; `~` am Anfang wird zum Home-Verzeichnis erweitert. Relative Pfade sind nicht erlaubt (Fehlermeldung beim Start) |
| `date_on_add` | `true` | Erstellungsdatum bei `/add` setzen |
| `color` | `true` | Farben verwenden |
| `default_sort` | `["priority", "due", "created"]` | Sortierschlüssel für `/ls` in dieser Reihenfolge; erlaubt sind `priority`, `due`, `created`. Als letzter Schlüssel gilt immer die Reihenfolge in der Datei |

Alle Einträge optional; ohne Datei gelten die Standardwerte. Unbekannte Einträge oder ungültige Werte führen zu einer Warnung beim Start; es gilt dann der Standardwert.

**Farben – Vorrang** (die erste zutreffende Regel gilt):

1. `--plain` → keine Farben
2. Umgebungsvariable `NO_COLOR` gesetzt und nicht leer → keine Farben
3. `color = false` → keine Farben
4. Terminal unterstützt keine Farben → keine Farben
5. sonst Farben

## 10. Datensicherheit

- **Verlustfreies Lesen/Schreiben:** Unbekannte Tokens, Reihenfolge und Schreibweise einer Zeile bleiben unverändert, solange die Zeile nicht bearbeitet wird
- **Atomares Schreiben:**
  1. Ist `todo.txt` ein Symlink, wird zuerst das Ziel aufgelöst; geschrieben wird immer die Zieldatei, der Link bleibt erhalten
  2. In eine temporäre Datei im **selben Verzeichnis wie die Zieldatei** schreiben und auf die Platte schreiben lassen (`sync`)
  3. Dateirechte der bisherigen Datei übernehmen (Unix)
  4. Temporäre Datei über die Zieldatei umbenennen
  - Schlägt das Umbenennen fehl (Windows: Datei von Editor, Sync-Client oder Virenscanner gesperrt), wird es bis zu dreimal im Abstand von 100 ms wiederholt. Danach Fehlermeldung, temporäre Datei löschen, `todo.txt` bleibt unverändert
- **Externe Änderungen:** Erkennung über einen Hash des Dateiinhalts (die Datei wird ohnehin vollständig gelesen; Änderungszeit und Größe allein sind zu ungenau). Ablauf in der REPL siehe Abschnitt 5.3. Unmittelbar vor dem Schreiben wird die Datei erneut gelesen und verglichen; weicht sie ab, wird nicht geschrieben
- **Zeilenenden:** vorhandene Zeilenenden (LF/CRLF) der Datei beibehalten; neue Dateien mit LF
- **Kodierung:** UTF-8; ein vorhandenes BOM bleibt erhalten
- Leerzeilen werden beim Anzeigen ignoriert, zählen aber für die Nummerierung mit und werden erst durch `/archive` entfernt
- **`done.txt`:** wird nur angehängt, nie neu geschrieben; daher keine Änderungserkennung nötig. Ein fehlender Zeilenumbruch am Dateiende wird vor dem Anhängen ergänzt

## 11. Architektur

Sprache: **Rust**, eine ausführbare Datei.

```
src/
├── main.rs          Einstieg: Startoptionen, Terminal-Prüfung, REPL
├── task.rs          Task-Modell
├── parser.rs        Zeile ↔ Task, verlustfrei
├── store.rs         Datei lesen/schreiben, atomar, Änderungserkennung
├── filter.rs        Filtersyntax parsen und anwenden
├── view.rs          Sortierung, Zuordnung Zeilennummer ↔ Aufgabe
├── render.rs        Ausgabe, Farben
├── repl.rs          Eingabe, History, Vervollständigung
├── config.rs        Konfiguration
├── undo.rs          Undo-Stapel
└── commands/
    ├── mod.rs       Command-Trait und Registry
    ├── add.rs
    ├── ls.rs
    ├── done.rs
    ├── edit.rs
    ├── del.rs
    ├── undo.rs
    ├── archive.rs
    ├── help.rs
    ├── use.rs       Version 2
    ├── pri.rs       Version 2
    └── tags.rs      Version 2: /projects und /contexts
```

### Command-Trait

```rust
pub trait Command {
    fn name(&self) -> &'static str;
    fn aliases(&self) -> &'static [&'static str] { &[] }
    /// Argumente für die Übersicht in /help, z. B. "Nr…"
    fn usage(&self) -> &'static str { "" }
    /// Einzeiler für /help
    fn summary(&self) -> &'static str;
    /// Ausführliche Hilfe für /help <befehl>
    fn help(&self) -> &'static str;
    /// Verwendet der Befehl Nummern? Dann wird er nach externer Änderung abgelehnt (5.3)
    fn uses_numbers(&self) -> bool { false }
    fn run(&self, args: &str, ctx: &mut Context) -> Result<Output>;
}
```

`/undo` meldet `uses_numbers` nicht, sondern prüft selbst gegen den Stand nach dem Undo-Schritt, damit es bei externer Änderung den Stapel leeren kann (6.2).

`Context` enthält Store, aktives Projekt (ab Version 2), Undo-Stapel, Konfiguration und einen `Prompter` für Interaktion. Eine „letzte Liste“ wird nicht benötigt, da Nummern Zeilennummern sind.

```rust
pub trait Prompter {
    /// Text vor einer Rückfrage anzeigen (z. B. die betroffenen Zeilen bei /del)
    fn show(&mut self, text: &str);
    /// Ja/Nein-Rückfrage; Enter, n und Strg+C ergeben false
    fn confirm(&mut self, question: &str) -> bool;
    /// Vorausgefüllte Eingabezeile; None bei Strg+C
    fn edit_line(&mut self, initial: &str) -> Option<String>;
}
```

In der REPL setzt `rustyline` den `Prompter` um, in Tests eine Attrappe mit vorgegebenen Antworten. Befehle greifen nie direkt auf das Terminal zu und bleiben dadurch ohne Terminal testbar. Ein neuer Befehl = eine neue Datei + ein Eintrag in der Registry.

### Bibliotheken (Crates)

| Zweck | Crate |
|---|---|
| REPL, History, Vervollständigung | `rustyline` (Alternative: `reedline`) |
| Startoptionen (`-f`, `--plain`, `--help`, `--version`) | `clap` |
| Datum | `chrono` |
| Konfiguration | `toml` (direkt als Tabelle gelesen, um unbekannte Einträge melden zu können) |
| Pfade (Home) | `dirs` |
| Farben | `owo-colors` |
| Fehler | `anyhow` |
| Windows-Konsole (Terminal-Prüfung, ANSI-Farben) | `windows-sys` |
| Tests | `tempfile` |

### Build

- `cargo build --release`
- Linux: zusätzlich `x86_64-unknown-linux-musl` für ein statisches Binary
- Releases für: Linux x86_64, macOS arm64/x86_64, Windows x86_64

## 12. Tests

- **Parser:** Roundtrip-Tests – jede Zeile aus einem Testkorpus muss nach Lesen und Schreiben byte-identisch sein
- **Befehle:** Unit-Tests gegen eine Datei im temporären Verzeichnis, Rückfragen über einen Test-`Prompter`
- **Store:** atomares Schreiben (kein Teilinhalt nach Abbruch), Symlink bleibt Symlink, Dateirechte bleiben erhalten, Erkennung externer Änderungen, `/undo` nach externer Änderung wird abgelehnt, CRLF- und LF-Dateien, BOM, fehlender Zeilenumbruch am Dateiende, nicht vorhandene Datei
- **Datum:** Tests mit fest vorgegebenem „heute“ (z. B. Monatsende, Schaltjahr für `m`)
- **Filter:** Tabellengetriebene Tests je Filterart
- **Nummern:** `/add`, `/done`, `/edit`, `/del` verändern keine fremde Zeilennummer; `/archive` erhält die Reihenfolge, und ein Abbruch zwischen beiden Schreibschritten verliert keine Aufgabe
- **Plattform:** manueller Test der REPL in Windows Terminal und Git Bash (mintty) vor Version 1.0

## 13. Offene Fragen

Derzeit keine.

### Entschieden in 0.2

- **Wiederkehrende Aufgaben (`rec:`):** vorerst weggelassen (siehe Nicht-Ziele); kann später ergänzt werden, da `rec:` schon jetzt verlustfrei erhalten bleibt
- **Keine Shell-Befehle:** Bedienung nur über die REPL; damit entfallen Konflikte der Filtersyntax mit Shell-Sonderzeichen
- **Name:** `todo`. Hinweis: Ein vorhandener Shell-Alias `todo` (z. B. für todo.sh) überdeckt das Binary und muss ggf. entfernt werden
- **Nummern:** Zeilennummer in `todo.txt` wie bei todo.sh, keine Kurz-Hashes (5.3)
- **Archivieren:** nicht automatisch bei `/done`, sondern nur per `/archive`, da sonst jede Erledigung die Nummern verschiebt
