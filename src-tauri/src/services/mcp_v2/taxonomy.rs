//! MCP tool taxonomy + host-selection contract.
//!
//! - [`TaxonomyCatalog`] trait — the abstract catalog interface the
//!   gateway consumes at boot.
//! - [`TaxonomyError`] — fail-fast variants with operator-readable
//!   `Display`.
//! - [`YamlTaxonomyCatalog`] — concrete YAML loader implementing the trait.
//!   Production uses `load_embedded()` reading
//!   the embedded `tool_descriptions` resource via `include_str!`. Dev uses
//!   `load_from_path(env DAILYOS_MCP_TAXONOMY_PATH)` for catalog
//!   iteration without rebuild.
//!
//! Handler bodies follow the handler-contract conventions documented on
//! [`TaxonomyCatalog`] below.

use std::collections::HashMap;
use std::path::Path;

use serde::Deserialize;

use super::contracts::{
    McpToolHandler, ParamSpec, ReturnSpec, Scope, ScopedName, Side, ToolDescription, ToolExample,
};

#[cfg(test)]
#[allow(unused_imports)]
use super::contracts::ToolDescription as _ToolDescriptionImported;

// ---------------------------------------------------------------------------
// TaxonomyError
// ---------------------------------------------------------------------------

/// Errors surfaced when the loaded tool catalog disagrees with the
/// registered [`McpToolHandler`] implementations at startup, or when
/// the YAML catalog itself fails to load / validate.
///
/// These are operator-facing — they indicate a mismatch between the
/// version-controlled YAML catalog and the compiled handler registry.
/// The gateway treats catalog load / validation failure as fatal at
/// boot via `Gateway::seal()` returning `Err(...)`; no `panic!()`.
#[derive(Debug, Clone, PartialEq)]
pub enum TaxonomyError {
    /// A registered handler's `ScopedName` has no matching entry in
    /// the loaded catalog, OR a catalog entry has no matching handler
    /// (catalog-to-handler direction), OR the registered handler's
    /// `Side` disagrees with the catalog entry's `Side`. The optional
    /// `nearest_candidate` is the closest catalog name by Levenshtein
    /// distance for typo DX.
    HandlerCatalogMismatch {
        handler: ScopedName,
        catalog_entry: Option<ScopedName>,
        nearest_candidate: Option<ScopedName>,
    },

    /// The catalog failed to parse (syntax error, unknown field on
    /// `YamlToolEntry` / nested `SelectionFixtures` / `PromptFixture` /
    /// `AdjacentFixture`, or missing required field).
    ParseFailed { error: String },

    /// An entry's `name` failed the canonical naming convention regex
    /// `^dailyos\.(read|write|submit|search|list|get|prepare)\.[a-z][a-z0-9_]*$`.
    InvalidName { name: String, reason: String },

    /// An entry's `scopes_required` includes a scope not in the
    /// allowlist (new `dailyos.<verb>.<noun>` or grandfathered v1.4.5).
    InvalidScope { tool: ScopedName, scope: Scope },

    /// Two YAML entries share the same `name`.
    DuplicateName { name: ScopedName },

    /// A registered handler's `Side` disagrees with the catalog entry's
    /// `Side`.
    SideMismatch {
        handler: ScopedName,
        expected: Side,
        actual: Side,
    },

    /// An entry's `selectionFixtures` lacks the AC-5 coverage minimums
    /// (≥ 2 positive, ≥ 2 negative_broad_corpus, ≥ 1 negative_adjacent_tool).
    FixtureCoverage {
        tool: ScopedName,
        missing: &'static str,
    },
}

