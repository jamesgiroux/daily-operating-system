//! Minimal HTML walker for the no-bypass harness.
//!
//! Scope is intentionally narrow:
//! 1. Extract every text node and the value of the nearest `data-claim-id`
//!    ancestor (or `None` if no ancestor carries one).
//! 2. List every `data-claim-id` attribute value seen in the document.
//!
//! These two operations are everything AC-461.6a / 6b need; pulling in
//! `html5ever` or `scraper` for that surface area is overkill. The parser
//! tolerates well-formed snippets the W2 block renderers produce; if a
//! renderer emits malformed HTML the harness will surface it as a binding
//! failure (text-without-binding), which is the correct failure mode.

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TextWithBinding {
    /// Trimmed text content.
    pub text: String,
    /// `Some(id)` if a `data-claim-id` ancestor (closest) carried this text,
    /// else `None` (text-without-binding — AC-461.6a fails).
    pub binding: Option<String>,
}

/// Walks `html` and returns one entry per non-empty text run, each annotated
/// with its closest-ancestor `data-claim-id` if any.
pub fn extract_text_with_bindings(html: &str) -> Vec<TextWithBinding> {
    let mut tokens = Tokenizer::new(html);
    let mut stack: Vec<Option<String>> = Vec::new(); // per-open-element claim-id
    let mut out = Vec::new();

    while let Some(tok) = tokens.next_token() {
        match tok {
            Token::OpenTag {
                self_closing,
                claim_id,
            } => {
                // The closest binding from this element's perspective is its
                // own claim-id, else the parent's.
                let inherited = stack.last().cloned().flatten();
                let effective = claim_id.or(inherited);
                if !self_closing {
                    stack.push(effective);
                }
            }
            Token::CloseTag => {
                let _ = stack.pop();
            }
            Token::Text(raw) => {
                let trimmed = raw.trim();
                if trimmed.is_empty() {
                    continue;
                }
                out.push(TextWithBinding {
                    text: html_unescape(trimmed),
                    binding: stack.last().cloned().flatten(),
                });
            }
        }
    }

    out
}

/// All `data-claim-id` attribute values present in `html`, in document order.
pub fn extract_claim_id_attributes(html: &str) -> Vec<String> {
    let mut tokens = Tokenizer::new(html);
    let mut out = Vec::new();
    while let Some(tok) = tokens.next_token() {
        if let Token::OpenTag {
            claim_id: Some(id), ..
        } = tok
        {
            out.push(id);
        }
    }
    out
}

// ---- internal tokenizer ----------------------------------------------------

enum Token {
    OpenTag {
        self_closing: bool,
        claim_id: Option<String>,
    },
    CloseTag,
    Text(String),
}

struct Tokenizer<'a> {
    bytes: &'a [u8],
    pos: usize,
}

impl<'a> Tokenizer<'a> {
    fn new(html: &'a str) -> Self {
        Self {
            bytes: html.as_bytes(),
            pos: 0,
        }
    }

    fn next_token(&mut self) -> Option<Token> {
        if self.pos >= self.bytes.len() {
            return None;
        }
        if self.bytes[self.pos] == b'<' {
            self.read_tag()
        } else {
            self.read_text()
        }
    }

    fn read_text(&mut self) -> Option<Token> {
        let start = self.pos;
        while self.pos < self.bytes.len() && self.bytes[self.pos] != b'<' {
            self.pos += 1;
        }
        let raw = std::str::from_utf8(&self.bytes[start..self.pos])
            .unwrap_or("")
            .to_string();
        Some(Token::Text(raw))
    }

