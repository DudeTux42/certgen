use std::fs;
use std::io::Write;
use std::path::Path;

use base64::{engine::general_purpose, Engine as _};
use chrono::Utc;

use certgen::error::Result;

/// Erzeugt eine .eml Datei mit HTML- oder Plain-Text-Körper und einem PDF-Anhang.
/// - `to` ist die Empfänger-E-Mail-Adresse (wird in "To:" geschrieben)
/// - `subject` ist der Mail-Subject
/// - `body_template` ist ein String mit Platzhaltern `{{NAME}}` und `{{CERT}}`
/// - `name` wird für `{{NAME}}` eingesetzt
/// - `attachment_path` ist der Pfad zur PDF-Datei, die angehängt wird
/// - `output_eml_path` ist der Pfad zur zu erzeugenden .eml-Datei
/// - `use_html` wenn true, wird der Body als HTML interpretiert
pub fn create_eml(
    to: &str,
    subject: &str,
    body_template: &str,
    name: &str,
    attachment_path: &Path,
    output_eml_path: &Path,
    use_html: bool,
) -> Result<()> {
    // Lese Attachment
    let attachment_bytes = fs::read(attachment_path)?;
    let attachment_filename = attachment_path
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("attachment.pdf");

    // Ersetze Platzhalter im Body
    let body = body_template
        .replace("{{NAME}}", name)
        .replace("{{CERT}}", attachment_filename);

    // Boundaries erzeugen
    let boundary_outer = format!("----=_CERTGEN_OUTER_{}", Utc::now().timestamp_nanos());
    let boundary_inner = format!("----=_CERTGEN_INNER_{}", Utc::now().timestamp_nanos());

    // Header
    let date = Utc::now().to_rfc2822();
    let from = "zertifikate@b1-systems.de";

    let mut eml = String::new();
    eml.push_str(&format!("From: {}\r\n", from));
    eml.push_str(&format!("To: {}\r\n", to));
    eml.push_str(&format!("Subject: {}\r\n", subject));
    eml.push_str("MIME-Version: 1.0\r\n");
    eml.push_str(&format!("Date: {}\r\n", date));
    eml.push_str(&format!(
        "Content-Type: multipart/mixed; boundary=\"{}\"\r\n",
        boundary_outer
    ));
    eml.push_str("\r\n"); // Header / Body-Trenner

    if use_html {
        // Multipart/alternative für Text + HTML
        eml.push_str(&format!("--{}\r\n", boundary_outer));
        eml.push_str(&format!(
            "Content-Type: multipart/alternative; boundary=\"{}\"\r\n",
            boundary_inner
        ));
        eml.push_str("\r\n");

        // Plain-Text-Version (Fallback)
        eml.push_str(&format!("--{}\r\n", boundary_inner));
        eml.push_str("Content-Type: text/plain; charset=\"utf-8\"\r\n");
        eml.push_str("Content-Transfer-Encoding: 7bit\r\n");
        eml.push_str("\r\n");
        // Vereinfachte Plain-Text-Version (HTML-Tags entfernen)
        let plain_body = strip_html_simple(&body);
        eml.push_str(&plain_body);
        eml.push_str("\r\n");

        // HTML-Version
        eml.push_str(&format!("--{}\r\n", boundary_inner));
        eml.push_str("Content-Type: text/html; charset=\"utf-8\"\r\n");
        eml.push_str("Content-Transfer-Encoding: 7bit\r\n");
        eml.push_str("\r\n");
        eml.push_str(&body);
        eml.push_str("\r\n");

        // Ende multipart/alternative
        eml.push_str(&format!("--{}--\r\n", boundary_inner));
    } else {
        // Nur Plain-Text
        eml.push_str(&format!("--{}\r\n", boundary_outer));
        eml.push_str("Content-Type: text/plain; charset=\"utf-8\"\r\n");
        eml.push_str("Content-Transfer-Encoding: 7bit\r\n");
        eml.push_str("\r\n");
        eml.push_str(&body);
        eml.push_str("\r\n");
    }

    // Attachment (PDF)
    eml.push_str(&format!("--{}\r\n", boundary_outer));
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
    eml.push_str(&format!("--{}--\r\n", boundary_outer));

    // Schreibe .eml Datei
    let mut f = fs::File::create(output_eml_path)?;
    f.write_all(eml.as_bytes())?;
    f.flush()?;

    Ok(())
}

/// Entfernt HTML-Tags aus einem String (einfache Implementierung)
fn strip_html_simple(html: &str) -> String {
    use regex::Regex;
    
    // Entferne <style> und <script> Blöcke komplett
    let re_style = Regex::new(r"(?is)<style[^>]*>.*?</style>").unwrap();
    let re_script = Regex::new(r"(?is)<script[^>]*>.*?</script>").unwrap();
    let mut result = re_style.replace_all(html, "").to_string();
    result = re_script.replace_all(&result, "").to_string();
    
    // Ersetze <br>, <p>, <div> etc. durch Zeilenumbrüche
    let re_br = Regex::new(r"(?i)<br\s*/?>").unwrap();
    result = re_br.replace_all(&result, "\n").to_string();
    
    let re_block = Regex::new(r"(?i)</?(p|div|h[1-6])[^>]*>").unwrap();
    result = re_block.replace_all(&result, "\n").to_string();
    
    // Entferne alle anderen HTML-Tags
    let re_tags = Regex::new(r"<[^>]+>").unwrap();
    result = re_tags.replace_all(&result, "").to_string();
    
    // HTML-Entities dekodieren (einfach)
    result = result
        .replace("&nbsp;", " ")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&amp;", "&")
        .replace("&quot;", "\"");
    
    // Mehrfache Leerzeilen reduzieren
    let re_multiline = Regex::new(r"\n{3,}").unwrap();
    result = re_multiline.replace_all(&result, "\n\n").to_string();
    
    result.trim().to_string()
}
