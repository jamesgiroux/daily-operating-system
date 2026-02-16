//! Text extraction from binary document formats.
//!
//! Converts PDF, DOCX, XLSX, PPTX, HTML, RTF, and plaintext files into
//! extractable text content for classification, enrichment, and companion .md
//! generation. Markdown files pass through unchanged.

use std::path::Path;

/// Maximum extracted text length (100KB). The enrichment prompt truncates
/// further to 8000 chars, but companion .md files get the full extraction.
const MAX_EXTRACT_BYTES: usize = 100_000;

/// Supported document formats, detected by file extension.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SupportedFormat {
    /// .md — pass through, no extraction needed
    Markdown,
    /// .txt, .csv, .tsv, .json, .yaml, .yml, .log — read_to_string
    PlainText,
    /// .pdf
    Pdf,
    /// .docx
    Docx,
    /// .xlsx, .xls, .xlsm, .ods
    Xlsx,
    /// .pptx
    Pptx,
    /// .html, .htm
    Html,
    /// .rtf
    Rtf,
    /// Everything else (images, video, etc.)
    Unsupported,
}

/// Errors that can occur during text extraction.
#[derive(Debug)]
pub enum ExtractError {
    /// File format is not supported for extraction.
    UnsupportedFormat(String),
    /// IO error reading the file.
    Io(std::io::Error),
    /// Format-specific extraction failure.
    ExtractionFailed(String),
}

impl std::fmt::Display for ExtractError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::UnsupportedFormat(ext) => write!(f, "Unsupported format: .{}", ext),
            Self::Io(e) => write!(f, "IO error: {}", e),
            Self::ExtractionFailed(msg) => write!(f, "Extraction failed: {}", msg),
        }
    }
}

impl From<std::io::Error> for ExtractError {
    fn from(e: std::io::Error) -> Self {
        Self::Io(e)
    }
}

/// Detect the document format from file extension.
pub fn detect_format(path: &Path) -> SupportedFormat {
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();

    match ext.as_str() {
        "md" | "markdown" => SupportedFormat::Markdown,
        "txt" | "csv" | "tsv" | "json" | "yaml" | "yml" | "log" | "xml" | "toml" => {
            SupportedFormat::PlainText
        }
        "pdf" => SupportedFormat::Pdf,
        "docx" => SupportedFormat::Docx,
        "xlsx" | "xls" | "xlsm" | "ods" => SupportedFormat::Xlsx,
        "pptx" => SupportedFormat::Pptx,
        "html" | "htm" => SupportedFormat::Html,
        "rtf" => SupportedFormat::Rtf,
        _ => SupportedFormat::Unsupported,
    }
}

/// Whether a file can have text extracted from it.
pub fn is_extractable(path: &Path) -> bool {
    !matches!(detect_format(path), SupportedFormat::Unsupported)
}

/// All file extensions that the extraction system handles (for stripping in classifier).
pub const KNOWN_EXTENSIONS: &[&str] = &[
    ".md", ".txt", ".csv", ".tsv", ".json", ".yaml", ".yml", ".log", ".xml", ".toml", ".pdf",
    ".docx", ".xlsx", ".xls", ".xlsm", ".ods", ".pptx", ".html", ".htm", ".rtf",
];

/// Extract text content from a file.
///
/// Returns the extracted text, truncated to [`MAX_EXTRACT_BYTES`].
/// For Markdown files, this simply reads the file as-is.
/// For binary formats, uses format-specific extractors.
pub fn extract_text(path: &Path) -> Result<String, ExtractError> {
    let format = detect_format(path);

    let raw = match format {
        SupportedFormat::Markdown | SupportedFormat::PlainText => extract_plaintext(path)?,
        SupportedFormat::Pdf => extract_pdf(path)?,
        SupportedFormat::Docx => extract_docx(path)?,
        SupportedFormat::Xlsx => extract_xlsx(path)?,
        SupportedFormat::Pptx => extract_pptx(path)?,
        SupportedFormat::Html => extract_html(path)?,
        SupportedFormat::Rtf => extract_rtf(path)?,
        SupportedFormat::Unsupported => {
            let ext = path
                .extension()
                .and_then(|e| e.to_str())
                .unwrap_or("unknown")
                .to_string();
            return Err(ExtractError::UnsupportedFormat(ext));
        }
    };

    Ok(truncate_text(&raw, MAX_EXTRACT_BYTES))
}