impl std::fmt::Display for TaxonomyError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::HandlerCatalogMismatch {
                handler,
                catalog_entry,
                nearest_candidate,
            } => match (catalog_entry, nearest_candidate) {
                (None, Some(near)) => write!(
                    f,
                    "mcp_v2 taxonomy: handler `{handler}` has no catalog entry. \
                     Did you mean `{near}`? Fix the handler name or add an entry to \
                     resources/mcp_v2/tool_descriptions.yaml.",
                ),
                (None, None) => write!(
                    f,
                    "mcp_v2 taxonomy: handler `{handler}` has no catalog entry. \
                     Add an entry to resources/mcp_v2/tool_descriptions.yaml.",
                ),
                (Some(_), _) => write!(
                    f,
                    "mcp_v2 taxonomy: handler `{handler}` Side disagrees with catalog entry. \
                     Verify the handler's description().side matches the catalog entry's side.",
                ),
            },
            Self::ParseFailed { error } => write!(
                f,
                "mcp_v2 taxonomy: catalog parse failed: {error}. \
                 Check resources/mcp_v2/tool_descriptions.yaml for syntax, unknown fields, \
                 or missing required fields.",
            ),
            Self::InvalidName { name, reason } => write!(
                f,
                "mcp_v2 taxonomy: catalog entry name `{name}` is invalid: {reason}. \
                 Names must match `dailyos.<verb>.<noun>` per ADR-0102 §E.",
            ),
            Self::InvalidScope { tool, scope } => write!(
                f,
                "mcp_v2 taxonomy: tool `{tool}` requires scope `{scope}` which is not in \
                 the allowlist. New scopes use `dailyos.<verb>.<noun>` form; v1.4.5 \
                 grandfathered scopes are listed in taxonomy.rs::GRANDFATHERED_SCOPES.",
            ),
            Self::DuplicateName { name } => write!(
                f,
                "mcp_v2 taxonomy: catalog has duplicate entry for `{name}`. \
                 Remove the duplicate from resources/mcp_v2/tool_descriptions.yaml.",
            ),
            Self::SideMismatch {
                handler,
                expected,
                actual,
            } => write!(
                f,
                "mcp_v2 taxonomy: handler `{handler}` side mismatch: \
                 catalog says {expected:?} but handler advertises {actual:?}. \
                 Fix the handler's description().side or update the catalog entry.",
            ),
            Self::FixtureCoverage { tool, missing } => write!(
                f,
                "mcp_v2 taxonomy: tool `{tool}` selectionFixtures missing required \
                 coverage: {missing}. AC-5 requires ≥ 2 positive + ≥ 2 \
                 negativeBroadCorpus + ≥ 1 negativeAdjacentTool.",
            ),
        }
    }
}

impl std::error::Error for TaxonomyError {}

// ---------------------------------------------------------------------------
// TaxonomyCatalog trait
// ---------------------------------------------------------------------------

/// The abstract catalog interface the gateway uses at boot. The
/// concrete implementation is [`YamlTaxonomyCatalog`].
///
/// # Handler-contract conventions (read before writing a handler in W2 / W3 / W4)
///
/// Every concrete [`McpToolHandler`] in `services::mcp_v2::handlers::*`
/// must obey the following rules. The gateway enforces them at
/// dispatch time; failure to follow them produces operator-visible
/// warnings or rejected calls.
///
/// ## Side::Read handlers
///
/// Read tools may return any JSON shape compatible with the
/// `ToolDescription.returns.schema` advertised in the catalog. There
/// is no `mutation_cursor` requirement.
///
/// ## Side::Write and Side::SubmitCorrection handlers — `mutation_cursor` field
///
/// Write and submit-correction tools must include a `mutation_cursor`
/// field in their success payload. The gateway extracts it via JSON
/// inspection and folds it into the audit detail JSON as forensic
/// ordering material. Omitting it produces a `Suite-S` warning signal
/// (`mcp_write_handler_missing_cursor`).
///
/// **Cursor shape: IDs only, no payload data.** Stable substrate IDs
/// (UUIDs, integer primary keys, opaque hashes). Never embed user
/// content, email addresses, names, or any PII-shaped string. The
/// gateway caps the cursor at depth 4 and 2 KiB serialized; oversize
/// cursors are replaced with `"truncated_oversize"` and a `Suite-S`
/// warning (`mcp_write_handler_cursor_truncated`) is emitted.
pub trait TaxonomyCatalog: Send + Sync {
    /// Handler→catalog: verify every registered handler has a matching
    /// catalog entry with matching `Side`. Production `Gateway::seal()`
    /// uses this; mismatch = fatal at boot.
    fn validate_against_handlers(
        &self,
        handlers: &[&dyn McpToolHandler],
    ) -> Result<(), TaxonomyError>;

    /// Catalog→handler: return the catalog entries that have no
    /// matching handler. NOT an error in production — W2/W3/W4 land
    /// handlers incrementally; the gateway logs the list as operator
    /// info.
    fn validate_catalog_against_handlers(
        &self,
        handlers: &[&dyn McpToolHandler],
    ) -> Vec<ScopedName>;

