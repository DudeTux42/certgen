use certgen::{Cli, Commands, CertificateData, OdfDocument, Result, CertgenError};
use clap::Parser;
use log::{error, info};
use serde_json::Value;
use std::path::Path;

mod mail;

const DEFAULT_EMAIL_BODY_PLAIN: &str = "\
Guten Tag {{NAME}},

herzlichen Glückwunsch zum erfolgreichen Abschluss!

Im Anhang finden Sie Ihr persönliches Zertifikat als PDF-Datei ({{CERT}}).

Wir freuen uns, dass Sie an unserer Schulung teilgenommen haben und 
wünschen Ihnen viel Erfolg bei der Anwendung des Gelernten.

Bei Fragen stehen wir Ihnen jederzeit gerne zur Verfügung.


Mit freundlichen Grüßen
Ihr Schulungsteam";

const DEFAULT_EMAIL_BODY_HTML: &str = r#"<!DOCTYPE html>
<html>
<head>
    <meta charset="utf-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
</head>
<body style="font-family: Arial, sans-serif; line-height: 1.6; color: #333; margin: 0; padding: 0; background-color: #f4f4f4;">
    <table width="100%" cellpadding="0" cellspacing="0" style="background-color: #f4f4f4;">
        <tr>
            <td align="center" style="padding: 20px;">
                <table width="600" cellpadding="0" cellspacing="0" style="background-color: #ffffff; border-radius: 8px; overflow: hidden; box-shadow: 0 2px 4px rgba(0,0,0,0.1);">
                    <!-- Header -->
                    <tr>
                        <td style="background-color: #0066cc; color: white; padding: 30px; text-align: center;">
                            <h2 style="margin: 0; font-size: 24px;">Ihr Zertifikat ist da! 🎉</h2>
                        </td>
                    </tr>
                    
                    <!-- Body -->
                    <tr>
                        <td style="padding: 30px;">
                            <p style="margin: 0 0 15px 0;">Guten Tag <strong>{{NAME}}</strong>,</p>
                            
                            <p style="margin: 0 0 15px 0;">herzlichen Glückwunsch zum erfolgreichen Abschluss des Kurses<br>
                            <strong>{{TITLE}}</strong>!</p>
                            
                            <div style="background-color: #f0f8ff; padding: 15px; border-left: 4px solid #0066cc; margin: 20px 0;">
                                <p style="margin: 0;"><strong>Kursdetails:</strong></p>
                                <ul style="margin: 10px 0 0 0; padding-left: 20px;">
                                    <li>Datum: {{DATE}}</li>
                                    <li>Trainer/in: {{INSTRUCTOR}}</li>
                                </ul>
                            </div>
                            
                            <p style="margin: 20px 0 15px 0;">Im Anhang finden Sie Ihr persönliches Zertifikat als PDF-Datei (<strong>{{CERT}}</strong>).</p>
                            
                            <p style="margin: 0 0 15px 0;">Wir hoffen, dass Sie wertvolle Kenntnisse mitnehmen konnten und wünschen 
                            Ihnen viel Erfolg bei der praktischen Anwendung!</p>
                            
                            <p style="margin: 0;">Bei Fragen stehen wir Ihnen jederzeit gerne zur Verfügung.</p>
                        </td>
                    </tr>
                    
                    <!-- Footer -->
                    <tr>
                        <td style="background-color: #f8f9fa; padding: 20px; text-align: center; border-top: 1px solid #dee2e6;">
                            <p style="margin: 0; font-size: 14px; color: #666;">
                                Mit freundlichen Grüßen<br>
                                <strong>Ihr B1 Systems Team</strong>
                            </p>
                        </td>
                    </tr>
                </table>
            </td>
        </tr>
    </table>
</body>
</html>"#;

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

        // Wenn eine E-Mail im Eintrag vorhanden ist, erzeuge automatisch eine .eml-Datei mit Anhang
        if let Some(Value::String(email_addr)) = item.get("email") {
            // Erzeuge ein emails-Unterverzeichnis unter output_dir
            let eml_dir = Path::new(output_dir).join("emails");
            std::fs::create_dir_all(&eml_dir)?;

            // EML-Dateiname basiert auf dem PDF-Dateinamen
            let eml_filename = format!("{}.eml", filename.trim_end_matches(".pdf"));
            let eml_path = eml_dir.join(&eml_filename);

            // Subject und Body
            let subject = format!("Ihr Zertifikat: {}", title);
            
            // Wähle HTML-Template (kannst später auf Plain-Text umstellen mit DEFAULT_EMAIL_BODY_PLAIN)
            let body_template = DEFAULT_EMAIL_BODY_HTML;
            let use_html = true;  // auf false setzen für Plain-Text
            
            // Ersetze Platzhalter im Body-Template
            let mut body = body_template
                .replace("{{NAME}}", &cert.name)
                .replace("{{CERT}}", &filename);
            
            // Ersetze weitere Platzhalter aus custom_fields
            if let Some(title_val) = cert.custom_fields.get("TITLE") {
                body = body.replace("{{TITLE}}", title_val);
            }
            body = body.replace("{{DATE}}", &cert.date);
            if let Some(instructor) = cert.custom_fields.get("INSTRUCTOR") {
                body = body.replace("{{INSTRUCTOR}}", instructor);
            }
            if let Some(duration) = cert.custom_fields.get("DURATION") {
                body = body.replace("{{DURATION}}", duration);
            }

            // create_eml erwartet Pfad zur erzeugten PDF als Path
            mail::create_eml(
                email_addr,
                &subject,
                &body,
                &cert.name,
                Path::new(output_str),
                &eml_path,
                use_html,  // ← HIER: der fehlende Parameter
            )?;

            // schreibe generated_eml ins JSON (voller Pfad relativ wie to_string_lossy liefert)
            if let Value::Object(map) = item {
                map.insert("generated_eml".to_string(), Value::String(eml_path.to_string_lossy().to_string()));
            }
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
