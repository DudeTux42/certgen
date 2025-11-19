use certgen::{Cli, Commands, CertificateData, OdfDocument, Result, CertgenError};
use clap::Parser;
use log::{error, info};
use serde_json::Value;
use std::path::Path;

fn main() {
    if let Err(e) = run() {
        error!("Error: {}", e);
        std::process::exit(1);
    }
}

fn run() -> Result<()> {
    let cli = Cli::parse();

    // Logging initialisieren
    if cli.verbose {
        env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("debug")).init();
    } else {
        env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();
    }

    match cli.command {
        Commands::Fill {
            template,
            output,
            name,
            title,
            date,
            date_from,
            date_to,
            agenda,
            custom_fields,
        } => {
            info!("Filling single certificate");
            fill_single(
                &template,
                &output,
                &name,
                &title,
                &date,
                date_from.as_deref(),
                date_to.as_deref(),
                &agenda,
                custom_fields,
            )?;
            println!("✓ Certificate created: {}", output);
        }

        Commands::Batch {
            template,
            json,
            output_dir,
        } => {
            info!("Starting batch processing");
            let count = fill_batch(&template, &json, &output_dir)?;
            println!("✓ Created {} certificates in {}", count, output_dir);
        }

        Commands::Example { output, extended } => {
            info!("Generating example JSON");
            generate_example(&output, extended)?;
            println!("✓ Example file created: {}", output);
        }

        Commands::CreateJson { output } => {
            certgen::interactive::create_json_interactive(&output)?;
        }
    }

    Ok(())
}

fn fill_single(
    template: &str,
    output: &str,
    name: &str,
    title: &str,
    date: &str,
    date_from: Option<&str>,
    date_to: Option<&str>,
    agenda: &str,
    custom_fields: Vec<(String, String)>,
) -> Result<()> {
    let doc = OdfDocument::open(template)?;
    
    let mut data = CertificateData::new(
        name.to_string(),
        date.to_string(),
        agenda.to_string(),
    );

    data.add_field("TITLE".to_string(), title.to_string());

    if let (Some(from), Some(to)) = (date_from, date_to) {
        data.date_from = Some(from.to_string());
        data.date_to = Some(to.to_string());
    }

    for (key, value) in custom_fields {
        info!("Adding custom field: {} = {}", key, value);
        data.add_field(key, value);
    }

    // Wenn .pdf als Ausgabe gewünscht ist, benutze die neue PDF-Kette
    if output.to_lowercase().ends_with(".pdf") {
        doc.fill_and_save_pdf(output, &data.to_replacements())?;
    } else {
        doc.fill_and_save(output, &data.to_replacements())?;
    }
    Ok(())
}


fn fill_batch(template: &str, json_path: &str, output_dir: &str) -> Result<usize> {
    let doc = OdfDocument::open(template)?;

    // Lies die ganze JSON-Datei als Value, damit wir später die erzeugten Dateinamen zurückschreiben können
    let content = std::fs::read_to_string(json_path)?;
    let mut v: Value = serde_json::from_str(&content)?;

    // Erwartet ein top-level Array
    let arr = match v.as_array_mut() {
        Some(a) => a,
        None => {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "Expected top-level JSON array in batch file",
            )
            .into())
        }
    };

    std::fs::create_dir_all(output_dir)?;

    let mut created = 0usize;

    for (idx, item) in arr.iter_mut().enumerate() {
        // bestimme das CertificateData-Objekt:
        // - falls wrapper { "email": "...", "certificate": { ... } } -> benutze das innere .certificate
        // - sonst: item selbst sollte ein CertificateData-Objekt sein
        let cert_value = if item.get("certificate").is_some() {
            item.get("certificate").unwrap().clone()
        } else {
            item.clone()
        };

        // Deserialisiere in CertificateData (explizit CertgenError verwenden, um Ambiguität zu vermeiden)
        let cert: CertificateData = serde_json::from_value(cert_value)
            .map_err(CertgenError::from)?;

        // Erzeuge Dateinamen wie zuvor: <name>_<title>.pdf (sanitisiert)
        let cleaned_name = sanitize_filename(&cert.name);
        let title = cert.custom_fields
            .get("TITLE")
            .map(|t| sanitize_filename(t))
            .unwrap_or_else(|| "Kurs".to_string());

        let filename = format!("{}_{}.pdf", cleaned_name, title);
        let output_path = Path::new(output_dir).join(&filename);
        let output_str = output_path.to_str().unwrap();

        // Erstelle das PDF (fill_and_save_pdf löscht die temporäre .odt selbst)
        doc.fill_and_save_pdf(output_str, &cert.to_replacements())?;

        // Schreibe den generierten Dateinamen zurück in das JSON-Objekt
        // Hier schreibe ich den Pfad mit Ordnernamen: "<output_dir>/<filename>"
        let stored_path_string = Path::new(output_dir)
            .join(&filename)
            .to_string_lossy()
            .to_string();

        if let Value::Object(map) = item {
            map.insert("generated_file".to_string(), Value::String(stored_path_string));
        }

        created += 1;
        info!("Created [{}] -> {}", idx, output_str);
    }

    // Schreibe die aktualisierte JSON-Datei zurück (überschreibt input file)
    let pretty = serde_json::to_string_pretty(&v)?;
    std::fs::write(json_path, pretty)?;

    Ok(created)
}

fn generate_example(output: &str, extended: bool) -> Result<()> {
    let examples = if extended {
        serde_json::json!([
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
        ])
    } else {
        serde_json::json!([
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
        ])
    };

    let json = serde_json::to_string_pretty(&examples)?;
    std::fs::write(output, json)?;

    Ok(())
}

fn sanitize_filename(s: &str) -> String {
    s.chars()
        .map(|c| match c {
            'a'..='z' | 'A'..='Z' | '0'..='9' | '-' | '_' => c,
            ' ' => '_',
            'ä' => 'a',
            'ö' => 'o',
            'ü' => 'u',
            'ß' => 's',
            _ => '_',
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sanitize_filename() {
        assert_eq!(sanitize_filename("Max Mustermann"), "Max_Mustermann");
        assert_eq!(sanitize_filename("Test/File"), "Test_File");
        assert_eq!(sanitize_filename("Müller"), "Muller");
    }
}
