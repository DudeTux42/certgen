use certgen::{Cli, Commands, CertificateData, OdfDocument, Result};
use clap::Parser;
use log::{error, info};
use std::collections::HashMap;

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

    doc.fill_and_save(output, &data.to_replacements())?;
    Ok(())
}

fn fill_batch(template: &str, json_path: &str, output_dir: &str) -> Result<usize> {
    let doc = OdfDocument::open(template)?;
    let certificates = CertificateData::batch_from_json_file(json_path)?;

    let batch_data: Vec<(String, HashMap<String, String>)> = certificates
        .iter()
        .enumerate()
        .map(|(i, cert)| {
            let filename = format!("certificate_{}_{}.odt", i + 1, sanitize_filename(&cert.name));
            (filename, cert.to_replacements())
        })
        .collect();

    let created = doc.batch_fill(output_dir, batch_data)?;
    Ok(created.len())
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