    fn read_tag(&mut self) -> Option<Token> {
        // Consume '<'.
        self.pos += 1;
        if self.pos >= self.bytes.len() {
            return None;
        }
        // Comment: <!-- ... --> — skip wholesale.
        if self.bytes[self.pos..].starts_with(b"!--") {
            if let Some(end) = find_substring(&self.bytes[self.pos..], b"-->") {
                self.pos += end + 3;
                return self.next_token();
            } else {
                // Unterminated comment — treat as EOF.
                self.pos = self.bytes.len();
                return None;
            }
        }
        // Doctype or other `<!...>` — skip to '>'.
        if self.bytes[self.pos] == b'!' {
            while self.pos < self.bytes.len() && self.bytes[self.pos] != b'>' {
                self.pos += 1;
            }
            if self.pos < self.bytes.len() {
                self.pos += 1;
            }
            return self.next_token();
        }
        let is_close = self.bytes[self.pos] == b'/';
        if is_close {
            self.pos += 1;
        }
        // Read tag name (we don't use it, just need to advance past it).
        while self.pos < self.bytes.len()
            && !self.bytes[self.pos].is_ascii_whitespace()
            && self.bytes[self.pos] != b'>'
            && self.bytes[self.pos] != b'/'
        {
            self.pos += 1;
        }
        if is_close {
            // Walk to '>'.
            while self.pos < self.bytes.len() && self.bytes[self.pos] != b'>' {
                self.pos += 1;
            }
            if self.pos < self.bytes.len() {
                self.pos += 1;
            }
            return Some(Token::CloseTag);
        }
        // Open tag — parse attributes looking for data-claim-id.
        let mut claim_id: Option<String> = None;
        let mut self_closing = false;
        while self.pos < self.bytes.len() && self.bytes[self.pos] != b'>' {
            // Skip whitespace.
            while self.pos < self.bytes.len() && self.bytes[self.pos].is_ascii_whitespace() {
                self.pos += 1;
            }
            if self.pos >= self.bytes.len() {
                break;
            }
            if self.bytes[self.pos] == b'>' {
                break;
            }
            if self.bytes[self.pos] == b'/' {
                self_closing = true;
                self.pos += 1;
                continue;
            }
            // Read attribute name.
            let name_start = self.pos;
            while self.pos < self.bytes.len()
                && !self.bytes[self.pos].is_ascii_whitespace()
                && self.bytes[self.pos] != b'='
                && self.bytes[self.pos] != b'>'
                && self.bytes[self.pos] != b'/'
            {
                self.pos += 1;
            }
            let name = std::str::from_utf8(&self.bytes[name_start..self.pos])
                .unwrap_or("")
                .to_string();
            // Optional value.
            let mut value: Option<String> = None;
            // Skip whitespace before `=`.
            while self.pos < self.bytes.len() && self.bytes[self.pos].is_ascii_whitespace() {
                self.pos += 1;
            }
            if self.pos < self.bytes.len() && self.bytes[self.pos] == b'=' {
                self.pos += 1;
                while self.pos < self.bytes.len() && self.bytes[self.pos].is_ascii_whitespace() {
                    self.pos += 1;
                }
                if self.pos < self.bytes.len()
                    && (self.bytes[self.pos] == b'"' || self.bytes[self.pos] == b'\'')
                {
                    let quote = self.bytes[self.pos];
                    self.pos += 1;
                    let val_start = self.pos;
                    while self.pos < self.bytes.len() && self.bytes[self.pos] != quote {
                        self.pos += 1;
                    }
                    let val = std::str::from_utf8(&self.bytes[val_start..self.pos])
                        .unwrap_or("")
                        .to_string();
                    if self.pos < self.bytes.len() {
                        self.pos += 1; // consume closing quote
                    }
                    value = Some(val);
                } else {
                    let val_start = self.pos;
                    while self.pos < self.bytes.len()
                        && !self.bytes[self.pos].is_ascii_whitespace()
                        && self.bytes[self.pos] != b'>'
                    {
                        self.pos += 1;
                    }
                    let val = std::str::from_utf8(&self.bytes[val_start..self.pos])
                        .unwrap_or("")
                        .to_string();
                    value = Some(val);
                }
            }
            if name.eq_ignore_ascii_case("data-claim-id") {
                if let Some(v) = value {
                    if !v.is_empty() {
                        claim_id = Some(v);
                    }
                }
            }
        }
        if self.pos < self.bytes.len() && self.bytes[self.pos] == b'>' {
            self.pos += 1;
        }
        Some(Token::OpenTag {
            self_closing,
            claim_id,
        })
    }
}

fn find_substring(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    if needle.is_empty() || needle.len() > haystack.len() {
        return None;
    }
    for i in 0..=haystack.len() - needle.len() {
        if &haystack[i..i + needle.len()] == needle {
            return Some(i);
        }
    }
    None
}

fn html_unescape(s: &str) -> String {
    s.replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
}

#[cfg(test)]
mod tokenizer_unit_tests {
    use super::*;

    #[test]
    fn text_with_claim_id_ancestor_traces_back() {
        let html = r#"<div data-claim-id="c-1"><span>hello world</span></div>"#;
        let texts = extract_text_with_bindings(html);
        assert_eq!(texts.len(), 1);
        assert_eq!(texts[0].text, "hello world");
        assert_eq!(texts[0].binding.as_deref(), Some("c-1"));
    }

    #[test]
    fn text_without_claim_id_ancestor_is_unbound() {
        let html = r#"<div><span>bare leak</span></div>"#;
        let texts = extract_text_with_bindings(html);
        assert_eq!(texts.len(), 1);
        assert!(texts[0].binding.is_none());
    }

    #[test]
    fn nearest_ancestor_wins_for_nested_claim_ids() {
        let html =
            r#"<div data-claim-id="outer"><section data-claim-id="inner">nested</section></div>"#;
        let texts = extract_text_with_bindings(html);
        assert_eq!(texts.len(), 1);
        assert_eq!(texts[0].binding.as_deref(), Some("inner"));
    }

    #[test]
    fn self_closing_does_not_persist_binding() {
        // A self-closing tag with data-claim-id MUST NOT bind sibling text.
        let html = r#"<div><br data-claim-id="c-self" />stray text</div>"#;
        let texts = extract_text_with_bindings(html);
        assert_eq!(texts.len(), 1);
        assert_eq!(texts[0].text, "stray text");
        assert!(texts[0].binding.is_none());
    }

    #[test]
    fn extract_claim_id_attributes_visits_all() {
        let html = r#"<a data-claim-id="c-1"></a><b data-claim-id="c-2"></b>"#;
        let ids = extract_claim_id_attributes(html);
        assert_eq!(ids, vec!["c-1".to_string(), "c-2".to_string()]);
    }

    #[test]
    fn comments_and_doctype_do_not_emit_text() {
        let html = r#"<!DOCTYPE html><!-- a comment --><div data-claim-id="c-1">visible</div>"#;
        let texts = extract_text_with_bindings(html);
        assert_eq!(texts.len(), 1);
        assert_eq!(texts[0].text, "visible");
    }
}
