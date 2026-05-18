use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use abilities_runtime::abilities::composition::{
    AbilityRef, BindingRole, Block, BlockId, ClaimRef, ClaimRefIndex, Composition,
    CompositionDocId, CompositionKind, CompositionMetadata, CompositionVersion, FieldBinding,
    ProvenanceRef, RenderHints, Salience, Section, SectionId,
};
use abilities_runtime::abilities::provenance::{FieldPath, InvocationId, SchemaVersion};
use abilities_runtime::abilities::registry::{Actor, ScopeSet, SurfaceClientId, SurfaceScope};
use abilities_runtime::abilities::{
    project_composition_for_surface, FallbackProjectionContext, ProjectedComposition,
    ProjectionDiagnostic as RuntimeProjectionDiagnostic, SurfaceKind,
};
use chrono::{TimeZone, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone)]
pub struct BlockIntegrationFixture {
    pub ability_name: String,
    pub composition_id: String,
    pub input_json: Value,
    pub expected_bindings: Vec<BindingExpectation>,
    pub expected_diagnostics: Vec<ProjectionDiagnostic>,
    pub expected_renderer_branches: Vec<RendererBranchAssertion>,
    pub expected_wrapper: BlockWrapperAssertion,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BindingExpectation {
    pub pointer: String,
    pub value_kind: ValueKind,
    pub required: bool,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ValueKind {
    String,
    Number,
    Bool,
    Array,
    Object,
    Null,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RendererBranchAssertion {
    pub branch_label: String,
    pub expected_html_pattern: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlockWrapperAssertion {
    pub tag: String,
    pub class: String,
    pub data_attrs: Vec<(String, String)>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectionDiagnostic {
    pub reason: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RenderedHtml(pub String);

#[derive(Debug, Serialize)]
struct PhpExpectedContract<'a> {
    bindings: &'a [BindingExpectation],
    diagnostics: &'a [ProjectionDiagnostic],
    renderer_branches: &'a [RendererBranchAssertion],
    wrapper: &'a BlockWrapperAssertion,
}

pub async fn run_block_integration_fixture(fixture: BlockIntegrationFixture) -> RenderedHtml {
    let composition: Composition = serde_json::from_value(fixture.input_json.clone())
        .unwrap_or_else(|error| {
            panic_contract_mismatch(
                "fixture.input_json",
                "Composition",
                &format!("invalid composition JSON: {error}"),
                None,
            )
        });

    if composition.id.as_str() != fixture.composition_id {
        panic_contract_mismatch(
            "fixture.composition_id",
            &fixture.composition_id,
            composition.id.as_str(),
            Some(composition.id.as_str().to_string()),
        );
    }

    let context = surface_client_projection_context();
    let (projection, _) =
        project_composition_for_surface(&composition, &context).unwrap_or_else(|error| {
            panic_contract_mismatch(
                "project_composition_for_surface",
                "successful projection",
                &format!("{error:?}"),
                None::<String>,
            )
        });

    assert_projection_diagnostics(&fixture, &projection);
    assert_binding_contracts(&fixture, &projection);

    let repo_root = repo_root();
    let fixture_root = repo_root.join("wp/dailyos/tests/fixtures/blocks");
    fs::create_dir_all(&fixture_root)
        .unwrap_or_else(|error| panic!("failed to create {}: {error}", fixture_root.display()));
    let temp_dir = tempfile::Builder::new()
        .prefix(&format!("{}-", safe_path_part(&fixture.ability_name)))
        .tempdir_in(&fixture_root)
        .unwrap_or_else(|error| {
            panic!(
                "failed to create block integration fixture dir under {}: {error}",
                fixture_root.display()
            )
        });

    let projection_path = temp_dir.path().join("projection.json");
    let expected_path = temp_dir.path().join("expected.json");
    write_json(&projection_path, &projection);
    write_json(
        &expected_path,
        &PhpExpectedContract {
            bindings: &fixture.expected_bindings,
            diagnostics: &fixture.expected_diagnostics,
            renderer_branches: &fixture.expected_renderer_branches,
            wrapper: &fixture.expected_wrapper,
        },
    );

    let output = Command::new("php")
        .current_dir(&repo_root)
        .arg("wp/dailyos/tests/blocks/StarterKitIntegrationTest.php")
        .arg(&projection_path)
        .arg(&fixture.ability_name)
        .arg(&expected_path)
        .output()
        .unwrap_or_else(|error| {
            panic!(
                "failed to run PHP block integration harness for {}: {error}",
                fixture.ability_name
            )
        });

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let stdout = String::from_utf8_lossy(&output.stdout);
        panic!(
            "PHP block integration harness failed for {}\nstatus: {}\nstderr:\n{}\nstdout:\n{}",
            fixture.ability_name, output.status, stderr, stdout
        );
    }

    RenderedHtml(String::from_utf8(output.stdout).unwrap_or_else(|error| {
        panic!(
            "PHP block integration harness returned non-UTF-8 HTML for {}: {error}",
            fixture.ability_name
        )
    }))
}

pub fn block_fixture_selected(fixture: &BlockIntegrationFixture) -> bool {
    let Ok(requested) = env::var("DAILYOS_BLOCK_SLUG") else {
        return true;
    };
    let requested = requested.trim();
    requested.is_empty()
        || requested == fixture.ability_name
        || fixture
            .ability_name
            .rsplit('/')
            .next()
            .is_some_and(|slug| slug == requested)
        || fixture.ability_name.replace('_', "-").ends_with(requested)
}

#[macro_export]
macro_rules! integration_test_block {
    ($name:ident, $fixture:path) => {
        #[tokio::test]
        async fn $name() {
            let fixture = $fixture();
            if !$crate::block_fixture_selected(&fixture) {
                return;
            }
            let rendered = $crate::run_block_integration_fixture(fixture).await;
            assert!(
                !rendered.0.trim().is_empty(),
                "block integration harness returned empty HTML"
            );
        }
    };
}

pub fn fixture_provenance_ref() -> ProvenanceRef {
    ProvenanceRef::from_pointer(
        InvocationId(uuid::Uuid::from_u128(
            0x1234_5678_90ab_cdef_1122_3344_5566_7788,
        )),
        "/sections/0/blocks/0",
    )
    .expect("fixture provenance pointer is valid")
}

pub fn fixture_claim(id: &str, field_path: &str) -> ClaimRef {
    ClaimRef::with_field(
        id,
        1,
        FieldPath::new(field_path).expect("fixture claim field path is valid"),
    )
}

pub fn fixture_binding(field_path: &str, role: BindingRole, claim_refs: &[usize]) -> FieldBinding {
    FieldBinding {
        field_path: FieldPath::new(field_path).expect("fixture binding field path is valid"),
        role,
        claim_refs: claim_refs.iter().copied().map(ClaimRefIndex).collect(),
    }
}

pub fn fixture_block(
    id: &str,
    block_type: abilities_runtime::abilities::composition::BlockType,
    attributes: Value,
    claim_refs: Vec<ClaimRef>,
    field_bindings: Vec<FieldBinding>,
) -> Block {
    Block {
        id: BlockId::new(id),
        block_type,
        attributes,
        claim_refs,
        field_bindings,
        provenance: fixture_provenance_ref(),
        salience: Salience::default(),
        render_hints: RenderHints::default(),
    }
}

pub fn fixture_composition(
    ability_name: &str,
    composition_id: &str,
    version: u64,
    blocks: Vec<Block>,
) -> Composition {
    let generated_at = Utc.with_ymd_and_hms(2026, 5, 18, 0, 0, 0).unwrap();
    let mut composition = Composition::empty(
        CompositionDocId::new(composition_id),
        CompositionVersion::new(version),
        generated_at,
    );
    composition.kind = CompositionKind::EntityPage;
    composition.sections = vec![Section::new(SectionId::new("section-fixture"), blocks)];
    composition.generated_by = AbilityRef::new(ability_name);
    composition.metadata = CompositionMetadata {
        schema_version: SchemaVersion(1),
        generated_at,
        composition_version: CompositionVersion::new(version),
        generated_by: ability_name.to_string(),
    };
    composition
}

fn surface_client_projection_context() -> FallbackProjectionContext {
    let scopes = ScopeSet::new([
        SurfaceScope::new("read.composition"),
        SurfaceScope::new("submit.feedback"),
    ])
    .expect("fixture surface scopes are valid");
    FallbackProjectionContext::new(
        Actor::SurfaceClient {
            instance: SurfaceClientId::new("wordpress-fixture"),
            scopes,
        },
        SurfaceKind::SurfaceClient,
        3,
    )
}

fn assert_projection_diagnostics(
    fixture: &BlockIntegrationFixture,
    projection: &ProjectedComposition,
) {
    let declared: Vec<String> = fixture
        .expected_diagnostics
        .iter()
        .map(|diagnostic| diagnostic.reason.clone())
        .collect();
    let actual: Vec<String> = projection
        .diagnostics
        .iter()
        .map(runtime_diagnostic_reason)
        .collect();
    if declared != actual {
        panic_contract_mismatch(
            "projection.diagnostics[*].reason",
            &format!("{declared:?}"),
            &format!("{actual:?}"),
            nearest_string(
                declared.first().map(String::as_str).unwrap_or_default(),
                &actual,
            ),
        );
    }
}

fn assert_binding_contracts(fixture: &BlockIntegrationFixture, projection: &ProjectedComposition) {
    let projection_json =
        serde_json::to_value(projection).expect("projected composition serializes to JSON");
    let mut candidates = Vec::new();
    collect_json_pointers(&projection_json, "", &mut candidates);
    for block in &projection.blocks {
        collect_json_pointers(&block.payload, "", &mut candidates);
    }

    for binding in &fixture.expected_bindings {
        let value = projection_json.pointer(&binding.pointer).or_else(|| {
            projection
                .blocks
                .first()
                .and_then(|block| block.payload.pointer(&binding.pointer))
        });

        let Some(value) = value else {
            if binding.required {
                panic_contract_mismatch(
                    &binding.pointer,
                    &format!("{:?}", binding.value_kind),
                    "missing",
                    nearest_string(&binding.pointer, &candidates),
                );
            }
            continue;
        };

        let actual_kind = ValueKind::from_value(value);
        if actual_kind != binding.value_kind {
            panic_contract_mismatch(
                &binding.pointer,
                &format!("{:?}", binding.value_kind),
                &format!("{actual_kind:?}"),
                nearest_string(&binding.pointer, &candidates),
            );
        }
    }
}

fn runtime_diagnostic_reason(diagnostic: &RuntimeProjectionDiagnostic) -> String {
    serde_json::to_value(diagnostic.reason)
        .ok()
        .and_then(|value| value.as_str().map(ToOwned::to_owned))
        .unwrap_or_else(|| format!("{:?}", diagnostic.reason))
}

impl ValueKind {
    fn from_value(value: &Value) -> Self {
        match value {
            Value::String(_) => Self::String,
            Value::Number(_) => Self::Number,
            Value::Bool(_) => Self::Bool,
            Value::Array(_) => Self::Array,
            Value::Object(_) => Self::Object,
            Value::Null => Self::Null,
        }
    }
}

fn repo_root() -> PathBuf {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    for ancestor in manifest_dir.ancestors() {
        if ancestor.join("wp/dailyos").is_dir() && ancestor.join("src-tauri").is_dir() {
            return ancestor.to_path_buf();
        }
    }
    panic!("could not locate repo root from {}", manifest_dir.display());
}

fn write_json<T: Serialize>(path: &Path, value: &T) {
    let payload = serde_json::to_vec_pretty(value)
        .unwrap_or_else(|error| panic!("failed to serialize {}: {error}", path.display()));
    fs::write(path, payload)
        .unwrap_or_else(|error| panic!("failed to write {}: {error}", path.display()));
}

fn safe_path_part(value: &str) -> String {
    value
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || ch == '-' || ch == '_' {
                ch
            } else {
                '-'
            }
        })
        .collect()
}

fn panic_contract_mismatch(
    location: &str,
    declared: &str,
    actual: &str,
    did_you_mean: Option<String>,
) -> ! {
    let suggestion = did_you_mean
        .as_deref()
        .filter(|value| !value.is_empty())
        .unwrap_or("n/a");
    panic!(
        "block contract mismatch\nlocation: {location}\ndeclared: {declared}\nactual: {actual}\ndid_you_mean: {suggestion}"
    );
}

fn nearest_string(target: &str, candidates: &[String]) -> Option<String> {
    candidates
        .iter()
        .min_by_key(|candidate| levenshtein(target, candidate))
        .cloned()
}

fn levenshtein(left: &str, right: &str) -> usize {
    let right_chars: Vec<char> = right.chars().collect();
    let mut previous: Vec<usize> = (0..=right_chars.len()).collect();
    let mut current = vec![0; right_chars.len() + 1];

    for (left_index, left_char) in left.chars().enumerate() {
        current[0] = left_index + 1;
        for (right_index, right_char) in right_chars.iter().enumerate() {
            let insertion = current[right_index] + 1;
            let deletion = previous[right_index + 1] + 1;
            let substitution = previous[right_index] + usize::from(left_char != *right_char);
            current[right_index + 1] = insertion.min(deletion).min(substitution);
        }
        std::mem::swap(&mut previous, &mut current);
    }

    previous[right_chars.len()]
}

fn collect_json_pointers(value: &Value, base: &str, pointers: &mut Vec<String>) {
    pointers.push(if base.is_empty() {
        "/".to_string()
    } else {
        base.to_string()
    });
    match value {
        Value::Array(items) => {
            for (index, item) in items.iter().enumerate() {
                collect_json_pointers(item, &format!("{base}/{index}"), pointers);
            }
        }
        Value::Object(map) => {
            for (key, item) in map {
                collect_json_pointers(
                    item,
                    &format!("{base}/{}", escape_pointer_segment(key)),
                    pointers,
                );
            }
        }
        Value::Null | Value::Bool(_) | Value::Number(_) | Value::String(_) => {}
    }
}

fn escape_pointer_segment(segment: &str) -> String {
    segment.replace('~', "~0").replace('/', "~1")
}

#[path = "fixtures/account_overview_integration_fixture.rs"]
mod account_overview_integration_fixture;
#[path = "fixtures/pill_integration_fixture.rs"]
mod pill_integration_fixture;
#[path = "fixtures/status_dot_integration_fixture.rs"]
mod status_dot_integration_fixture;

#[test]
fn expected_block_fixtures_cover_requested_ci_block() {
    let Ok(requested) = env::var("DAILYOS_BLOCK_SLUG") else {
        return;
    };
    let requested = requested.trim();
    if requested.is_empty() {
        return;
    }
    let known = [
        account_overview_integration_fixture::account_overview_fixture(),
        pill_integration_fixture::pill_fixture(),
        status_dot_integration_fixture::status_dot_fixture(),
    ];
    assert!(
        known.iter().any(block_fixture_selected),
        "missing block integration fixture for wp/dailyos/blocks/{requested}/block.json"
    );
}
