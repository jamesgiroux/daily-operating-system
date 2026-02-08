# ADR-0050: Universal File Extraction for Inbox Pipeline

**Date:** 2026-02-08
**Status:** Accepted

## Context

The inbox processing pipeline only handles `.md` files. Binary files (PDF, DOCX, XLSX, PPTX, HTML, RTF) can be dropped into the inbox via the drop zone, but `process_file()` calls `read_to_string()` which fails on binary content. These files get stuck in the inbox forever. `get_inbox_file_content()` shows a "[Binary file]" placeholder with no extracted content.

This contradicts the AI-native philosophy: the archive is the product, markdown is the universal format, and every file should become AI-consumable. Users who receive meeting notes as PDFs, account data as spreadsheets, or proposals as DOCX files get no value from the inbox pipeline.

## Decision

Add format-aware text extraction to the inbox pipeline. When a non-markdown file enters processing:

1. **Detect** format by extension (PDF, DOCX, XLSX, PPTX, HTML, RTF, plaintext)
2. **Extract** text content using pure-Rust crates (no system dependencies)
3. **Classify** using extracted text (same classifier, same patterns)
4. **Route** original binary file to destination
5. **Write companion .md** alongside the original with extracted text + YAML frontmatter

### Companion .md Pattern

Every non-markdown file gets a companion `.md` file at its destination:

```
_archive/2026-02-08/quarterly-review.pdf        ← original binary
_archive/2026-02-08/quarterly-review.md          ← extracted text + metadata
```

The companion includes YAML frontmatter linking back to the source:

```yaml
---
source: quarterly-review.pdf
format: pdf
extracted: 2026-02-08T06:02:15Z
---
```

If AI enrichment runs, the frontmatter gains classification metadata:

```yaml
---
source: quarterly-review.pdf
format: pdf
extracted: 2026-02-08T06:02:15Z
classification: meeting_notes
account: Acme Corp
summary: Quarterly review covering Q4 results.
---
```

### Crate Selection

All crates are pure Rust with no system dependencies:

| Format | Crate | Approach |
|--------|-------|----------|
| PDF | `pdf-extract` | Direct text extraction, `catch_unwind` for panic safety |
| DOCX | `zip` + `quick-xml` | Manual: unzip, walk `<w:t>` tags in `word/document.xml` |
| XLSX | `calamine` | Sheet data → markdown tables |
| PPTX | `zip` + `quick-xml` | Manual: unzip, walk `<a:t>` tags in `ppt/slides/slideN.xml` |
| HTML | `html2text` | HTML → plain text at 80-column width |
| RTF | `rtf-parser` | Token-level parsing, extract PlainText tokens |
| Plaintext | `std::fs` | `read_to_string` with UTF-8 lossy fallback |

Manual DOCX/PPTX extraction avoids heavier crates and gives us exactly what we need (~50 lines each).

### Classifier Extension Stripping

The filename classifier now strips all known extensions (not just `.md`) before pattern matching. `acme-meeting-notes.pdf` is classified the same as `acme-meeting-notes.md`. A bug in `extract_account_from_filename` was also fixed where `split()` on a non-matching suffix returned the full string.

### Extraction Limits

Extracted text is truncated at 100KB. The enrichment prompt truncates further to 8000 chars, but companion .md files retain the full extraction up to the limit.

## Consequences

- Files dropped as PDF/DOCX/XLSX/PPTX/HTML/RTF are now fully processed, classified, routed, and enriched
- Every routed binary file has an AI-consumable companion .md alongside it
- The inbox preview shows extracted text instead of "[Binary file]" for supported formats
- Unsupported formats (images, video, etc.) return a clear error rather than a silent failure
- 6 new Cargo dependencies add to compile time and binary size, but all are pure Rust with no system deps
- The companion .md pattern means the archive contains two files per binary document — this is intentional (the binary is the artifact, the .md is the knowledge)

## Not In Scope

- **OCR for images** — too heavy, too unreliable
- **Format conversion** (DOCX → formatted markdown) — extraction is text-only, not layout-preserving
- **Workspace-wide extraction** (non-md files already in Accounts/, Projects/) — future phase
- **I29 structured document schemas** — stays in parking lot with original scope
