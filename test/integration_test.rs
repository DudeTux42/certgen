use certgen::{CertificateData, OdfDocument};
use std::collections::HashMap;
use tempfile::tempdir;

#[test]
#[ignore] // Nur ausführen wenn Template vorhanden
fn test_fill_certificate() {
    let dir = tempdir().unwrap();
    let output = dir.path().join("test.odt");

    // Dieser Test benötigt eine echte template.odt Datei
    if std::path::Path::new("examples/vorlage.odt").exists() {
        let doc = O
