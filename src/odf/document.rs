use crate::error::{CertgenError, Result};
use crate::odf::replacer::PlaceholderReplacer;
use std::collections::HashMap;
use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::Path;
use std::process::Command;
use zip::{ZipArchive, ZipWriter, write::FileOptions, CompressionMethod};
use log::{debug, info, warn};
use which::which; // Laufzeit-Check ob 'soffice' vorhanden

/// PDF-Konfiguration für Seitenvalidierung
#[derive(Debug, Clone)]
pub struct PdfConfig {
    /// Automatisch leere zweite Seite entfernen (Standard: true)
    pub remove_empty_second_page: bool,
    /// Bei mehr als X Seiten einen Fehler werfen (Standard: 2)
    pub max_pages: usize,
}

impl Default for PdfConfig {
    fn default() -> Self {
        Self {
            remove_empty_second_page: true,
            max_pages: 2,
        }
    }
}

/// Repräsentiert ein ODF-Dokument
pub struct OdfDocument {
    path: String,
}

impl OdfDocument {
    /// Öffnet ein ODF-Dokument
    pub fn open(path: &str) -> Result<Self> {
        if !std::path::Path::new(path).exists() {
            return Err(CertgenError::TemplateNotFound(path.to_string()));
        }
        
        Ok(Self {
            path: path.to_string(),
        })
    }

    /// Entfernt XML-Tags aus Platzhaltern
    /// Wandelt: von {{</text:span><text:span>INSTRUCTOR</text:span><text:span>}}
    /// In: von {{INSTRUCTOR}}
    fn clean_split_placeholders(content: &str) -> String {
        use regex::Regex;
        
        // Regex die {{...}} findet, auch wenn XML-Tags dazwischen sind
        // Matcht: {{ [beliebiger Text mit optionalen XML-Tags] }}
        let re = Regex::new(r"\{\{([^}]*(?:<[^>]+>[^}]*)*)\}\}").unwrap();
        
        let result = re.replace_all(content, |caps: &regex::Captures| {
            let inner = &caps[1];
            
            // Entferne alle XML-Tags aus dem Inneren
            let tag_remover = Regex::new(r"<[^>]+>").unwrap();
            let cleaned = tag_remover.replace_all(inner, "");
            
            // Entferne Whitespace
            let trimmed = cleaned.trim();
            
            format!("{{{{{}}}}}", trimmed)
        });
        
        result.to_string()
    }

    /// Füllt das Dokument mit Daten und speichert es
    pub fn fill_and_save(
        &self,
        output_path: &str,
        replacements: &HashMap<String, String>,
    ) -> Result<()> {
        info!("Processing template: {}", self.path);
        info!("Output will be written to: {}", output_path);
        
        let file = File::open(&self.path)?;
        let mut archive = ZipArchive::new(file)?;
        
        let output_file = File::create(output_path)?;
        let mut output_archive = ZipWriter::new(output_file);
        
        let replacer = PlaceholderReplacer::new();
        
        // WICHTIG: mimetype MUSS als erstes kommen und UNKOMPRIMIERT sein!
        if let Ok(mut mimetype_file) = archive.by_name("mimetype") {
            let mut content = String::new();
            mimetype_file.read_to_string(&mut content)?;
            
            let options = FileOptions::default()
                .compression_method(CompressionMethod::Stored);
            
            output_archive.start_file("mimetype", options)?;
            output_archive.write_all(content.as_bytes())?;
        }
        
        // Alle anderen Dateien
        for i in 0..archive.len() {
            let mut file = archive.by_index(i)?;
            let filename = file.name().to_string();
            
            // mimetype überspringen - haben wir schon geschrieben
            if filename == "mimetype" {
                continue;
            }
            
            debug!("Processing file: {}", filename);
            
            // content.xml und styles.xml können Text enthalten
            if filename == "content.xml" || filename == "styles.xml" {
                let mut content = String::new();
                file.read_to_string(&mut content)?;
                
                // ERST: XML-Tags aus Platzhaltern entfernen
                debug!("Cleaning split placeholders...");
                let cleaned = Self::clean_split_placeholders(&content);
                
                // DANN: Replacements durchführen (mit XML-Escaping)
                let replaced = replacer.replace_all(&cleaned, replacements);
                
                let options = FileOptions::default()
                    .compression_method(CompressionMethod::Deflated);
                
                output_archive.start_file(&filename, options)?;
                output_archive.write_all(replaced.as_bytes())?;
            } else {
                // Andere Dateien 1:1 kopieren mit Original-Kompression
                let compression = file.compression();
                let options = FileOptions::default()
                    .compression_method(compression);
                
                output_archive.start_file(&filename, options)?;
                
                let mut buffer = Vec::new();
                file.read_to_end(&mut buffer)?;
                output_archive.write_all(&buffer)?;
            }
        }
        
        output_archive.finish()?;
        info!("Successfully created: {}", output_path);
        
        Ok(())
    }

