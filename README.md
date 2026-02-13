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
- Senden der Mails per Bash Script
- Ausgabe-Dateinamen & Sanitisierung
- Logging & Debugging
- Fehlerbehebung
- Lizenz

---

## Voraussetzungen

- Rust Toolchain (rustc + cargo) — https://www.rust-lang.org/tools/install
- ODF-Vorlage (.odt) mit Platzhaltern (die Platzhalter-Namen müssen den Keys in JSON / CLI entsprechen)
- (Für Batch) JSON-Datei mit einem Array von Zertifikats-Objekten
- Libre Office muss installiert sein für das konvertieren der ODT-Dateien in PDF'schreibt
- Swaks muss auf dem Gerät installiert sein für das Sendeml Script.

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

Die Implementierung enthält die folgenden Subkommandos: create-json, batch, fill und example. Die typische Arbeitsweise folgt diesem Ablauf:

1) create-json — Interaktiver JSON-Generator (Schritt 1)

Beschreibung:
- Führt interaktiv durch das Anlegen von Datensätzen für Teilnehmer und schreibt die Ergebnisse in eine JSON-Datei.
- Dies ist üblicherweise der erste Schritt: Hier werden alle Teilnehmerdaten erfasst.

Typischer Aufruf:

```bash
certgen create-json -o schulungstitel.json
```

Erklärung:
- -o / --output: Name der zu erzeugenden JSON-Datei
- Das Programm fragt interaktiv nach Name, Titel, Datum, Agenda etc. für jeden Teilnehmer
- Die erfassten Daten werden als JSON-Array gespeichert

2) batch — Batch-Verarbeitung aus JSON (Schritt 2)

Beschreibung:
- Liest die zuvor erstellte JSON-Datei ein und erzeugt für jeden Eintrag automatisch eine ausgefüllte ODT-Datei.
- Dies ist der Hauptbefehl für die Massenerstellung von Zertifikaten.

Typischer Aufruf:

```bash
certgen batch template.odt schulungstitel.json out_dir
```

Parameter:
- template.odt: Vorlagendatei
- schulungstitel.json: JSON-Datei mit Teilnehmerdaten (aus Schritt 1)
- out_dir: Zielverzeichnis für erzeugte Zertifikate

Dateinamenskonvention:
- Erzeugte Dateien heißen: certificate_{index}_{sanitized_name}.odt  
  Beispiel: certificate_1_Max_Mustermann.odt

3) fill — Einzelnes Zertifikat befüllen (optional)

Beschreibung:
- Befüllt eine Vorlage einmalig mit Werten, die per CLI übergeben werden.
- Dieser Befehl wird in der Praxis selten benötigt, da normalerweise der Batch-Modus verwendet wird.
- Nützlich für Testzwecke oder Einzelfälle.

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

Erklärung der wichtigsten Optionen:
- template.odt: Pfad zur ODF-Vorlage
- -o / --output: Ausgabedatei
- --name: Teilnehmer / Empfänger
- --title: Titel / Kursname
- --date: Datum (z. B. Ausstellungsdatum)
- --date-from / --date-to: (optional) Zeitraumangaben
- --agenda: Mehrzeilige Agenda / Kursinhalt
- --custom-field KEY=VALUE: zusätzliche Platzhalter (mehrfach möglich)

4) example �� JSON-Beispiel erzeugen (Hilfsfunktion)

Beschreibung:
- Erzeugt eine Beispiel-JSON-Datei ohne interaktive Eingabe.
- Nützlich, um die Struktur der JSON-Datei zu verstehen oder manuell anzupassen.

Aufruf:

```bash
certgen example -o example.json
certgen example -o example_extended.json --extended
```

---

**Typischer Workflow:**
1. `certgen create-json -o teilnehmer.json` → Teilnehmerdaten erfassen
2. `certgen batch template.odt teilnehmer.json zertifikate/` → Alle Zertifikate erstellen

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

## E-Mail-Versand (sendeml Script)

### Aktueller Stand

Derzeit existiert ein separates Bash-Script `sendeml`, das den automatischen Versand von vorbereiteten E-Mail-Dateien (.eml) ermöglicht.

### Funktionsweise

Das Script durchläuft alle `.eml`-Dateien im aktuellen Verzeichnis und versendet sie nacheinander per SMTP:

```bash
./sendeml
```

**Was passiert:**
1. Das Script fragt interaktiv nach dem SMTP-Passwort
2. Für jede `.eml`-Datei im Verzeichnis:
   - Wird die Empfängeradresse aus dem `To:`-Header der EML-Datei gelesen
   - Die E-Mail wird via `swaks` über den konfigurierten SMTP-Server versendet
   - Bei Erfolg wird die Datei in `.eml.sent` umbenannt (als Archiv)
   - Bei Fehler bleibt die Datei unverändert
3. Am Ende wird eine Zusammenfassung angezeigt (erfolgreich vs. fehlgeschlagen)

**Voraussetzungen:**
- `swaks` muss installiert sein
- SMTP-Server, Benutzername und Absenderadresse müssen im Script konfiguriert werden
- Die `.eml`-Dateien müssen korrekt formatiert sein (mit `To:`-Header)

**Konfiguration:**
Im Script müssen folgende Variablen angepasst werden:
```bash
SMTP_SERVER="smtp.example.com:587"
SMTP_USER="benutzername"
FROM_ADDRESS="absender@example.com"
```

### Geplante Integration

**Diese Funktionalität soll zukünftig direkt in das `certgen`-Programm integriert werden**, wobei der Umweg über EML-Dateien entfällt. Stattdessen sollen E-Mails direkt mit der Rust-Library `lettre` erstellt und versendet werden:

```bash
# Geplant für zukünftige Version:
certgen send \
--json teilnehmer.json \
--cert-dir zertifikate/ \
--subject "Ihr Zertifikat - {TITLE}" \
--body-template email.txt \
--smtp-config smtp.toml
```

**Vorteile der geplanten Lösung:**
- Keine temporären EML-Dateien mehr nötig
- Direkte Integration in Rust (keine externe Abhängigkeit von `swaks`)
- Besseres Fehlerhandling und Logging
- Template-Unterstützung für E-Mail-Texte mit Platzhaltern
- SMTP-Konfiguration über TOML-Datei

Der vereinfachte Workflow:
1. `certgen create-json -o teilnehmer.json` → Daten erfassen
2. `certgen batch template.odt teilnehmer.json zertifikate/` → Zertifikate erstellen
3. `certgen send --json teilnehmer.json --cert-dir zertifikate/` → Direkt versenden (ohne EML-Zwischenschritt)

Bis zur vollständigen Integration kann das separate `sendeml`-Script verwendet werden.

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
