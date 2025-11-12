use crate::error::{CertgenError, Result};
use crate::odf::replacer::PlaceholderReplacer;
use std::collections::HashMap;
use std::fs::File;
use std::io::{Read, Write};
use zip::{ZipArchive, ZipWriter, write::FileOptions, CompressionMethod};
use log::{debug, info};

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

    /// Batch-Verarbeitung: Mehrere Dokumente aus einer Liste erstellen
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
