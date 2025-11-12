# certgen

certgen ist ein kleines CLI-Tool zum Befüllen von ODF-Vorlagen (z. B. .odt) mit benutzerdefinierten Daten — einzeln oder im Batch aus einer JSON-Datei.

Kurz: Du gibst eine ODF-Vorlage und Daten (CLI-Argumente oder JSON) vor; certgen ersetzt Platzhalter in der Vorlage und schreibt ausgefüllte ODT-Dateien.

---

## Inhaltsverzeichnis

- Voraussetzungen
- Installation / Build
- Allgemeine Nutzung & Hilfe
- Befehle
  - fill (einzelnes Zertifikat)
  - batch (stapelweise Erzeugung aus JSON)
  - example (JSON-Beispieldatei erzeugen)
  - create-json (interaktiver JSON-Generator)
- JSON-Beispiel (einfach / erweitert)
- Custom Fields (zusätzliche Platzhalter)
- Ausgabe-Dateinamen & Sanitisierung
- Logging & Debugging
- Fehlerbehebung
- Lizenz

---

## Voraussetzungen

- Rust Toolchain (rustc + cargo) — https://www.rust-lang.org/tools/install
- ODF-Vorlage (.odt) mit Platzhaltern (die Platzhalter-Namen müssen den Keys in JSON / CLI entsprechen)
- (Für Batch) JSON-Datei mit einem Array von Zertifikats-Objekten

---

## Installation / Build

Aus dem Quellcode bauen:

```bash
git clone https://github.com/DudeTux42/certgen.git
cd certgen
cargo build --release
# ausführbares: target/release/certgen
```

Optional systemweit installieren:

```bash
cargo install --path .
```

---

## Allgemeine Nutzung & Hilfe

Das CLI bietet Subkommandos. Für die aktuelle Liste / genaue Flag-Namen:

```bash
certgen --help
certgen <subcommand> --help
```

---

## Befehle

Die Implementierung enthält die folgenden Subkommandos: fill, batch, example und create-json. Unten sind typische Aufrufe und Beschreibungen.

1) fill — Einzelnes Zertifikat befüllen

Beschreibung:
- Befüllt eine Vorlage einmalig mit Werten, die du per CLI übergibst.

Typischer Aufruf:

```bash
certgen fill template.odt \
-o output.odt \
--name "Max Mustermann" \
--title "Rust Workshop" \
--date "2025-11-11" \
--agenda "· Modul 1\n· Modul 2" \
--custom-field INSTRUCTOR="Dr. Schmidt" \
--custom-field HOURS="40"
```

Erklärung der wichtigsten Optionen (Namen können in --help nachgesehen werden):
- template.odt: Pfad zur ODF-Vorlage
- -o / --output: Ausgabedatei
- --name: Teilnehmer / Empfänger (wird als `name` verwendet)
- --title: Titel / Kursname (wird intern als Feld `TITLE` hinzugefügt)
- --date: Datum (z. B. Ausstellungsdatum)
- --date-from / --date-to: (optional) Zeitraumangaben
- --agenda: Mehrzeilige Agenda / Kursinhalt
- --custom-field KEY=VALUE: zusätzliche Platzhalter (mehrfach möglich)

Hinweis: Das Programm baut intern ein Mapping aus Feldnamen → Werte (z. B. `TITLE`, `NAME`, `DATE`, u. a.) und übergibt dieses an die ODF-Füllroutine.

2) batch — Batch-Verarbeitung aus JSON

Beschreibung:
- Liest eine JSON-Datei mit einem Array von Objekten ein und erzeugt für jeden Eintrag eine ausgefüllte ODT-Datei.

Typischer Aufruf:

```bash
certgen batch template.odt participants.json out_dir
```

Parameter:
- template.odt: Vorlagendatei
- participants.json: JSON-Datei (Array von Objekten — siehe Beispiel weiter unten)
- out_dir: Zielverzeichnis für erzeugte Zertifikate

Dateinamenskonvention:
- Erzeugte Dateien heißen: certificate_{index}_{sanitized_name}.odt  
  Beispiel: certificate_1_Max_Mustermann.odt

3) example — JSON-Beispiel erzeugen (non-interaktiv)

Beschreibung:
- Erzeugt eine Beispiel-JSON-Datei (pretty-printed). Es gibt eine einfache und eine erweiterte Variante.

Aufruf:

```bash
certgen example -o example.json
certgen example -o example_extended.json --extended
```

4) create-json — Interaktiver JSON-Generator