    /// Füllt das Dokument, speichert zunächst als .odt, konvertiert per LibreOffice (soffice) nach PDF und löscht die .odt
    pub fn fill_and_save_pdf(
        &self,
        output_pdf_path: &str,
        replacements: &HashMap<String, String>,
    ) -> Result<()> {
        let pdf_path = Path::new(output_pdf_path);
        let odt_path = pdf_path.with_extension("odt");

        // 1) Erzeuge .odt
        self.fill_and_save(odt_path.to_str().unwrap(), replacements)?;

        // 2) Prüfe ob soffice verfügbar ist
        if which("soffice").is_err() {
            return Err(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                "LibreOffice (soffice) nicht im PATH gefunden. Bitte installiere LibreOffice oder sorge dafür, dass 'soffice' im PATH liegt."
            ).into());
        }

        // 3) Konvertiere .odt -> .pdf via soffice (LibreOffice)
        let outdir = pdf_path.parent().unwrap_or_else(|| Path::new("."));
        let status = Command::new("soffice")
            .arg("--headless")
            .arg("--convert-to")
            .arg("pdf")
            .arg("--outdir")
            .arg(outdir)
            .arg(&odt_path)
            .status()?;

        if !status.success() {
            return Err(std::io::Error::new(
                std::io::ErrorKind::Other,
                format!("LibreOffice-Konvertierung schlug fehl (exit: {:?}).", status.code())
            ).into());
        }

        // LibreOffice schreibt <basename>.pdf in outdir
        let generated_pdf = outdir.join(odt_path.file_stem().unwrap()).with_extension("pdf");
        if generated_pdf != pdf_path {
            fs::rename(&generated_pdf, &pdf_path)?;
        }

        // 3.5) WORKAROUND Prüfe Seitenanzahl und entferne ggf. Seite 2
        let config = PdfConfig::default();
        Self::ensure_single_page(&pdf_path, &config)?;

        // 4) entferne temporäre .odt
        if odt_path.exists() {
            fs::remove_file(&odt_path)?;
        }

