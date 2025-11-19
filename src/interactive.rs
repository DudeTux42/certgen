use crate::template::CertificateData;
use crate::error::Result;
use std::io::{self, Write};
use serde::Serialize;

/// Liest eine Zeile von stdin
fn read_line(prompt: &str) -> io::Result<String> {
    print!("{}", prompt);
    io::stdout().flush()?;
    
    let mut input = String::new();
    io::stdin().read_line(&mut input)?;
    Ok(input.trim().to_string())
}

/// Liest eine optionale Zeile (leer = None)
fn read_optional_line(prompt: &str) -> io::Result<Option<String>> {
    let input = read_line(prompt)?;
    if input.is_empty() {
        Ok(None)
    } else {
        Ok(Some(input))
    }
}

#[derive(Serialize)]
struct ParticipantEntry {
    email: String,
    certificate: CertificateData,
}

/// Interaktives Erstellen einer JSON-Datei
pub fn create_json_interactive(output_path: &str) -> Result<()> {
    println!("╔══════════════════════════════════════════════════════╗");
    println!("║  📝 Interaktiver JSON-Generator für Zertifikate     ║");
    println!("╚══════════════════════════════════════════════════════╝");
    println!();

    // Titel abfragen
    println!("📌 Allgemeine Informationen (für alle Teilnehmer)");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    
    let title = read_line("Kurstitel: ")?;
    if title.is_empty() {
        println!("❌ Kurstitel darf nicht leer sein!");
        return Ok(());
    }

    // Datum abfragen (Start zuerst, dann optional Ende)
    println!();
    let date_from = read_line("Datum / Start-Datum (z.B. 15.01.2024): ")?;
    if date_from.is_empty() {
        println!("❌ Datum darf nicht leer sein!");
        return Ok(());
    }
    
    let date_to = read_optional_line("End-Datum (leer lassen für eintägigen Kurs): ")?;

    // Agenda abfragen
    println!();
    println!("📋 Agenda / Kursinhalte");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("Geben Sie die Agendapunkte einzeln ein (Maximal 10 Zeilen, leer = fertig):");
    
    let mut agenda_items = Vec::new();
    let mut item_number = 1;
    
    loop {
        if item_number > 10 {
            break
        };

        let item = read_line(&format!("  {}. ", item_number))?;
        if item.is_empty() {
            break;
        }

        agenda_items.push(format!("· {}", item));
        item_number += 1;
    }

    if agenda_items.is_empty() {
        println!("⚠️  Keine Agenda-Punkte eingegeben. Verwende Platzhalter.");
        agenda_items.push("· Kursinhalt".to_string());
    }

    let agenda = agenda_items.join("\n");

    // Custom Fields abfragen
    println!();
    println!("🔧 Zusätzliche Felder (optional)");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("Geben Sie zusätzliche Felder an (z.B. INSTRUCTOR, HOURS)");
    println!("Feldname leer lassen = fertig");
    
    let mut custom_fields = std::collections::HashMap::new();
    
    loop {
        println!();
        let field_name = read_line("Feldname (z.B. INSTRUCTOR): ")?;
        if field_name.is_empty() {
            break;
        }
        
        let field_value = read_line(&format!("Wert für {}: ", field_name))?;
        if field_value.is_empty() {
            println!("⚠️  Wert darf nicht leer sein, Feld wird übersprungen.");
            continue;
        }
        
        custom_fields.insert(field_name.to_uppercase(), field_value);
    }

    // Zusammenfassung der Custom Fields
    if !custom_fields.is_empty() {
        println!();
        println!("✓ Folgende zusätzliche Felder werden verwendet:");
        for (key, value) in &custom_fields {
            println!("  • {}: {}", key, value);
        }
    }


    // Teilnehmer abfragen
    println!();
    println!("👥 Teilnehmer");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("Geben Sie die Namen der Teilnehmer ein (leer = fertig):");
    
    let mut participants: Vec<ParticipantEntry> = Vec::new();
    let mut participant_number = 1;
    
    loop {
        let name = read_line(&format!("  {}. Name: ", participant_number))?;
        if name.is_empty() {
            break;
        }
        
        let mail = read_line(&format!("  {}. E-Mail: ", participant_number ))?;
        // Bestimme das Haupt-Datum (für eintägig = date_from, für mehrtägig = date_to)
        let main_date = match &date_to {
            Some(to) => to.clone(),
            None => date_from.clone(),
        };
        
        // CertificateData::new erwartet (name, date, agenda)
        let mut cert_data = CertificateData::new(
            name,
            main_date,
            agenda.clone(),
        );

        // Titel hinzufügen
        cert_data.add_field("TITLE".to_string(), title.clone());

        // Datumbereich hinzufügen (falls mehrtägig)
        if let Some(ref to) = date_to {
            cert_data.date_from = Some(date_from.clone());
            cert_data.date_to = Some(to.clone());
        }

        // custom_fields hinzufügen
        for (key, value) in &custom_fields {
            cert_data.add_field(key.clone(), value.clone());
        }

        // Teilnehmer-Eintrag erstellen (E-Mail wird nur im JSON gespeichert)
        participants.push(ParticipantEntry { email: mail, certificate: cert_data });
        participant_number += 1;
    }

    if participants.is_empty() {
        println!("❌ Keine Teilnehmer eingegeben!");
        return Ok(());
    }

    // JSON speichern
    println!();
    println!("💾 Speichere JSON...");
    
    let json = serde_json::to_string_pretty(&participants)?;
    std::fs::write(output_path, json)?;

    // Zusammenfassung
    println!();
    println!("╔══════════════════════════════════════════════════════╗");
    println!("║  ✅ JSON erfolgreich erstellt!                       ║");
    println!("╚══════════════════════════════════════════════════════╝");
    println!();
    println!("📄 Datei: {}", output_path);
    println!("📊 Anzahl Teilnehmer: {}", participants.len());
    println!("📚 Kurstitel: {}", title);
    
    if let Some(to) = date_to {
        println!("📅 Zeitraum: {} bis {}", date_from, to);
    } else {
        println!("📅 Datum: {}", date_from);
    }
    
    println!("📋 Agenda-Punkte: {}", agenda_items.len());
    println!();
    println!("🚀 Nächster Schritt:");
    println!("   certgen batch -t <vorlage.odt> -j {} -o zertifikate", output_path);
    println!();

    Ok(())
}
