use certgen::{Cli, Commands, CertificateData, LatexDocument, Result, CertgenError};
use clap::Parser;
use log::{error, info};
use serde_json::Value;
use std::path::Path;

mod mail;

// ... (DEFAULT_EMAIL_BODY_* unverändert lassen)

fn main() {
    if let Err(e) = run() {
        error!("Error: {}", e);
        std::process::exit(1);
    }
}

fn run() -> Result<()> {
    let cli = Cli::parse();

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
    let doc = LatexDocument::open(template)?;

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

    doc.fill_and_save_pdf(output, &data.to_replacements())?;
    Ok(())
}

fn fill_batch(template: &str, json_path: &str, output_dir: &str) -> Result<usize> {
    let doc = LatexDocument::open(template)?;

    let content = std::fs::read_to_string(json_path)?;
    let mut v: Value = serde_json::from_str(&content)?;

    let arr = match v.as_array_mut() {
        Some(a) => a,
        None => {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "Expected top-level JSON array in batch file",
            ).into())
        }
    };

    std::fs::create_dir_all(output_dir)?;
    let mut created = 0usize;

    for (idx, item) in arr.iter_mut().enumerate() {
        let cert_value = if item.get("certificate").is_some() {
            item.get("certificate").unwrap().clone()
        } else {
            item.clone()
        };

        let cert: CertificateData = serde_json::from_value(cert_value).map_err(CertgenError::from)?;

        let cleaned_name = sanitize_filename(&cert.name);
        let title = cert.custom_fields
            .get("TITLE")
            .map(|t| sanitize_filename(t))
            .unwrap_or_else(|| "Kurs".to_string());

        let filename = format!("{}_{}.pdf", cleaned_name, title);
        let output_path = Path::new(output_dir).join(&filename);
        let output_str = output_path.to_str().unwrap();

        doc.fill_and_save_pdf(output_str, &cert.to_replacements())?;

        let stored_path_string = Path::new(output_dir)
            .join(&filename)
            .to_string_lossy()
            .to_string();

        if let Value::Object(map) = item {
            map.insert("generated_file".to_string(), Value::String(stored_path_string));
        }

        if let Some(Value::String(email_addr)) = item.get("email") {
            let eml_dir = Path::new(output_dir).join("emails");
            std::fs::create_dir_all(&eml_dir)?;

            let eml_filename = format!("{}.eml", filename.trim_end_matches(".pdf"));
            let eml_path = eml_dir.join(&eml_filename);

            let subject = format!("Ihr Zertifikat: {}", title);
            let body_template = DEFAULT_EMAIL_BODY_HTML;
            let use_html = true;

            let mut body = body_template
                .replace("{{NAME}}", &cert.name)
                .replace("{{CERT}}", &filename);

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

            mail::create_eml(
                email_addr,
                &subject,
                &body,
                &cert.name,
                Path::new(output_str),
                &eml_path,
                use_html,
            )?;

            if let Value::Object(map) = item {
                map.insert(
                    "generated_eml".to_string(),
                    Value::String(eml_path.to_string_lossy().to_string()),
                );
            }
        }

        created += 1;
        info!("Created [{}] -> {}", idx, output_str);
    }

    let pretty = serde_json::to_string_pretty(&v)?;
    std::fs::write(json_path, pretty)?;

    Ok(created)
}

// generate_example + sanitize_filename + tests bleiben wie bisher