        info!("Successfully created PDF: {}", output_pdf_path);
        Ok(())
    }
    
    /// Stellt sicher, dass das PDF nur eine Seite hat.
    /// Entfernt automatisch eine leere zweite Seite (typischerweise durch Fußzeilen-Overflow).
    fn ensure_single_page(pdf_path: &Path, config: &PdfConfig) -> Result<()> {
        use lopdf::Document;

        let mut doc = Document::load(pdf_path).map_err(|e| {
            std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                format!("Konnte PDF nicht laden: {}", e)
            )
        })?;

        let page_count = doc.get_pages().len();

        match page_count {
            1 => {
                // Perfekt, nichts zu tun
                info!("✓ PDF hat 1 Seite");
                Ok(())
            }
            2 => {
                if config.remove_empty_second_page {
                    // Automatisch entfernen
                    warn!("PDF hat 2 Seiten - entferne automatisch leere Seite 2");

                    // Seite 2 entfernen
                    Self::remove_page(&mut doc, 2)?;

                    doc.save(pdf_path).map_err(|e| {
                        std::io::Error::new(
                            std::io::ErrorKind::Other,
                            format!("Konnte bereinigtes PDF nicht speichern: {}", e)
                        )
                    })?;

                    info!("✓ Leere Seite 2 entfernt");
                    Ok(())
                } else {
                    // Nur warnen
                    warn!("⚠️  PDF hat 2 Seiten (automatisches Entfernen ist deaktiviert)");
                    Ok(())
                }
            }
            n => {
                if n > config.max_pages {
                    // Fehler werfen
                    Err(std::io::Error::new(
                        std::io::ErrorKind::InvalidData,
                        format!(
                            "PDF hat {} Seiten! Bitte prüfe die Vorlage - Zertifikate sollten einseitig sein.",
                            n
                        )
                    ).into())
                } else {
                    // Nur warnen
                    warn!("⚠️  PDF hat {} Seiten", n);
                    Ok(())
                }
            }
        }
    }

    /// Entfernt eine einzelne Seite aus dem PDF-Dokument
    fn remove_page(doc: &mut lopdf::Document, page_number: u32) -> Result<()> {
        use lopdf::Object;

        // Hole alle Seiten
        let pages = doc.get_pages();
        let page_ids: Vec<_> = pages.into_iter().collect();

        if page_number as usize > page_ids.len() || page_number == 0 {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                format!("Ungültige Seitennummer: {}", page_number)
            ).into());
        }

        // Seiten-ID der zu entfernenden Seite (page_ids enthält (page_num, ObjectId))
        let (_page_num, page_id) = page_ids[(page_number - 1) as usize];

        // Entferne die Seite aus dem Dokument
        doc.delete_object(page_id);

        // Aktualisiere das Pages-Objekt
        if let Ok(pages_ref) = doc.catalog().and_then(|cat| cat.get(b"Pages")) {
            if let Ok(pages_id) = pages_ref.as_reference() {
                if let Ok(pages_dict) = doc.get_object_mut(pages_id).and_then(|obj| obj.as_dict_mut()) {
                    // Kids-Array aktualisieren
                    if let Ok(kids) = pages_dict.get_mut(b"Kids").and_then(|obj| obj.as_array_mut()) {
                        kids.retain(|kid| {
                            if let Ok(kid_ref) = kid.as_reference() {
                                kid_ref != page_id
                            } else {
                                true
                            }
                        });

                        // Count aktualisieren
                        let new_count = kids.len() as i64;
                        pages_dict.set("Count", Object::Integer(new_count));
                    }
                }
            }
        }

        Ok(())
    }
    
    /// Batch-Verarbeitung: Mehrere Dokumente aus einer Liste erstellen (ODT)
    pub fn batch_fill(
        &self,
        output_dir: &str,
        batch_data: Vec<(String, HashMap<String, String>)>,
    ) -> Result<Vec<String>> {
        std::fs::create_dir_all(output_dir)?;
        
        let mut created_files = Vec::new();
        
        for (filename, replacements) in batch_data {
            let output_path = format!("{}/{}", output_dir, filename);
            self.fill_and_save(&output_path, &replacements)?;
            created_files.push(output_path);
        }
        
        Ok(created_files)
    }

    /// Batch-Verarbeitung: Mehrere Dokumente direkt als PDF erstellen
    pub fn batch_fill_pdf(
        &self,
        output_dir: &str,
        batch_data: Vec<(String, HashMap<String, String>)>,
    ) -> Result<Vec<String>> {
        std::fs::create_dir_all(output_dir)?;
        
        let mut created_files = Vec::new();
        
        for (filename, replacements) in batch_data {
            let output_path = Path::new(output_dir).join(filename);
            // Stelle sicher, dass die Dateiendung .pdf ist
            let pdf_path = if output_path.extension().is_some() {
                output_path.with_extension("pdf")
            } else {
                output_path.with_extension("pdf")
            };
            let pdf_str = pdf_path.to_str().unwrap().to_string();
            self.fill_and_save_pdf(&pdf_str, &replacements)?;
            created_files.push(pdf_str);
        }
        
        Ok(created_files)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_clean_split_placeholders() {
        let input = r#"text <text:span>{{</text:span><text:span>NAME</text:span><text:span>}}</text:span> more"#;
        let expected = "text {{NAME}} more";
        let result = OdfDocument::clean_split_placeholders(input);
        assert_eq!(result, expected);
    }

    #[test]
    fn test_clean_complex_split() {
        let input = r#"von {{</text:span><text:span text:style-name="T2">INSTRUCTOR</text:span><text:span>}}"#;
        let expected = "von {{INSTRUCTOR}}";
        let result = OdfDocument::clean_split_placeholders(input);
        assert_eq!(result, expected);
    }
}
