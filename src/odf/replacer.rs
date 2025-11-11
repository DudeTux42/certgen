use std::collections::HashMap;
use log::{warn, info};

pub struct PlaceholderReplacer {
    prefix: String,
    suffix: String,
}

impl PlaceholderReplacer {
    pub fn new() -> Self {
        Self {
            prefix: "{{".to_string(),
            suffix: "}}".to_string(),
        }
    }

    /// Escaped XML-Sonderzeichen und konvertiert Newlines zu XML line breaks
    fn escape_xml(text: &str) -> String {
        // Erst die normalen XML-Zeichen escapen
        let escaped = text
            .replace('&', "&amp;")
            .replace('<', "&lt;")
            .replace('>', "&gt;")
            .replace('"', "&quot;")
            .replace('\'', "&apos;");
        
        // Dann Newlines zu ODF line breaks konvertieren
        escaped.replace('\n', "<text:line-break/>")
    }

    pub fn replace_all(&self, content: &str, replacements: &HashMap<String, String>) -> String {
        let mut result = content.to_string();
        
        info!("Starting replacements. Total placeholders: {}", replacements.len());
        
        for (key, value) in replacements {
            let placeholder = format!("{}{}{}", self.prefix, key, self.suffix);
            let count = result.matches(&placeholder).count();
            
            if count > 0 {
                let escaped_value = Self::escape_xml(value);
                info!("✓ Replacing {} occurrences of '{}' with '{}'", count, placeholder, value);
                result = result.replace(&placeholder, &escaped_value);
            } else {
                warn!("✗ Placeholder '{}' not found in document", placeholder);
            }
        }
        
        result
    }
}

impl Default for PlaceholderReplacer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_xml_escaping() {
        assert_eq!(
            PlaceholderReplacer::escape_xml("1&1 Test"),
            "1&amp;1 Test"
        );
        assert_eq!(
            PlaceholderReplacer::escape_xml("<html>"),
            "&lt;html&gt;"
        );
    }

    #[test]
    fn test_newline_conversion() {
        assert_eq!(
            PlaceholderReplacer::escape_xml("Line 1\nLine 2"),
            "Line 1<text:line-break/>Line 2"
        );
    }

    #[test]
    fn test_combined_escaping() {
        assert_eq!(
            PlaceholderReplacer::escape_xml("1&1\nLine 2"),
            "1&amp;1<text:line-break/>Line 2"
        );
    }

    #[test]
    fn test_replace_with_special_chars() {
        let replacer = PlaceholderReplacer::new();
        let mut replacements = HashMap::new();
        replacements.insert("COMPANY".to_string(), "1&1 Internet".to_string());
        
        let content = "Firma: {{COMPANY}}";
        let result = replacer.replace_all(content, &replacements);
        
        assert_eq!(result, "Firma: 1&amp;1 Internet");
    }
}