// ---------------------------------------------------------------------------
// Format-specific extractors
// ---------------------------------------------------------------------------

fn extract_plaintext(path: &Path) -> Result<String, ExtractError> {
    // Try UTF-8, fall back to lossy conversion
    match std::fs::read_to_string(path) {
        Ok(s) => Ok(s),
        Err(_) => {
            let bytes = std::fs::read(path)?;
            Ok(String::from_utf8_lossy(&bytes).into_owned())
        }
    }
}

fn extract_pdf(path: &Path) -> Result<String, ExtractError> {
    // pdf-extract can panic on malformed PDFs — wrap in catch_unwind
    let path_buf = path.to_path_buf();
    let result = std::panic::catch_unwind(move || pdf_extract::extract_text(&path_buf));

    match result {
        Ok(Ok(text)) => Ok(text),
        Ok(Err(e)) => Err(ExtractError::ExtractionFailed(format!("PDF: {}", e))),
        Err(_) => Err(ExtractError::ExtractionFailed(
            "PDF extraction panicked (malformed file)".to_string(),
        )),
    }
}

fn extract_docx(path: &Path) -> Result<String, ExtractError> {
    // DOCX = ZIP archive containing word/document.xml
    // Walk <w:t> tags to extract text runs.
    let file = std::fs::File::open(path)?;
    let mut archive = zip::ZipArchive::new(file)
        .map_err(|e| ExtractError::ExtractionFailed(format!("DOCX zip: {}", e)))?;

    let doc = archive
        .by_name("word/document.xml")
        .map_err(|e| ExtractError::ExtractionFailed(format!("DOCX missing document.xml: {}", e)))?;

    let mut reader = quick_xml::Reader::from_reader(std::io::BufReader::new(doc));
    let mut buf = Vec::new();
    let mut text = String::new();
    let mut in_text_tag = false;
    let mut in_paragraph = false;

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(quick_xml::events::Event::Start(ref e))
            | Ok(quick_xml::events::Event::Empty(ref e)) => {
                let local = e.local_name();
                if local.as_ref() == b"t" {
                    in_text_tag = true;
                } else if local.as_ref() == b"p" {
                    if in_paragraph && !text.ends_with('\n') {
                        text.push('\n');
                    }
                    in_paragraph = true;
                }
            }
            Ok(quick_xml::events::Event::End(ref e)) => {
                if e.local_name().as_ref() == b"t" {
                    in_text_tag = false;
                } else if e.local_name().as_ref() == b"p" {
                    in_paragraph = false;
                }
            }
            Ok(quick_xml::events::Event::Text(ref e)) => {
                if in_text_tag {
                    if let Ok(s) = e.unescape() {
                        text.push_str(&s);
                    }
                }
            }
            Ok(quick_xml::events::Event::Eof) => break,
            Err(e) => {
                return Err(ExtractError::ExtractionFailed(format!("DOCX XML: {}", e)));
            }
            _ => {}
        }
        buf.clear();
    }

    Ok(text)
}

fn extract_xlsx(path: &Path) -> Result<String, ExtractError> {
    use calamine::{open_workbook_auto, Reader};

    let mut workbook = open_workbook_auto(path)
        .map_err(|e| ExtractError::ExtractionFailed(format!("XLSX: {}", e)))?;

    let mut output = String::new();

    for sheet_name in workbook.sheet_names().to_vec() {
        if let Ok(range) = workbook.worksheet_range(&sheet_name) {
            if !output.is_empty() {
                output.push_str("\n\n");
            }
            output.push_str(&format!("## {}\n\n", sheet_name));

            // Render as markdown table
            let mut rows = range.rows();
            if let Some(header) = rows.next() {
                let header_cells: Vec<String> = header.iter().map(cell_to_string).collect();
                output.push_str("| ");
                output.push_str(&header_cells.join(" | "));
                output.push_str(" |\n");
                output.push_str("| ");
                output.push_str(
                    &header_cells
                        .iter()
                        .map(|_| "---")
                        .collect::<Vec<_>>()
                        .join(" | "),
                );
                output.push_str(" |\n");

                for row in rows {
                    let cells: Vec<String> = row.iter().map(cell_to_string).collect();
                    output.push_str("| ");
                    output.push_str(&cells.join(" | "));
                    output.push_str(" |\n");
                }
            }
        }
    }

    Ok(output)
}

