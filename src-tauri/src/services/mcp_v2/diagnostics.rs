//! MCP diagnostic logging helpers.
//!
//! MCP stderr is visible to the host process. Treat it as egress: keep
//! categories and short digests, never raw request, DB, path, actor, handle,
//! or correction payload details.

use sha2::{Digest, Sha256};

pub(crate) fn digest_token(value: &str) -> String {
    let digest = Sha256::digest(value.as_bytes());
    digest
        .iter()
        .take(6)
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>()
}

pub(crate) fn sanitized_category(category: &str) -> &str {
    if category
        .bytes()
        .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-' | b'.'))
    {
        category
    } else {
        "uncategorized"
    }
}

pub(crate) fn log_detail(category: &str, detail: impl AsRef<str>) {
    eprintln!(
        "mcp_v2.diagnostic category={} detail_digest={}",
        sanitized_category(category),
        digest_token(detail.as_ref())
    );
}

pub(crate) fn log_event(category: &str) {
    eprintln!(
        "mcp_v2.diagnostic category={}",
        sanitized_category(category)
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn digest_token_does_not_echo_raw_identifier_or_payload() {
        let raw = r#"claim_id=claim-123 correction={"text":"private"} /Users/example/db.sqlite"#;
        let digest = digest_token(raw);

        assert_eq!(digest.len(), 12);
        assert!(digest.chars().all(|ch| ch.is_ascii_hexdigit()));
        assert!(!digest.contains("claim-123"));
        assert!(!digest.contains("private"));
        assert!(!digest.contains("/Users"));
    }

    #[test]
    fn category_rejects_freeform_strings() {
        assert_eq!(
            sanitized_category("rate-limit.rollback"),
            "rate-limit.rollback"
        );
        assert_eq!(
            sanitized_category("detail=/Users/example/db.sqlite"),
            "uncategorized"
        );
    }

    #[test]
    fn mcp_v2_stderr_call_sites_do_not_name_raw_identifiers() {
        let files = [
            ("gateway.rs", include_str!("gateway.rs")),
            (
                "tool_account_status.rs",
                include_str!("handlers/tool_account_status.rs"),
            ),
            (
                "tool_placement.rs",
                include_str!("handlers/tool_placement.rs"),
            ),
        ];
        let forbidden = [
            "client_id=",
            "conversation_handle=",
            "detail={",
            "{err",
            "{error",
            "{rollback_err",
            "{commit_err",
            "claim_id=",
            "source_id=",
            "action_id=",
        ];

        for (file, source) in files {
            for line in source.lines().filter(|line| line.contains("eprintln!")) {
                for needle in forbidden {
                    assert!(
                        !line.contains(needle),
                        "{file} emits raw diagnostic pattern `{needle}` in `{line}`"
                    );
                }
            }
        }
    }
}