    /// Look up the catalog-declared [`Side`] tier for a tool by name.
    fn side_for(&self, tool_name: &ScopedName) -> Option<Side>;

    /// Look up the full `ToolDescription` for a tool by name. Used by
    /// the v2 transport (W1.5) to compose `tools/list` descriptions
    /// from `summary` + `when_to_call` + `when_NOT_to_call`.
    fn description_for(&self, tool_name: &ScopedName) -> Option<&ToolDescription>;

    /// Iterate every catalog entry's `ScopedName`. Used by transport
    /// `tools/list` to enumerate handler candidates before filtering by
    /// manifest grants.
    fn iter_names(&self) -> Box<dyn Iterator<Item = &ScopedName> + '_>;
}

// ---------------------------------------------------------------------------
// YAML DTOs
// ---------------------------------------------------------------------------

/// Wire-shape DTO for a single tool entry in the embedded tool catalog.
/// Splits into `(ToolDescription, SelectionFixtures)` at load time via
/// [`Self::into_description_and_fixtures`].
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct YamlToolEntry {
    pub name: ScopedName,
    pub side: Side,
    pub summary: String,
    #[serde(rename = "when_to_call")]
    pub when_to_call: String,
    #[serde(rename = "when_NOT_to_call")]
    pub when_not_to_call: String,
    pub scopes_required: Vec<Scope>,
    pub parameters: Vec<ParamSpec>,
    pub returns: ReturnSpec,
    pub examples: Vec<ToolExample>,
    pub selection_fixtures: SelectionFixtures,
}

impl YamlToolEntry {
    pub fn into_description_and_fixtures(self) -> (ToolDescription, SelectionFixtures) {
        let description = ToolDescription {
            name: self.name,
            summary: self.summary,
            when_to_call: self.when_to_call,
            when_not_to_call: self.when_not_to_call,
            side: self.side,
            parameters: self.parameters,
            returns: self.returns,
            examples: self.examples,
            scopes_required: self.scopes_required,
        };
        (description, self.selection_fixtures)
    }
}