fn cell_to_string(cell: &calamine::Data) -> String {
    use calamine::Data;
    match cell {
        Data::Empty => String::new(),
        Data::String(s) => s.clone(),
        Data::Int(n) => n.to_string(),
        Data::Float(f) => format!("{}", f),
        Data::Bool(b) => b.to_string(),
        Data::Error(e) => format!("#ERR({:?})", e),
        Data::DateTime(dt) => format!("{}", dt),
        Data::DateTimeIso(s) => s.clone(),
        Data::DurationIso(s) => s.clone(),
    }
}

fn extract_pptx(path: &Path) -> Result<String, ExtractError> {
    // PPTX = ZIP archive containing ppt/slides/slideN.xml
    // Walk <a:t> tags to extract text runs.
    let file = std::fs::File::open(path)?;
    let mut archive = zip::ZipArchive::new(file)
        .map_err(|e| ExtractError::ExtractionFailed(format!("PPTX zip: {}", e)))?;

    let mut text = String::new();

    // Collect slide file names and sort them
    let mut slide_names: Vec<String> = (0..archive.len())
        .filter_map(|i| {
            let name = archive.by_index(i).ok()?.name().to_string();
            if name.starts_with("ppt/slides/slide") && name.ends_with(".xml") {
                Some(name)
            } else {
                None
            }
        })
        .collect();
    slide_names.sort();

    for (idx, slide_name) in slide_names.iter().enumerate() {
        let slide = archive.by_name(slide_name).map_err(|e| {
            ExtractError::ExtractionFailed(format!("PPTX slide {}: {}", slide_name, e))
        })?;

        if idx > 0 {
            text.push_str("\n\n");
        }
        text.push_str(&format!("--- Slide {} ---\n", idx + 1));

        let mut reader = quick_xml::Reader::from_reader(std::io::BufReader::new(slide));
        let mut buf = Vec::new();
        let mut in_text_tag = false;

        loop {
            match reader.read_event_into(&mut buf) {
                Ok(quick_xml::events::Event::Start(ref e)) => {
                    if e.local_name().as_ref() == b"t" {
                        in_text_tag = true;
                    }
                }
                Ok(quick_xml::events::Event::End(ref e)) => {
                    if e.local_name().as_ref() == b"t" {
                        in_text_tag = false;
                    }
                }
                Ok(quick_xml::events::Event::Text(ref e)) => {
                    if in_text_tag {
                        if let Ok(s) = e.unescape() {
                            text.push_str(&s);
                            text.push(' ');
                        }
                    }
                }
                Ok(quick_xml::events::Event::Eof) => break,
                Err(_) => break,
                _ => {}
            }
            buf.clear();
        }
    }

    Ok(text)
}

fn extract_html(path: &Path) -> Result<String, ExtractError> {
    let html = std::fs::read_to_string(path)?;
    let text = html2text::from_read(html.as_bytes(), 80)
        .map_err(|e| ExtractError::ExtractionFailed(format!("HTML: {}", e)))?;
    Ok(text)
}

fn extract_rtf(path: &Path) -> Result<String, ExtractError> {
    let rtf_bytes = std::fs::read(path)?;
    let rtf_str = String::from_utf8_lossy(&rtf_bytes);

    // rtf-parser provides token-level parsing. We extract text tokens.
    use rtf_parser::{lexer::Lexer, tokens::Token};

    let tokens = Lexer::scan(&rtf_str)
        .map_err(|e| ExtractError::ExtractionFailed(format!("RTF: {:?}", e)))?;

    let mut text = String::new();
    for token in &tokens {
        if let Token::PlainText(s) = token {
            text.push_str(s);
        } else if let Token::CRLF = token {
            text.push('\n');
        }
    }

    Ok(text)
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Truncate text at a safe UTF-8 boundary.
fn truncate_text(text: &str, max_bytes: usize) -> String {
    if text.len() <= max_bytes {
        return text.to_string();
    }

    let mut end = max_bytes;
    while end > 0 && !text.is_char_boundary(end) {
        end -= 1;
    }
    let mut result = text[..end].to_string();
    result.push_str("\n\n[... content truncated at 100KB ...]");
    result
}

/// Generate the companion .md content with YAML frontmatter.
pub fn build_companion_md(
    source_filename: &str,
    format: SupportedFormat,
    extracted_text: &str,
) -> String {
    let now = chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true);
    let format_label = match format {
        SupportedFormat::Pdf => "pdf",
        SupportedFormat::Docx => "docx",
        SupportedFormat::Xlsx => "xlsx",
        SupportedFormat::Pptx => "pptx",
        SupportedFormat::Html => "html",
        SupportedFormat::Rtf => "rtf",
        SupportedFormat::PlainText => "plaintext",
        SupportedFormat::Markdown => "markdown",
        SupportedFormat::Unsupported => "unknown",
    };

    format!(
        "---\nsource: {}\nformat: {}\nextracted: {}\n---\n\n{}",
        source_filename, format_label, now, extracted_text
    )
}