Beschreibung:
- Führt interaktiv durch das Anlegen von Datensätzen und schreibt die Ergebnisse in die angegebene JSON-Datei.

Aufruf:

```bash
certgen create-json -o schulungstitel.json
```

---

## JSON-Beispiel (aus dem Programm)

Einfaches Beispiel (pretty JSON):

```json
[
  {
    "name": "Max Mustermann",
    "date": "15.01.2024",
    "agenda": "· Rust Grundlagen\n· Ownership & Borrowing\n· Error Handling",
    "TITLE": "Rust Grundlagen Workshop"
  },
  {
    "name": "Erika Musterfrau",
    "date": "20.01.2024",
    "agenda": "· Python Basics\n· Libraries\n· Best Practices",
    "TITLE": "Python Einführung"
  }
]
```

Erweitertes Beispiel (enthält zusätzliche Felder wie DATE_FROM, DATE_TO, INSTRUCTOR, HOURS):

```json
[
  {
    "name": "Max Mustermann",
    "date": "15.01.2024",
    "date_from": "10.01.2024",
    "date_to": "15.01.2024",
    "agenda": "· Modul 1: Grundlagen\n· Modul 2: Advanced\n· Modul 3: Praxis",
    "TITLE": "Rust Programmierung Intensivkurs",
    "INSTRUCTOR": "Dr. Schmidt",
    "HOURS": "40"
  },
  {
    "name": "Erika Musterfrau",
    "date": "20.01.2024",
    "agenda": "· Python Basics\n· Data Science\n· Machine Learning",
    "TITLE": "Python für Data Science",
    "INSTRUCTOR": "Prof. Müller",
    "HOURS": "8"
  }
]
```

Diese JSON-Dateien kannst du direkt mit `certgen batch` verwenden.

---

## Custom Fields (zusätzliche Platzhalter)

- In JSON: füge beliebige Schlüssel/Werte in jedes Objekt ein — diese werden 1:1 als Platzhalter-Namen übernommen (z. B. `"INSTRUCTOR": "Dr. Schmidt"`).
- Per CLI (single fill): Nutze wiederholbare Flags wie `--custom-field KEY=VALUE` (Beispiel oben). Jeder Eintrag wird als weiterer Platzhalter in die Ersetzungstabelle übernommen.
- Achte darauf, dass die Platzhalter-Namen in deiner ODF-Vorlage exakt den Keys entsprechen (Groß-/Kleinschreibung beachten).

---

## Ausgabe-Dateinamen & Sanitisierung

Beim Batch-Modus erzeugt certgen Dateinamen im Format:

certificate_{index}_{sanitized_name}.odt

Sanitisierung (vereinfachte Regeln, wie sie im Code implementiert sind):
- Erlaubte Zeichen bleiben: a–z, A–Z, 0–9, '-' und '_'
- Leerzeichen → '_'
- Umlaute werden ersetzt: ä → a, ö → o, ü → u, ß → s
- Alle anderen Zeichen → '_'

Beispiel: "Müller & Söhne" → "Muller___Sohne" (je nach Anzahl der Sonderzeichen werden '_' eingesetzt)

---

## Logging & Debugging

certgen nutzt die Standard-Logging-Umgebung. Für detailliertere Ausgaben:

```bash
RUST_LOG=info certgen ...
RUST_LOG=debug certgen ...
```

Wenn du verbose-Mode in der CLI aktivierst (falls verfügbar), werden ebenfalls ausführlichere Logs initialisiert.

---

## Fehlerbehebung — Häufige Probleme

- Datei nicht gefunden: Pfad prüfen, Leserechte sicherstellen.
- JSON-Parsing-Fehler: JSON-Datei auf Gültigkeit prüfen; Batch erwartet ein Array aus Objekten.
- Platzhalter werden nicht ersetzt: Stelle sicher, dass die Platzhalternamen in der ODT-Vorlage mit den Keys in JSON/CLI übereinstimmen.
- Ausgabe leer / nicht ersetzt: Prüfe Logs (RUST_LOG) und teste mit einem Minimalfall (ein einfacher Platzhalter und ein kleines JSON-Objekt).

Wenn ein Fehler nicht klar ist, teste mit:
- Einfache Vorlage mit einem offensichtlichen Platzhalter (z. B. NAME)
- Einfache JSON-Datei mit einem Objekt für Batch oder Werte per CLI für Single-Fill
- `certgen --help` für genaue Flag-Bezeichnungen

---

## Lizenz

Siehe LICENSE-Datei im Repository.

---