/// Host-selection eval fixture stubs consumed by the routing evaluator.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct SelectionFixtures {
    pub positive: Vec<PromptFixture>,
    pub negative_broad_corpus: Vec<PromptFixture>,
    pub negative_adjacent_tool: Vec<AdjacentFixture>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct PromptFixture {
    pub prompt: String,
    #[serde(default)]
    pub expected_tool: Option<ScopedName>,
    #[serde(default)]
    pub expected_tool_class: Option<String>,
    #[serde(default)]
    pub expected_argument_keys: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct AdjacentFixture {
    pub prompt: String,
    pub expected_tool: ScopedName,
}

// ---------------------------------------------------------------------------
// YamlTaxonomyCatalog
// ---------------------------------------------------------------------------

/// Embedded catalog compiled into the binary. Production uses
/// [`YamlTaxonomyCatalog::load_embedded`]; dev can override via
/// `DAILYOS_MCP_TAXONOMY_PATH` env var pointed at a local YAML file.
const EMBEDDED_YAML: &str = include_str!("resources/tool_descriptions.yaml");

/// Tool-name regex per ADR-0102 §E.
const NAME_RE: &str = r"^dailyos\.(read|write|submit|search|list|get|prepare)\.[a-z][a-z0-9_]*$";

/// Grandfathered scopes (per ADR-0102 §E): kept unprefixed verbatim to avoid
/// breaking existing substrate.
const GRANDFATHERED_SCOPES: &[&str] = &[
    "write.workspace_place_document",
    "read.workspace_graph",
    "read.entity_names",
    "submit.feedback",
    "submit.correction",
    "submit.dismissal",
];

#[derive(Debug)]
pub struct YamlTaxonomyCatalog {
    entries: HashMap<ScopedName, (ToolDescription, SelectionFixtures)>,
}

impl YamlTaxonomyCatalog {
    /// Production loader — reads the embedded YAML resource.
    pub fn load_embedded() -> Result<Self, TaxonomyError> {
        Self::load_from_str(EMBEDDED_YAML)
    }

    /// Dev override — reads YAML from filesystem. Mirrors
    /// `src/presets/loader.rs` precedent. Env-gate via
    /// `DAILYOS_MCP_TAXONOMY_PATH` in callers; this function itself is
    /// path-agnostic so tests can call directly.
    pub fn load_from_path(path: &Path) -> Result<Self, TaxonomyError> {
        let content = std::fs::read_to_string(path).map_err(|e| TaxonomyError::ParseFailed {
            error: format!("read {path:?}: {e}"),
        })?;
        Self::load_from_str(&content)
    }

    pub fn load_from_str(yaml: &str) -> Result<Self, TaxonomyError> {
        let raw: Vec<YamlToolEntry> =
            serde_json::from_str(yaml).map_err(|e| TaxonomyError::ParseFailed {
                error: e.to_string(),
            })?;

        let name_re = regex::Regex::new(NAME_RE).expect("static regex compiles");
        let mut entries = HashMap::with_capacity(raw.len());

        for entry in raw {
            // Naming convention.
            if !name_re.is_match(entry.name.as_str()) {
                return Err(TaxonomyError::InvalidName {
                    name: entry.name.as_str().to_string(),
                    reason: format!("does not match `{NAME_RE}`"),
                });
            }

            // Scope allowlist.
            for scope in &entry.scopes_required {
                if !scope_in_allowlist(scope) {
                    return Err(TaxonomyError::InvalidScope {
                        tool: entry.name.clone(),
                        scope: scope.clone(),
                    });
                }
            }

            // Fixture coverage.
            let (desc, fixtures) = entry.into_description_and_fixtures();
            if fixtures.positive.len() < 2 {
                return Err(TaxonomyError::FixtureCoverage {
                    tool: desc.name.clone(),
                    missing: "positive < 2",
                });
            }
            if fixtures.negative_broad_corpus.len() < 2 {
                return Err(TaxonomyError::FixtureCoverage {
                    tool: desc.name.clone(),
                    missing: "negativeBroadCorpus < 2",
                });
            }
            if fixtures.negative_adjacent_tool.is_empty() {
                return Err(TaxonomyError::FixtureCoverage {
                    tool: desc.name.clone(),
                    missing: "negativeAdjacentTool < 1",
                });
            }

            // Duplicate name.
            if entries.contains_key(&desc.name) {
                return Err(TaxonomyError::DuplicateName {
                    name: desc.name.clone(),
                });
            }
            entries.insert(desc.name.clone(), (desc, fixtures));
        }

        Ok(Self { entries })
    }

    pub fn description_for(&self, name: &ScopedName) -> Option<&ToolDescription> {
        self.entries.get(name).map(|(d, _)| d)
    }

    pub fn fixtures_for(&self, name: &ScopedName) -> Option<&SelectionFixtures> {
        self.entries.get(name).map(|(_, f)| f)
    }

    pub fn entries_with_fixtures(
        &self,
    ) -> impl Iterator<Item = (&ScopedName, &ToolDescription, &SelectionFixtures)> {
        self.entries.iter().map(|(n, (d, f))| (n, d, f))
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

impl TaxonomyCatalog for YamlTaxonomyCatalog {
    fn validate_against_handlers(
        &self,
        handlers: &[&dyn McpToolHandler],
    ) -> Result<(), TaxonomyError> {
        for handler in handlers {
            let desc = handler.description();
            match self.entries.get(&desc.name) {
                None => {
                    return Err(TaxonomyError::HandlerCatalogMismatch {
                        handler: desc.name.clone(),
                        catalog_entry: None,
                        nearest_candidate: self.nearest_name(desc.name.as_str()),
                    });
                }
                Some((catalog_desc, _)) => {
                    if catalog_desc.side != desc.side {
                        return Err(TaxonomyError::SideMismatch {
                            handler: desc.name.clone(),
                            expected: catalog_desc.side,
                            actual: desc.side,
                        });
                    }
                }
            }
        }
        Ok(())
    }

    fn validate_catalog_against_handlers(
        &self,
        handlers: &[&dyn McpToolHandler],
    ) -> Vec<ScopedName> {
        let handler_names: std::collections::HashSet<&ScopedName> =
            handlers.iter().map(|h| &h.description().name).collect();
        self.entries
            .keys()
            .filter(|n| !handler_names.contains(n))
            .cloned()
            .collect()
    }

    fn side_for(&self, tool_name: &ScopedName) -> Option<Side> {
        self.entries.get(tool_name).map(|(d, _)| d.side)
    }

    fn description_for(&self, tool_name: &ScopedName) -> Option<&ToolDescription> {
        self.entries.get(tool_name).map(|(d, _)| d)
    }

    fn iter_names(&self) -> Box<dyn Iterator<Item = &ScopedName> + '_> {
        Box::new(self.entries.keys())
    }
}

impl YamlTaxonomyCatalog {
    /// Nearest catalog name by simple prefix + Levenshtein-ish edit
    /// distance for typo DX. Returns None if no entry is close enough.
    fn nearest_name(&self, needle: &str) -> Option<ScopedName> {
        self.entries
            .keys()
            .min_by_key(|name| levenshtein(name.as_str(), needle))
            .cloned()
    }
}

fn scope_in_allowlist(scope: &Scope) -> bool {
    let s = scope.as_str();
    if GRANDFATHERED_SCOPES.contains(&s) {
        return true;
    }
    // dailyos.<verb>.<noun>
    let parts: Vec<&str> = s.split('.').collect();
    if parts.len() != 3 || parts[0] != "dailyos" {
        return false;
    }
    matches!(
        parts[1],
        "read" | "write" | "submit" | "search" | "list" | "get" | "prepare"
    ) && !parts[2].is_empty()
        && parts[2]
            .chars()
            .next()
            .map(|c| c.is_ascii_lowercase())
            .unwrap_or(false)
        && parts[2]
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_')
}

/// Minimal Levenshtein for nearest-candidate suggestion DX.
fn levenshtein(a: &str, b: &str) -> usize {
    let (a, b) = (a.as_bytes(), b.as_bytes());
    let (m, n) = (a.len(), b.len());
    if m == 0 {
        return n;
    }
    if n == 0 {
        return m;
    }
    let mut prev: Vec<usize> = (0..=n).collect();
    let mut curr = vec![0usize; n + 1];
    for i in 1..=m {
        curr[0] = i;
        for j in 1..=n {
            let cost = if a[i - 1] == b[j - 1] { 0 } else { 1 };
            curr[j] = (prev[j] + 1).min(curr[j - 1] + 1).min(prev[j - 1] + cost);
        }
        std::mem::swap(&mut prev, &mut curr);
    }
    prev[n]
}

#[cfg(test)]
mod tests {
    use super::*;

    fn catalog_json(entries: Vec<String>) -> String {
        format!("[{}]", entries.join(","))
    }

    fn valid_entry_json(name: &str, scope: &str) -> String {
        format!(
            r#"{{
                "name": "{name}",
                "side": "Read",
                "summary": "x",
                "when_to_call": "y",
                "when_NOT_to_call": "z",
                "scopesRequired": ["{scope}"],
                "parameters": [],
                "returns": {{ "schema": {{}}, "description": "x" }},
                "examples": [],
                "selectionFixtures": {{
                    "positive": [
                        {{ "prompt": "a", "expectedTool": "dailyos.read.foo" }},
                        {{ "prompt": "b", "expectedTool": "dailyos.read.foo" }}
                    ],
                    "negativeBroadCorpus": [
                        {{ "prompt": "c", "expectedToolClass": "external" }},
                        {{ "prompt": "d", "expectedToolClass": "external" }}
                    ],
                    "negativeAdjacentTool": [
                        {{ "prompt": "e", "expectedTool": "dailyos.read.bar" }}
                    ]
                }}
            }}"#
        )
    }

    #[test]
    fn embedded_catalog_loads_clean_with_ten_entries() {
        let catalog = YamlTaxonomyCatalog::load_embedded().expect("embedded catalog loads");
        assert_eq!(catalog.len(), 10, "expected 10 tool entries per DOS-478 §5");
    }

    #[test]
    fn embedded_catalog_contains_required_inventory() {
        let catalog = YamlTaxonomyCatalog::load_embedded().expect("embedded catalog loads");
        for name in [
            "dailyos.read.account_status",
            "dailyos.read.daily_briefing",
            "dailyos.read.meeting_briefing",
            "dailyos.read.portfolio_attention",
            "dailyos.search.workspace_memory",
            "dailyos.read.workspace_source_provenance",
            "dailyos.write.place_document",
            "dailyos.submit.note",
            "dailyos.submit.action",
            "dailyos.submit.action_status",
        ] {
            assert!(
                catalog.description_for(&ScopedName::new(name)).is_some(),
                "missing required tool: {name}",
            );
        }
    }

    #[test]
    fn invalid_name_rejected() {
        let catalog = catalog_json(vec![valid_entry_json(
            "NotDailyos.read.foo",
            "dailyos.read.foo",
        )]);
        match YamlTaxonomyCatalog::load_from_str(&catalog) {
            Err(TaxonomyError::InvalidName { .. }) => {}
            other => panic!("expected InvalidName, got {other:?}"),
        }
    }

    #[test]
    fn invalid_scope_rejected() {
        let catalog = catalog_json(vec![valid_entry_json(
            "dailyos.read.foo",
            "some.random.scope",
        )]);
        match YamlTaxonomyCatalog::load_from_str(&catalog) {
            Err(TaxonomyError::InvalidScope { .. }) => {}
            other => panic!("expected InvalidScope, got {other:?}"),
        }
    }

    #[test]
    fn duplicate_name_rejected() {
        let catalog = catalog_json(vec![
            valid_entry_json("dailyos.read.foo", "dailyos.read.foo"),
            valid_entry_json("dailyos.read.foo", "dailyos.read.foo"),
        ]);
        match YamlTaxonomyCatalog::load_from_str(&catalog) {
            Err(TaxonomyError::DuplicateName { .. }) => {}
            other => panic!("expected DuplicateName, got {other:?}"),
        }
    }

    #[test]
    fn unknown_top_level_field_rejected() {
        let entry = valid_entry_json("dailyos.read.foo", "dailyos.read.foo").replace(
            r#""selectionFixtures""#,
            r#""unknownField": "oops", "selectionFixtures""#,
        );
        let catalog = catalog_json(vec![entry]);
        match YamlTaxonomyCatalog::load_from_str(&catalog) {
            Err(TaxonomyError::ParseFailed { .. }) => {}
            other => panic!("expected ParseFailed, got {other:?}"),
        }
    }

    #[test]
    fn fixture_coverage_enforced() {
        let entry = valid_entry_json("dailyos.read.foo", "dailyos.read.foo").replace(
            r#""positive": [
                        { "prompt": "a", "expectedTool": "dailyos.read.foo" },
                        { "prompt": "b", "expectedTool": "dailyos.read.foo" }
                    ]"#,
            r#""positive": [
                        { "prompt": "a", "expectedTool": "dailyos.read.foo" }
                    ]"#,
        );
        let catalog = catalog_json(vec![entry]);
        match YamlTaxonomyCatalog::load_from_str(&catalog) {
            Err(TaxonomyError::FixtureCoverage { missing, .. }) => {
                assert!(missing.contains("positive"));
            }
            other => panic!("expected FixtureCoverage, got {other:?}"),
        }
    }

    #[test]
    fn account_status_host_selection_positive_negative() {
        let catalog = YamlTaxonomyCatalog::load_embedded().expect("embedded catalog loads");
        let fixtures = catalog
            .fixtures_for(&ScopedName::new("dailyos.read.account_status"))
            .expect("account status fixtures");

        assert!(fixtures.positive.iter().all(|fixture| {
            fixture.expected_tool.as_ref() == Some(&ScopedName::new("dailyos.read.account_status"))
                && fixture.expected_tool_class.is_none()
        }));
        assert!(fixtures.negative_broad_corpus.iter().all(|fixture| {
            fixture.expected_tool.is_none()
                && fixture.expected_tool_class.as_deref() == Some("external")
        }));
        assert!(fixtures.negative_adjacent_tool.iter().any(|fixture| {
            fixture.expected_tool == ScopedName::new("dailyos.search.workspace_memory")
        }));
    }

    #[test]
    fn nearest_candidate_returned_for_typo() {
        let catalog = YamlTaxonomyCatalog::load_embedded().expect("embedded catalog loads");
        let near = catalog.nearest_name("dailyos.read.accont_status"); // typo
        assert_eq!(
            near,
            Some(ScopedName::new("dailyos.read.account_status")),
            "nearest should suggest the real name",
        );
    }

    #[test]
    fn levenshtein_basic() {
        assert_eq!(levenshtein("kitten", "sitting"), 3);
        assert_eq!(levenshtein("", "abc"), 3);
        assert_eq!(levenshtein("abc", ""), 3);
        assert_eq!(levenshtein("same", "same"), 0);
    }
}