/// Generate the companion .md content with enrichment metadata in frontmatter.
pub fn build_enriched_companion_md(
    source_filename: &str,
    format: SupportedFormat,
    extracted_text: &str,
    classification: &str,
    account: Option<&str>,
    summary: &str,
) -> String {
    let now = chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true);
    let format_label = match format {
        SupportedFormat::Pdf => "pdf",
        SupportedFormat::Docx => "docx",
        SupportedFormat::Xlsx => "xlsx",
        SupportedFormat::Pptx => "pptx",
        SupportedFormat::Html => "html",
        SupportedFormat::Rtf => "rtf",
        SupportedFormat::PlainText => "plaintext",
        SupportedFormat::Markdown => "markdown",
        SupportedFormat::Unsupported => "unknown",
    };

    let account_line = match account {
        Some(a) if !a.is_empty() => format!("\naccount: {}", a),
        _ => String::new(),
    };

    let summary_line = if !summary.is_empty() {
        format!("\nsummary: {}", summary)
    } else {
        String::new()
    };

    format!(
        "---\nsource: {}\nformat: {}\nextracted: {}\nclassification: {}{}{}\n---\n\n{}",
        source_filename,
        format_label,
        now,
        classification,
        account_line,
        summary_line,
        extracted_text
    )
}

