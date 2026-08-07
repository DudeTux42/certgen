use std::fs;
use std::io::Write;
use std::path::Path;

use base64::{engine::general_purpose, Engine as _};
use chrono::Utc;
use serde::Deserialize;
use lettre::transport::smtp::authentication::Credentials;
use lettre::{SmtpTransport, Transport};

use certgen::error::Result;

const LOGO_PNG: &[u8] = include_bytes!("../assets/b1_logo.png");

#[derive(Debug, Deserialize, Clone)]
pub struct TomlConfig {
    pub smtp_server: String,
    pub smtp_port: u16,
    pub smtp_user: String,
    pub from_addr: String,
}

/// Lädt die globale Konfiguration aus ~/.config/certgen/config.toml
pub fn load_config() -> Result<TomlConfig> {
    let home = std::env::var("HOME")
        .map_err(|_| std::io::Error::new(std::io::ErrorKind::NotFound, "HOME-Verzeichnis nicht gefunden"))?;
    let config_path = Path::new(&home).join(".config/certgen/config.toml");

    if !config_path.exists() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            format!("Konfigurationsdatei fehlt! Bitte anlegen unter: {}", config_path.display())
        ).into());
    }

    let config_content = fs::read_to_string(config_path)?;
    let config: TomlConfig = toml::from_str(&config_content)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, format!("TOML-Fehler: {}", e)))?;

    Ok(config)
}

/// Erzeugt eine .eml Datei mit HTML- oder Plain-Text-Körper und einem PDF-Anhang.
pub fn create_eml(
    from: &str,
    to: &str,
    subject: &str,
    body_template: &str,
    name: &str,
    attachment_path: &Path,
    output_eml_path: &Path,
    use_html: bool,
) -> Result<()> {
    let attachment_bytes = fs::read(attachment_path)?;
    let attachment_filename = attachment_path
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("attachment.pdf");

    let body = body_template
        .replace("{{NAME}}", name)
        .replace("{{CERT}}", attachment_filename);

    let boundary_outer = format!("----=_CERTGEN_OUTER_{}", Utc::now().timestamp_micros());
    let boundary_inner = format!("----=_CERTGEN_INNER_{}", Utc::now().timestamp_micros());

    let date = Utc::now().to_rfc2822();

    let mut eml = String::new();
    eml.push_str(&format!("From: {}\r\n", from));
    eml.push_str(&format!("To: {}\r\n", to));
    eml.push_str(&format!("Subject: {}\r\n", subject));
    eml.push_str("MIME-Version: 1.0\r\n");
    eml.push_str(&format!("Date: {}\r\n", date));
    eml.push_str(&format!("Content-Type: multipart/mixed; boundary=\"{}\"\r\n", boundary_outer));
    eml.push_str("\r\n");

    if use_html {
        eml.push_str(&format!("--{}\r\n", boundary_outer));
        eml.push_str(&format!("Content-Type: multipart/alternative; boundary=\"{}\"\r\n", boundary_inner));
        eml.push_str("\r\n");

        eml.push_str(&format!("--{}\r\n", boundary_inner));
        eml.push_str("Content-Type: text/plain; charset=\"utf-8\"\r\n");
        eml.push_str("Content-Transfer-Encoding: 7bit\r\n");
        eml.push_str("\r\n");
        let plain_body = strip_html_simple(&body);
        eml.push_str(&plain_body);
        eml.push_str("\r\n");

        eml.push_str(&format!("--{}\r\n", boundary_inner));
        eml.push_str("Content-Type: text/html; charset=\"utf-8\"\r\n");
        eml.push_str("Content-Transfer-Encoding: 7bit\r\n");
        eml.push_str("\r\n");
        eml.push_str(&body);
        eml.push_str("\r\n");

        eml.push_str(&format!("--{}--\r\n", boundary_inner));

        eml.push_str(&format!("--{}\r\n", boundary_outer));
        eml.push_str("Content-Type: image/png; name=\"b1_logo.png\"\r\n");
        eml.push_str("Content-Transfer-Encoding: base64\r\n");
        eml.push_str("Content-Disposition: Inline\r\n");
        eml.push_str("Content-ID: <logo@b1systems>\r\n");
        eml.push_str("\r\n");

        let b64 = general_purpose::STANDARD.encode(LOGO_PNG);
        for chunk in b64.as_bytes().chunks(76) {
            eml.push_str(&format!("{}\r\n", std::str::from_utf8(chunk).unwrap()));
        }
    } else {
        eml.push_str(&format!("--{}\r\n", boundary_outer));
        eml.push_str("Content-Type: text/plain; charset=\"utf-8\"\r\n");
        eml.push_str("Content-Transfer-Encoding: 7bit\r\n");
        eml.push_str("\r\n");
        eml.push_str(&body);
        eml.push_str("\r\n");
    }

    eml.push_str(&format!("--{}\r\n", boundary_outer));
    eml.push_str(&format!("Content-Type: application/pdf; name=\"{}\"\r\n", attachment_filename));
    eml.push_str(&format!("Content-Disposition: attachment; filename=\"{}\"\r\n", attachment_filename));
    eml.push_str("Content-Transfer-Encoding: base64\r\n");
    eml.push_str("\r\n");

    let b64 = general_purpose::STANDARD.encode(&attachment_bytes);
    for chunk in b64.as_bytes().chunks(76) {
        eml.push_str(&format!("{}\r\n", std::str::from_utf8(chunk).unwrap()));
    }

    eml.push_str(&format!("--{}--\r\n", boundary_outer));

    let mut f = fs::File::create(output_eml_path)?;
    f.write_all(eml.as_bytes())?;
    f.flush()?;

    Ok(())
}

