use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Daten für das Zertifikat
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CertificateData {
    pub name: String,
    
    /// Hauptdatum (bei eintägigen Kursen) oder Enddatum (bei mehrtägigen)
    pub date: String,
    
    pub agenda: String,
    
    /// Startdatum (optional, nur bei mehrtägigen Kursen)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub date_from: Option<String>,
    
    /// Enddatum (optional, nur bei mehrtägigen Kursen)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub date_to: Option<String>,
    
    #[serde(flatten)]
    pub custom_fields: HashMap<String, String>,
}

impl CertificateData {
    pub fn new(name: String, date: String, agenda: String) -> Self {
        Self {
            name,
            date,
            agenda,
            date_from: None,
            date_to: None,
            custom_fields: HashMap::new(),
        }
    }

    /// Fügt ein benutzerdefiniertes Feld hinzu
    pub fn add_field(&mut self, key: String, value: String) {
        self.custom_fields.insert(key, value);
    }

    /// Generiert den intelligenten Datumstext
    fn get_intelligent_date_text(&self) -> String {
        match (&self.date_from, &self.date_to) {
            (Some(from), Some(to)) => {
                // Mehrtägiger Kurs: "vom ... bis ..."
                format!("von {} bis {}", from, to)
            }
            _ => {
                // Eintägiger Kurs: "am ..."
                format!("am {}", self.date)
            }
        }
    }

    /// Gibt alle Platzhalter mit ihren Werten zurück
    pub fn to_replacements(&self) -> HashMap<String, String> {
        let mut replacements = HashMap::new();
        
        // NAME
        replacements.insert("NAME".to_string(), self.name.clone());
        
        // VON_AN - der intelligente Datumstext
        let date_text = self.get_intelligent_date_text();
        replacements.insert("VON_AN".to_string(), date_text.clone());
        
        // DATE - auch als Alias für VON_AN
        replacements.insert("DATE".to_string(), date_text);
        
        // AGENDA
        replacements.insert("AGENDA".to_string(), self.agenda.clone());
        
        // Benutzerdefinierte Felder (z.B. TITLE)
        for (key, value) in &self.custom_fields {
            replacements.insert(key.clone(), value.clone());
        }
        
        replacements
    }

    /// Lädt Daten aus einer JSON-Datei
    pub fn from_json_file(path: &str) -> crate::error::Result<Self> {
        let content = std::fs::read_to_string(path)?;
        let data = serde_json::from_str(&content)?;
        Ok(data)
    }

    /// Lädt mehrere Datensätze aus einer JSON-Datei
    pub fn batch_from_json_file(path: &str) -> crate::error::Result<Vec<Self>> {
        let content = std::fs::read_to_string(path)?;
        let data = serde_json::from_str(&content)?;
        Ok(data)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_single_day_course() {
        let data = CertificateData::new(
            "Max".to_string(),
            "15.01.2024".to_string(),
            "Rust".to_string(),
        );
        
        let replacements = data.to_replacements();
        assert_eq!(replacements.get("VON_AN"), Some(&"am 15.01.2024".to_string()));
    }

    #[test]
    fn test_multi_day_course() {
        let mut data = CertificateData::new(
            "Max".to_string(),
            "15.01.2024".to_string(),
            "Rust".to_string(),
        );
        data.date_from = Some("10.01.2024".to_string());
        data.date_to = Some("15.01.2024".to_string());
        
        let replacements = data.to_replacements();
        assert_eq!(replacements.get("VON_AN"), Some(&"vom 10.01.2024 bis 15.01.2024".to_string()));
    }
}
