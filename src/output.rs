use std::{fs, path::Path};

use anyhow::{Context, Result};

pub fn save_results(path: &Path, hits: &[String]) -> Result<()> {
    let contents = match path.extension().and_then(|ext| ext.to_str()) {
        Some(ext) if ext.eq_ignore_ascii_case("csv") => render_csv(hits),
        _ => render_text(hits),
    };

    fs::write(path, contents)
        .with_context(|| format!("failed to write results to {}", path.display()))
}

fn render_text(hits: &[String]) -> String {
    if hits.is_empty() {
        return String::new();
    }

    let mut contents = hits.join("\n");
    contents.push('\n');
    contents
}

fn render_csv(hits: &[String]) -> String {
    let mut contents = String::from("name\n");
    for hit in hits {
        contents.push_str(hit);
        contents.push('\n');
    }
    contents
}

#[cfg(test)]
mod tests {
    use super::{render_csv, render_text};

    #[test]
    fn renders_plain_text_results() {
        let hits = vec!["eqk0".to_string(), "evFQ".to_string()];
        assert_eq!(render_text(&hits), "eqk0\nevFQ\n");
    }

    #[test]
    fn renders_csv_results() {
        let hits = vec!["eqk0".to_string(), "evFQ".to_string()];
        assert_eq!(render_csv(&hits), "name\neqk0\nevFQ\n");
    }
}
