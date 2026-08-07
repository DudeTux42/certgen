use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Daten für das Zertifikat
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CertificateData {
    pub name: String,

    /// Hauptdatum (bei eintägigen Kursen) oder Enddatum (bei mehrtägigen)
    pub date: String,

    /// Legacy-Feld (rückwärtskompatibel), falls alte JSONs noch agenda als String liefern.
    #[serde(default)]
    pub agenda: String,

    /// Neue bevorzugte Form: einzelne Agenda-Punkte
    #[serde(default)]
    pub agenda_items: Vec<String>,

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
        // Rückwärtskompatibel: wenn agenda gesetzt wurde, als ein item speichern
        let agenda_items = if agenda.trim().is_empty() {
            Vec::new()
        } else {
            agenda
                .lines()
                .map(|l| l.trim().trim_start_matches('·').trim().to_string())
                .filter(|s| !s.is_empty())
                .collect()
        };

        Self {
            name,
            date,
            agenda,
            agenda_items,
            date_from: None,
            date_to: None,
            custom_fields: HashMap::new(),
        }
    }

    /// Fügt ein benutzerdefiniertes Feld hinzu
    pub fn add_field(&mut self, key: String, value: String) {
        self.custom_fields.insert(key, value);
    }

    /// Agenda robust als Liste zurückgeben (neu + fallback auf legacy)
    pub fn resolved_agenda_items(&self) -> Vec<String> {
        if !self.agenda_items.is_empty() {
            return self
                .agenda_items
                .iter()
                .map(|s| s.trim().trim_start_matches('·').trim().to_string())
                .filter(|s| !s.is_empty())
                .collect();
        }

        self.agenda
            .lines()
            .map(|l| l.trim().trim_start_matches('·').trim().to_string())
            .filter(|s| !s.is_empty())
            .collect()
    }

    /// Generiert den intelligenten Datumstext
    fn get_intelligent_date_text(&self) -> String {
        match (&self.date_from, &self.date_to) {
            (Some(from), Some(to)) => format!("von {} bis {}", from, to),
            _ => format!("am {}", self.date),
        }
    }

    /// Gibt alle Platzhalter mit ihren Werten zurück
    pub fn to_replacements(&self) -> HashMap<String, String> {
        let mut replacements = HashMap::new();

        replacements.insert("NAME".to_string(), self.name.clone());

        let date_text = self.get_intelligent_date_text();
        replacements.insert("VON_AN".to_string(), date_text.clone());
        replacements.insert("DATE".to_string(), date_text);

        // Legacy-String weiter befüllen (für alte Templates)
        replacements.insert("AGENDA".to_string(), self.agenda.clone());

        // Neue bevorzugte Rohdaten (durch '\n' getrennt) für Renderer
        let items = self.resolved_agenda_items();
        replacements.insert("AGENDA_ITEMS".to_string(), items.join("\n"));

        for (key, value) in &self.custom_fields {
            replacements.insert(key.clone(), value.clone());
        }

        replacements
    }

    pub fn from_json_file(path: &str) -> crate::error::Result<Self> {
        let content = std::fs::read_to_string(path)?;
        let data = serde_json::from_str(&content)?;
        Ok(data)
    }

    pub fn batch_from_json_file(path: &str) -> crate::error::Result<Vec<Self>> {
        let content = std::fs::read_to_string(path)?;
        let data = serde_json::from_str(&content)?;
        Ok(data)
    }
}
