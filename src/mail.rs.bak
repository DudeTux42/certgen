use std::fs;
use std::io::Write;
use std::path::Path;

use base64::{engine::general_purpose, Engine as _};
use chrono::Utc;

use certgen::error::Result;

/// Erzeugt eine .eml Datei mit einfachem Textkörper und einem PDF-Anhang.
/// - `to` ist die Empfänger-E-Mail-Adresse (wird in "To:" geschrieben)
/// - `subject` ist der Mail-Subject
/// - `body_template` ist ein String mit Platzhaltern `<name>` und `<cert>`
/// - `name` wird für `<name>` eingesetzt
/// - `attachment_path` ist der Pfad zur PDF-Datei, die angehängt wird
/// - `output_eml_path` ist der Pfad zur zu erzeugenden .eml-Datei
pub fn create_eml(
    to: &str,
    subject: &str,
    body_template: &str,
    name: &str,
    attachment_path: &Path,
    output_eml_path: &Path,
) -> Result<()> {
    // Lese Attachment
    let attachment_bytes = fs::read(attachment_path)?;
    let attachment_filename = attachment_path
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("attachment.pdf");

    // Ersetze Platzhalter im Body
    let body = body_template
        .replace("<name>", name)
        .replace("<cert>", attachment_filename);

    // Boundary erzeugen (einfach, eindeutig genug)
    let boundary = format!("----=_CERTGEN_{}",
        Utc::now().timestamp_nanos());

    // Header
    let date = Utc::now().to_rfc2822();
    // From: kannst du anpassen; hier ein neutraler Default
    let from = "lindermayr@b1-systems.de";

    let mut eml = String::new();
    eml.push_str(&format!("From: {}\r\n", from));
    eml.push_str(&format!("To: {}\r\n", to));
    eml.push_str(&format!("Subject: {}\r\n", subject));
    eml.push_str("MIME-Version: 1.0\r\n");
    eml.push_str(&format!(
        "Date: {}\r\n",
        date
    ));
    eml.push_str(&format!(
        "Content-Type: multipart/mixed; boundary=\"{}\"\r\n",
        boundary
    ));
    eml.push_str("\r\n"); // Header / Body-Trenner

    // Teil 1: Text-Teil (plain)
    eml.push_str(&format!("--{}\r\n", boundary));
    eml.push_str("Content-Type: text/plain; charset=\"utf-8\"\r\n");
    eml.push_str("Content-Transfer-Encoding: 7bit\r\n");
    eml.push_str("\r\n");
    eml.push_str(&body);
    eml.push_str("\r\n");

    // Teil 2: Attachment (PDF)
    eml.push_str(&format!("--{}\r\n", boundary));
    eml.push_str(&format!(
        "Content-Type: application/pdf; name=\"{}\"\r\n",
        attachment_filename
    ));
    eml.push_str(&format!(
        "Content-Disposition: attachment; filename=\"{}\"\r\n",
        attachment_filename
    ));
    eml.push_str("Content-Transfer-Encoding: base64\r\n");
    eml.push_str("\r\n");

    // Base64 kodieren, RFC-konforme Zeilenlänge (76 Zeichen)
    let b64 = general_purpose::STANDARD.encode(&attachment_bytes);
    for chunk in b64.as_bytes().chunks(76) {
        eml.push_str(&format!("{}\r\n", std::str::from_utf8(chunk).unwrap()));
    }

    // Ende-Marker
    eml.push_str(&format!("--{}--\r\n", boundary));

    // Schreibe .eml Datei (überschreibt falls vorhanden)
    let mut f = fs::File::create(output_eml_path)?;
    f.write_all(eml.as_bytes())?;
    f.flush()?;

    Ok(())
}

