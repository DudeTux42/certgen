use crate::error::{CertgenError, Result};
use log::info;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use tempfile::tempdir;
use walkdir::WalkDir;
use which::which;

/// Repräsentiert ein LaTeX-Template-Dokument
pub struct LatexDocument {
    path: String,
}

impl LatexDocument {
    /// Öffnet ein LaTeX-Template
    pub fn open(path: &str) -> Result<Self> {
        if !Path::new(path).exists() {
            return Err(CertgenError::TemplateNotFound(path.to_string()));
        }

        Ok(Self {
            path: path.to_string(),
        })
    }

    /// Escaped Sonderzeichen für LaTeX-Textkontext
    fn escape_latex(text: &str) -> String {
        let mut out = String::with_capacity(text.len() + 16);

        for ch in text.chars() {
            match ch {
                '\\' => out.push_str(r"\textbackslash{}"),
                '{' => out.push_str(r"\{"),
                '}' => out.push_str(r"\}"),
                '$' => out.push_str(r"\$"),
                '&' => out.push_str(r"\&"),
                '#' => out.push_str(r"\#"),
                '%' => out.push_str(r"\%"),
                '_' => out.push_str(r"\_"),
                '~' => out.push_str(r"\textasciitilde{}"),
                '^' => out.push_str(r"\textasciicircum{}"),
                '\n' => out.push_str(r"\\ "),
                _ => out.push(ch),
            }
        }

        out
    }

    /// Ersetzt {{KEY}}-Platzhalter im Template
    fn render_template(content: &str, replacements: &HashMap<String, String>) -> String {
        let mut result = content.to_string();
        for (key, value) in replacements {
            let token = format!("{{{{{}}}}}", key);
            let escaped_value = Self::escape_latex(value);
            result = result.replace(&token, &escaped_value);
        }
        result
    }

    /// Kopiert alle Dateien aus template_dir rekursiv in dst_dir.
    fn copy_template_assets(template_dir: &Path, dst_dir: &Path) -> Result<()> {
        for entry in WalkDir::new(template_dir) {
            let entry = entry.map_err(|e| {
                std::io::Error::new(std::io::ErrorKind::Other, format!("WalkDir error: {}", e))
            })?;
            let path = entry.path();

            let rel = path.strip_prefix(template_dir).map_err(|e| {
                std::io::Error::new(std::io::ErrorKind::Other, format!("strip_prefix error: {}", e))
            })?;
            let target = dst_dir.join(rel);

            if entry.file_type().is_dir() {
                fs::create_dir_all(&target)?;
            } else if entry.file_type().is_file() {
                if let Some(parent) = target.parent() {
                    fs::create_dir_all(parent)?;
                }
                fs::copy(path, &target)?;
            }
        }
        Ok(())
    }

    /// Rendert das Template und kompiliert direkt nach PDF via xelatex
    pub fn fill_and_save_pdf(
        &self,
        output_pdf_path: &str,
        replacements: &HashMap<String, String>,
    ) -> Result<()> {
        // Laufzeit-Check xelatex
        if which("xelatex").is_err() {
            return Err(CertgenError::LatexEngineNotFound(
                "xelatex".to_string(),
            ));
        }

        let template_path = PathBuf::from(&self.path);
        let template_dir = template_path
            .parent()
            .ok_or_else(|| CertgenError::InvalidTemplate)?;

        let template_content = fs::read_to_string(&template_path)?;

        let rendered = Self::render_template(&template_content, replacements);

        let tmp = tempdir()?;
        let workdir = tmp.path();

        // Assets (z.B. Hintergrund-PDF) kopieren
        Self::copy_template_assets(template_dir, workdir)?;

        // Gerendertes Hauptdokument immer als document.tex
        let tex_path = workdir.join("document.tex");
        fs::write(&tex_path, rendered)?;

        // 2x laufen lassen für stabile Layout-Elemente/Refs
        for _ in 0..2 {
            let output = Command::new("xelatex")
                .arg("-interaction=nonstopmode")
                .arg("-halt-on-error")
                .arg("document.tex")
                .current_dir(workdir)
                .output()?;

            if !output.status.success() {
                let stdout = String::from_utf8_lossy(&output.stdout).to_string();
                let stderr = String::from_utf8_lossy(&output.stderr).to_string();
                return Err(CertgenError::LatexCompileFailed {
                    engine: "xelatex".to_string(),
                    stdout,
                    stderr,
                });
            }
        }

        let generated_pdf = workdir.join("document.pdf");
        if !generated_pdf.exists() {
            return Err(CertgenError::PdfGenerationFailed(
                output_pdf_path.to_string(),
            ));
        }

        let out_path = Path::new(output_pdf_path);
        if let Some(parent) = out_path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::copy(&generated_pdf, out_path)?;

        info!("Successfully created PDF via LaTeX: {}", output_pdf_path);
        Ok(())
    }

    /// Batch-Verarbeitung: Mehrere PDFs direkt aus LaTeX erzeugen
    pub fn batch_fill_pdf(
        &self,
        output_dir: &str,
        batch_data: Vec<(String, HashMap<String, String>)>,
    ) -> Result<Vec<String>> {
        fs::create_dir_all(output_dir)?;

        let mut created_files = Vec::new();

        for (filename, replacements) in batch_data {
            let output_path = Path::new(output_dir).join(filename);
            let pdf_path = output_path.with_extension("pdf");
            let pdf_str = pdf_path.to_string_lossy().to_string();

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
    fn test_escape_latex() {
        let s = r#"A&B_%$#{}\~^"#;
        let e = LatexDocument::escape_latex(s);
        assert!(e.contains(r"\&"));
        assert!(e.contains(r"\_"));
        assert!(e.contains(r"\%"));
        assert!(e.contains(r"\$"));
        assert!(e.contains(r"\#"));
        assert!(e.contains(r"\{"));
        assert!(e.contains(r"\}"));
    }

    #[test]
    fn test_render_template() {
        let tpl = "Hallo {{NAME}}, Kurs {{TITLE}}";
        let mut map = HashMap::new();
        map.insert("NAME".to_string(), "Max".to_string());
        map.insert("TITLE".to_string(), "Linux Basics".to_string());

        let out = LatexDocument::render_template(tpl, &map);
        assert_eq!(out, "Hallo Max, Kurs Linux Basics");
    }
}
