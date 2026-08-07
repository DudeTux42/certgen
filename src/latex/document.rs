use crate::error::{CertgenError, Result};
use log::{info, warn};
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

    /// Baut Agenda-Layout mit dynamischer Skalierung gegen Überlauf
    fn build_agenda_layout(items_raw: &str) -> (String, String, String) {
        const ONE_COL_MAX: usize = 10;
        const MAX_PHYSICAL_LINES: usize = 11; 

        let items: Vec<String> = items_raw
            .lines()
            .map(|l| l.trim().trim_start_matches('·').trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();

        if items.is_empty() {
            return (
                String::from(r"\fontsize{14pt}{16pt}\selectfont\RaggedRight\begin{itemize}[leftmargin=*,nosep]\n\item Kursinhalt\n\end{itemize}"),
                String::new(),
                String::new(),
            );
        }

        // Hilfsfunktion zur Schätzung der echten Zeilenanzahl
        let estimate_lines = |slice: &[String], col_width_chars: usize| -> usize {
            let mut total_lines = 0;
            for item in slice {
                let wrapped = (item.len() as f64 / col_width_chars as f64).ceil() as usize;
                total_lines += std::cmp::max(1, wrapped);
            }
            total_lines
        };

        let (is_single, left_slice, right_slice) = if items.len() <= ONE_COL_MAX {
            (true, items, Vec::new())
        } else {
            let mid = items.len().div_ceil(2);
            let left = items[..mid].to_vec();
            let right = items[mid..].to_vec();
            (false, left, right)
        };

        let target_slice = if is_single { 
            &left_slice 
        } else if left_slice.len() > right_slice.len() { 
            &left_slice 
        } else { 
            &right_slice 
        };

        let estimated_lines = estimate_lines(target_slice, if is_single { 45 } else { 35 });

        let (font_size, line_skip, item_sep) = if estimated_lines > 12 {
            ("9pt", "10.5pt", "0.5pt") // Ultimativer Kompaktmodus
        } else if estimated_lines > MAX_PHYSICAL_LINES {
            ("10pt", "11.5pt", "1.5pt") // Sehr kompakt
        } else if estimated_lines > 8 {
            ("11.5pt", "13pt", "2.5pt")  // Normal kompakt
        } else {
            ("13pt", "15pt", "4pt")      // Großzügig
        };

        let to_lines = |slice: &[String], max_width: Option<&str>| -> String {
            if slice.is_empty() { return String::new(); }

            let mut item_block = String::new();

            if let Some(width) = max_width {
                item_block.push_str(&format!("\\begin{{varwidth}}{{{}}}\n", width));
            }

            // \RaggedRight direkt HIER einfügen, damit der Listeninhalt nie im Blocksatz landet!
            item_block.push_str(&format!(
                "\\fontsize{{{}}}{{{}}}\\selectfont\\RaggedRight\\begin{{itemize}}[leftmargin=*,topsep=0pt,parsep=0pt,itemsep={}]\n", 
                font_size, line_skip, item_sep
            ));

            for i in slice {
                item_block.push_str(&format!("  \\item {}\n", i));
            }
            item_block.push_str("\\end{itemize}");

            if max_width.is_some() {
                item_block.push_str("\n\\end{varwidth}");
            }

            item_block
        };

        if is_single {
            // Einspaltig: Nutzt varwidth mit einer maximalen Breite von 8.8cm für die Zentrierung
            (to_lines(&left_slice, Some("8.8cm")), String::new(), String::new())
        } else {
            // Zweispaltig: KEIN varwidth (None), damit die feste Spaltenbreite im Template greift
            (String::new(), to_lines(&left_slice, None), to_lines(&right_slice, None))
        }
    }

    fn render_template(content: &str, replacements: &HashMap<String, String>) -> String {
        let mut result = content.to_string();

        if let Some(items_raw) = replacements.get("AGENDA_ITEMS") {
            let (single, left, right) = Self::build_agenda_layout(items_raw);
            result = result.replace("{{AGENDA_SINGLE}}", &single);
            result = result.replace("{{AGENDA_LEFT}}", &left);
            result = result.replace("{{AGENDA_RIGHT}}", &right);
        } else {
            result = result.replace("{{AGENDA_SINGLE}}", r"\textbullet\ Kursinhalt");
            result = result.replace("{{AGENDA_LEFT}}", "");
            result = result.replace("{{AGENDA_RIGHT}}", "");
        }

        for (key, value) in replacements {
            if key == "AGENDA_ITEMS"
                || key == "AGENDA_SINGLE"
                || key == "AGENDA_LEFT"
                || key == "AGENDA_RIGHT"
            {
                continue;
            }

            let token = format!("{{{{{}}}}}", key);
            result = result.replace(&token, &Self::escape_latex(value));
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
        if which("xelatex").is_err() {
            return Err(CertgenError::LatexEngineNotFound("xelatex".to_string()));
        }

        let template_path = PathBuf::from(&self.path);
        let template_dir = template_path
            .parent()
            .filter(|p| !p.as_os_str().is_empty())
            .unwrap_or_else(|| Path::new("."));

        let template_content = fs::read_to_string(&template_path)?;
        let rendered = Self::render_template(&template_content, replacements);

        let tmp = tempdir()?;
        let workdir = tmp.path();

        if let Err(e) = Self::copy_template_assets(template_dir, workdir) {
            warn!("Asset copy failed: {}", e);
            return Err(e);
        }

        let tex_path = workdir.join("document.tex");
        fs::write(&tex_path, rendered)?;

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
}