/// Liest die globale TOML-Config und verschickt alle EML-Dateien aus der JSON-Datei via SMTP.
pub fn send_batch_mails(json_path: &str) -> Result<()> {
    // 1. Konfiguration laden
    let config = load_config()?;

    // 2. Secret aus Umgebungsvariable holen
    let smtp_pass = std::env::var("CERTGEN_SMTP_PASS").map_err(|_| {
        std::io::Error::new(
            std::io::ErrorKind::PermissionDenied,
            "Fehler: Die Umgebungsvariable 'CERTGEN_SMTP_PASS' ist nicht gesetzt!",
        )
    })?;

    // 3. JSON einlesen
    let content = fs::read_to_string(json_path)?;
    let v: serde_json::Value = serde_json::from_str(&content)?;
    let arr = v.as_array().ok_or_else(|| {
        std::io::Error::new(std::io::ErrorKind::InvalidData, "JSON-Struktur ungültig (Array erwartet)")
    })?;

    // 4. SMTP Transport aufbauen via STARTTLS (Nutzt den smtp_user für den Login)
    let creds = Credentials::new(config.smtp_user.clone(), smtp_pass);
    let mailer = SmtpTransport::starttls_relay(&config.smtp_server)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, format!("SMTP-Relay-Fehler: {}", e)))?
        .port(config.smtp_port)
        .credentials(creds)
        .build();

    log::info!("SMTP-Verbindung zu {} konfiguriert.", config.smtp_server);

    // 5. Senden via Raw-Bytes
    for item in arr {
        let email_addr = item.get("email").and_then(|e| e.as_str()).unwrap_or("");
        if let Some(eml_path_str) = item.get("generated_eml").and_then(|e| e.as_str()) {
            let eml_path = Path::new(eml_path_str);
            if !eml_path.exists() {
                log::warn!("EML für {} nicht gefunden, überspringe.", email_addr);
                continue;
            }

            let eml_bytes = fs::read(eml_path)?;

            // Envelope für den SMTP-Handshake: Nutzt zwingend config.from_addr (die volle Adresse) für MAIL FROM!
            let envelope = lettre::address::Envelope::new(
                Some(config.from_addr.parse().map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, format!("Ungültige Absender-Adresse (from_addr): {}", e)))?),
                vec![email_addr.parse().map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, format!("Ungültige Empfänger-Adresse: {}", e)))?]
            ).map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, format!("Envelope-Fehler: {}", e)))?;

            match mailer.send_raw(&envelope, &eml_bytes) {
                Ok(_) => log::info!("✓ Mail erfolgreich gesendet an: {}", email_addr),
                Err(e) => log::error!("❌ Senden fehlgeschlagen an {}: {}", email_addr, e),
            }
        }
    }

    Ok(())
}

fn strip_html_simple(html: &str) -> String {
    use regex::Regex;

    let re_style = Regex::new(r"(?is)<style[^>]*>.*?</style>").unwrap();
    let re_script = Regex::new(r"(?is)<script[^>]*>.*?</script>").unwrap();
    let mut result = re_style.replace_all(html, "").to_string();
    result = re_script.replace_all(&result, "").to_string();

    let re_br = Regex::new(r"(?i)<br\s*/?>").unwrap();
    result = re_br.replace_all(&result, "\n").to_string();

    let re_block = Regex::new(r"(?i)</?(p|div|h[1-6])[^>]*>").unwrap();
    result = re_block.replace_all(&result, "\n").to_string();

    let re_tags = Regex::new(r"<[^>]+>").unwrap();
    result = re_tags.replace_all(&result, "").to_string();

    result = result
        .replace("&nbsp;", " ")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&amp;", "&")
        .replace("&quot;", "\"");

    let re_multiline = Regex::new(r"\n{3,}").unwrap();
    result = re_multiline.replace_all(&result, "\n\n").to_string();

    result.trim().to_string()
}