/// Compute the companion .md path for a given file path.
/// e.g., /workspace/_archive/2026-02-08/report.pdf → /workspace/_archive/2026-02-08/report.md
pub fn companion_md_path(original_path: &Path) -> std::path::PathBuf {
    let stem = original_path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("file");
    let parent = original_path.parent().unwrap_or(Path::new("."));
    parent.join(format!("{}.md", stem))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_detect_format_markdown() {
        assert_eq!(
            detect_format(Path::new("file.md")),
            SupportedFormat::Markdown
        );
        assert_eq!(
            detect_format(Path::new("file.markdown")),
            SupportedFormat::Markdown
        );
    }

    #[test]
    fn test_detect_format_plaintext() {
        assert_eq!(
            detect_format(Path::new("file.txt")),
            SupportedFormat::PlainText
        );
        assert_eq!(
            detect_format(Path::new("data.csv")),
            SupportedFormat::PlainText
        );
        assert_eq!(
            detect_format(Path::new("data.tsv")),
            SupportedFormat::PlainText
        );
        assert_eq!(
            detect_format(Path::new("config.json")),
            SupportedFormat::PlainText
        );
        assert_eq!(
            detect_format(Path::new("config.yaml")),
            SupportedFormat::PlainText
        );
        assert_eq!(
            detect_format(Path::new("config.yml")),
            SupportedFormat::PlainText
        );
        assert_eq!(
            detect_format(Path::new("app.log")),
            SupportedFormat::PlainText
        );
    }

    #[test]
    fn test_detect_format_documents() {
        assert_eq!(detect_format(Path::new("report.pdf")), SupportedFormat::Pdf);
        assert_eq!(detect_format(Path::new("doc.docx")), SupportedFormat::Docx);
        assert_eq!(detect_format(Path::new("data.xlsx")), SupportedFormat::Xlsx);
        assert_eq!(detect_format(Path::new("data.xls")), SupportedFormat::Xlsx);
        assert_eq!(
            detect_format(Path::new("slides.pptx")),
            SupportedFormat::Pptx
        );
        assert_eq!(detect_format(Path::new("page.html")), SupportedFormat::Html);
        assert_eq!(detect_format(Path::new("page.htm")), SupportedFormat::Html);
        assert_eq!(detect_format(Path::new("doc.rtf")), SupportedFormat::Rtf);
    }

    #[test]
    fn test_detect_format_unsupported() {
        assert_eq!(
            detect_format(Path::new("image.png")),
            SupportedFormat::Unsupported
        );
        assert_eq!(
            detect_format(Path::new("video.mp4")),
            SupportedFormat::Unsupported
        );
        assert_eq!(
            detect_format(Path::new("archive.zip")),
            SupportedFormat::Unsupported
        );
        assert_eq!(
            detect_format(Path::new("no_extension")),
            SupportedFormat::Unsupported
        );
    }

    #[test]
    fn test_is_extractable() {
        assert!(is_extractable(Path::new("doc.pdf")));
        assert!(is_extractable(Path::new("file.md")));
        assert!(is_extractable(Path::new("data.csv")));
        assert!(!is_extractable(Path::new("image.png")));
        assert!(!is_extractable(Path::new("video.mp4")));
    }

    #[test]
    fn test_extract_plaintext() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("test.txt");
        std::fs::write(&path, "Hello, world!\nLine two.").unwrap();

        let text = extract_text(&path).unwrap();
        assert_eq!(text, "Hello, world!\nLine two.");
    }

    #[test]
    fn test_extract_unsupported() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("image.png");
        std::fs::write(&path, [0x89, 0x50, 0x4E, 0x47]).unwrap();

        let result = extract_text(&path);
        assert!(result.is_err());
        match result.unwrap_err() {
            ExtractError::UnsupportedFormat(ext) => assert_eq!(ext, "png"),
            other => panic!("Expected UnsupportedFormat, got: {}", other),
        }
    }

    #[test]
    fn test_extract_truncation() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("large.txt");
        // Write 150KB of text
        let large_content = "x".repeat(150_000);
        std::fs::write(&path, &large_content).unwrap();

        let text = extract_text(&path).unwrap();
        // Should be truncated to ~100KB + truncation notice
        assert!(text.len() < 150_000);
        assert!(text.contains("[... content truncated at 100KB ...]"));
    }

    #[test]
    fn test_extract_html_basic() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("page.html");
        std::fs::write(
            &path,
            "<html><body><h1>Title</h1><p>Content here</p></body></html>",
        )
        .unwrap();

        let text = extract_text(&path).unwrap();
        assert!(text.contains("Title"));
        assert!(text.contains("Content here"));
    }

    #[test]
    fn test_build_companion_md() {
        let md = build_companion_md("report.pdf", SupportedFormat::Pdf, "Extracted content here");
        assert!(md.starts_with("---\n"));
        assert!(md.contains("source: report.pdf"));
        assert!(md.contains("format: pdf"));
        assert!(md.contains("extracted:"));
        assert!(md.contains("Extracted content here"));
    }

    #[test]
    fn test_build_enriched_companion_md() {
        let md = build_enriched_companion_md(
            "notes.pdf",
            SupportedFormat::Pdf,
            "Content",
            "meeting_notes",
            Some("Acme Corp"),
            "Q4 review notes",
        );
        assert!(md.contains("classification: meeting_notes"));
        assert!(md.contains("account: Acme Corp"));
        assert!(md.contains("summary: Q4 review notes"));
    }

    #[test]
    fn test_companion_md_path() {
        let original = PathBuf::from("/workspace/_archive/2026-02-08/report.pdf");
        let companion = companion_md_path(&original);
        assert_eq!(
            companion,
            PathBuf::from("/workspace/_archive/2026-02-08/report.md")
        );
    }

    #[test]
    fn test_companion_md_path_no_extension() {
        let original = PathBuf::from("/workspace/_archive/2026-02-08/readme");
        let companion = companion_md_path(&original);
        assert_eq!(
            companion,
            PathBuf::from("/workspace/_archive/2026-02-08/readme.md")
        );
    }

    #[test]
    fn test_known_extensions_covers_formats() {
        // Verify all extensions in detect_format are also in KNOWN_EXTENSIONS
        let exts = [
            "md", "txt", "csv", "tsv", "json", "yaml", "yml", "log", "pdf", "docx", "xlsx", "xls",
            "xlsm", "ods", "pptx", "html", "htm", "rtf",
        ];
        for ext in &exts {
            let with_dot = format!(".{}", ext);
            assert!(
                KNOWN_EXTENSIONS.contains(&with_dot.as_str()),
                "Extension .{} not in KNOWN_EXTENSIONS",
                ext
            );
        }
    }
}
