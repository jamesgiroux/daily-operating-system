//! DOS-210 W3-A: Ability registry, AbilityContext, and typed/erased invocation.
//!
//! Per ADR-0102 §181-258. Type definitions consumed by the `#[ability]`
//! proc macro (W3-A part 3) for `inventory::submit!` registration.

use std::collections::{BTreeMap, HashMap, HashSet};
use std::fs;
use std::path::Path;
use std::sync::OnceLock;

use chrono::{DateTime, Utc};
use schemars::schema::{InstanceType, Schema, SchemaObject, SingleOrVec};
use schemars::{gen::SchemaGenerator, JsonSchema};
use serde::de::Error as _;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

use crate::abilities::provenance::{AbilityOutput, CompositionId};
use crate::services::context::{ExecutionMode, ServiceContext};

/// ADR-0102 §76-95: ability category drives mutation policy.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
pub enum AbilityCategory {
    Read,
    Transform,
    Publish,
    Maintenance,
}

/// Who is invoking. ADR-0102 §250-258.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
pub enum Actor {
    Agent,
    User,
    Admin,
    System,
}

/// Per-ability policy (which actors may invoke, which modes, etc.).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AbilityPolicy {
    pub allowed_actors: &'static [Actor],
    pub allowed_modes: &'static [ExecutionMode],
    pub requires_confirmation: bool,
    pub may_publish: bool,
}

/// Composition entry per descriptor.
#[derive(Debug, Clone, PartialEq)]
pub struct ComposesEntry {
    pub id: CompositionId,
    pub ability: &'static str,
    pub optional: bool,
}

/// Signal policy metadata for ADR-0115. W3-A records, does not emit.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SignalPolicy {
    pub emits_on_output_change: &'static [&'static str],
    pub coalesce: bool,
}

/// One ability's frozen description. The proc macro emits this via
/// inventory::submit! in part 3. For part 2 we define the shape and the
/// registry that collects it.
#[derive(Debug, Clone)]
pub struct AbilityDescriptor {
    pub name: &'static str,
    pub version: &'static str,
    pub schema_version: u32,
    pub category: AbilityCategory,
    pub policy: AbilityPolicy,
    pub composes: &'static [ComposesEntry],
    pub mutates: &'static [&'static str],
    pub experimental: bool,
    pub registered_at: Option<&'static str>,
    pub signal_policy: SignalPolicy,
    pub invoke_erased:
        fn(&AbilityContext<'_>, serde_json::Value) -> Result<serde_json::Value, AbilityError>,
    pub input_schema: fn() -> serde_json::Value,
    pub output_schema: fn() -> serde_json::Value,
}

inventory::collect!(AbilityDescriptor);

/// Confirmation token — Publish abilities require this for mutation auth.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConfirmationToken {
    pub source: String,
}

/// Ability error kinds — ADR-0102 Amendment A §466-483.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub enum AbilityErrorKind {
    Validation,
    Capability,
    OptionalComposedReadFailed {
        composition_id: CompositionId,
        reason: String,
    },
    HardError(String),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct AbilityError {
    pub kind: AbilityErrorKind,
    pub message: String,
}

pub type AbilityResult<T> = Result<AbilityOutput<T>, AbilityError>;

/// AbilityContext wraps ServiceContext and adds actor + confirmation.
///
/// DOS-304 hard boundary: this is the ONLY way ability code accesses runtime;
/// raw ActionDb / AppState / SQL handles / fs writers / live queues are NEVER
/// surfaced here.
pub struct AbilityContext<'a> {
    services: &'a ServiceContext<'a>,
    pub actor: Actor,
    pub confirmation: Option<&'a ConfirmationToken>,
}

impl<'a> AbilityContext<'a> {
    pub fn new(
        services: &'a ServiceContext<'a>,
        actor: Actor,
        confirmation: Option<&'a ConfirmationToken>,
    ) -> Self {
        Self {
            services,
            actor,
            confirmation,
        }
    }

    pub fn services(&self) -> &ServiceContext<'a> {
        self.services
    }

    pub fn mode(&self) -> ExecutionMode {
        self.services.mode
    }
}

/// Registry violations.
#[derive(Debug, Clone, PartialEq)]
pub enum RegistryViolation {
    DuplicateAbilityName(String),
    UnknownComposes {
        ability: String,
        target: String,
    },
    CompositionCycle(Vec<String>),
    CategoryViolation {
        ability: String,
        category: AbilityCategory,
        transitively_composes: AbilityCategory,
    },
    ExperimentalMissingRegisteredAt(String),
    ExperimentalExpired {
        ability: String,
        age_days: i64,
    },
    MetadataDrift {
        ability: String,
        observed: String,
        declared: String,
    },
}

#[derive(Debug)]
pub struct AbilityRegistry {
    by_name: HashMap<&'static str, AbilityDescriptor>,
}

impl AbilityRegistry {
    /// Collect from inventory and validate. Fails closed on any violation.
    pub fn from_inventory_checked() -> Result<Self, Vec<RegistryViolation>> {
        let descriptors = inventory::iter::<AbilityDescriptor>
            .into_iter()
            .cloned()
            .collect();
        Self::from_descriptors_checked(descriptors)
    }

    pub fn global_checked() -> Result<&'static Self, &'static [RegistryViolation]> {
        static REGISTRY: OnceLock<Result<AbilityRegistry, Vec<RegistryViolation>>> =
            OnceLock::new();
        match REGISTRY.get_or_init(Self::from_inventory_checked) {
            Ok(registry) => Ok(registry),
            Err(violations) => Err(violations.as_slice()),
        }
    }

    pub fn from_descriptors_checked(
        descriptors: Vec<AbilityDescriptor>,
    ) -> Result<Self, Vec<RegistryViolation>> {
        let mut violations = Vec::new();
        let mut by_name = HashMap::new();

        for descriptor in descriptors {
            if by_name.contains_key(descriptor.name) {
                violations.push(RegistryViolation::DuplicateAbilityName(
                    descriptor.name.to_string(),
                ));
            } else {
                by_name.insert(descriptor.name, descriptor);
            }
        }

        validate_unknown_composes(&by_name, &mut violations);
        let cycle_count_before = violations.len();
        validate_cycles(&by_name, &mut violations);
        let graph_has_hard_errors = violations[cycle_count_before..]
            .iter()
            .any(|violation| matches!(violation, RegistryViolation::CompositionCycle(_)))
            || violations
                .iter()
                .any(|violation| matches!(violation, RegistryViolation::UnknownComposes { .. }));
        if !graph_has_hard_errors {
            validate_category_transitivity(&by_name, &mut violations);
        }
        validate_experimental(&by_name, &mut violations);

        if violations.is_empty() {
            Ok(Self { by_name })
        } else {
            Err(violations)
        }
    }

    pub fn iter_for(&self, actor: Actor) -> impl Iterator<Item = &AbilityDescriptor> {
        self.by_name.values().filter(move |descriptor| {
            if descriptor.experimental && actor != Actor::System {
                return false;
            }
            if actor == Actor::Agent && descriptor.category == AbilityCategory::Maintenance {
                return false;
            }
            descriptor.policy.allowed_actors.contains(&actor)
        })
    }

    pub fn invoke_read(
        &self,
        ctx: &AbilityContext<'_>,
        name: &str,
        input: serde_json::Value,
    ) -> Result<serde_json::Value, AbilityError> {
        self.invoke_with_category(ctx, name, input, AbilityCategory::Read)
    }

    pub fn invoke_transform(
        &self,
        ctx: &AbilityContext<'_>,
        name: &str,
        input: serde_json::Value,
    ) -> Result<serde_json::Value, AbilityError> {
        self.invoke_with_category(ctx, name, input, AbilityCategory::Transform)
    }

    pub fn invoke_publish(
        &self,
        ctx: &AbilityContext<'_>,
        name: &str,
        input: serde_json::Value,
    ) -> Result<serde_json::Value, AbilityError> {
        self.invoke_with_category(ctx, name, input, AbilityCategory::Publish)
    }

    pub fn invoke_maintenance(
        &self,
        ctx: &AbilityContext<'_>,
        name: &str,
        input: serde_json::Value,
    ) -> Result<serde_json::Value, AbilityError> {
        self.invoke_with_category(ctx, name, input, AbilityCategory::Maintenance)
    }

    pub fn invoke_by_name_json(
        &self,
        ctx: &AbilityContext<'_>,
        name: &str,
        input: serde_json::Value,
    ) -> Result<serde_json::Value, AbilityError> {
        let descriptor = self.descriptor(name)?;
        validate_invocation_policy(ctx, descriptor)?;
        (descriptor.invoke_erased)(ctx, input)
    }

    /// Render docs to a directory. Deterministic key order, pretty JSON schemas.
    pub fn render_docs(&self, out_dir: &Path) -> std::io::Result<()> {
        fs::create_dir_all(out_dir)?;
        let descriptors: BTreeMap<&str, &AbilityDescriptor> = self
            .by_name
            .iter()
            .map(|(name, descriptor)| (*name, descriptor))
            .collect();

        for (name, descriptor) in descriptors {
            let input_schema = serde_json::to_string_pretty(&(descriptor.input_schema)())
                .unwrap_or_else(|_| "{}".to_string());
            let output_schema = serde_json::to_string_pretty(&(descriptor.output_schema)())
                .unwrap_or_else(|_| "{}".to_string());
            fs::write(
                out_dir.join(format!("{name}.md")),
                render_descriptor_doc(descriptor, &input_schema, &output_schema),
            )?;
        }
        Ok(())
    }

    fn invoke_with_category(
        &self,
        ctx: &AbilityContext<'_>,
        name: &str,
        input: serde_json::Value,
        expected_category: AbilityCategory,
    ) -> Result<serde_json::Value, AbilityError> {
        let descriptor = self.descriptor(name)?;
        if descriptor.category != expected_category {
            return Err(AbilityError {
                kind: AbilityErrorKind::Validation,
                message: format!(
                    "ability `{}` is {:?}, expected {:?}",
                    descriptor.name, descriptor.category, expected_category
                ),
            });
        }
        validate_invocation_policy(ctx, descriptor)?;
        (descriptor.invoke_erased)(ctx, input)
    }

    fn descriptor(&self, name: &str) -> Result<&AbilityDescriptor, AbilityError> {
        self.by_name.get(name).ok_or_else(|| AbilityError {
            kind: AbilityErrorKind::Validation,
            message: format!("unknown ability `{name}`"),
        })
    }
}

impl Serialize for ExecutionMode {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(self.as_str())
    }
}

impl<'de> Deserialize<'de> for ExecutionMode {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;
        match value.as_str() {
            "live" | "Live" => Ok(ExecutionMode::Live),
            "simulate" | "Simulate" => Ok(ExecutionMode::Simulate),
            "evaluate" | "Evaluate" => Ok(ExecutionMode::Evaluate),
            other => Err(D::Error::custom(format!("unknown execution mode `{other}`"))),
        }
    }
}

impl JsonSchema for ExecutionMode {
    fn schema_name() -> String {
        "ExecutionMode".to_string()
    }

    fn json_schema(_gen: &mut SchemaGenerator) -> Schema {
        let mut schema = SchemaObject {
            instance_type: Some(SingleOrVec::Single(Box::new(InstanceType::String))),
            ..Default::default()
        };
        schema.enum_values = Some(vec![
            serde_json::json!("live"),
            serde_json::json!("simulate"),
            serde_json::json!("evaluate"),
        ]);
        Schema::Object(schema)
    }
}

fn validate_unknown_composes(
    by_name: &HashMap<&'static str, AbilityDescriptor>,
    violations: &mut Vec<RegistryViolation>,
) {
    for descriptor in descriptors_sorted(by_name) {
        for entry in descriptor.composes {
            if !by_name.contains_key(entry.ability) {
                violations.push(RegistryViolation::UnknownComposes {
                    ability: descriptor.name.to_string(),
                    target: entry.ability.to_string(),
                });
            }
        }
    }
}

fn validate_cycles(
    by_name: &HashMap<&'static str, AbilityDescriptor>,
    violations: &mut Vec<RegistryViolation>,
) {
    #[derive(Clone, Copy, PartialEq, Eq)]
    enum Color {
        Unvisited,
        Visiting,
        Done,
    }

    fn visit(
        name: &'static str,
        by_name: &HashMap<&'static str, AbilityDescriptor>,
        color: &mut HashMap<&'static str, Color>,
        stack: &mut Vec<&'static str>,
        violations: &mut Vec<RegistryViolation>,
    ) {
        color.insert(name, Color::Visiting);
        stack.push(name);

        if let Some(descriptor) = by_name.get(name) {
            let mut targets: Vec<&'static str> = descriptor
                .composes
                .iter()
                .filter_map(|entry| by_name.get(entry.ability).map(|target| target.name))
                .collect();
            targets.sort_unstable();

            for target in targets {
                match color.get(target).copied().unwrap_or(Color::Unvisited) {
                    Color::Unvisited => visit(target, by_name, color, stack, violations),
                    Color::Visiting => {
                        if let Some(pos) = stack.iter().position(|stacked| *stacked == target) {
                            let mut cycle: Vec<String> =
                                stack[pos..].iter().map(|entry| (*entry).to_string()).collect();
                            cycle.push(target.to_string());
                            violations.push(RegistryViolation::CompositionCycle(cycle));
                        }
                    }
                    Color::Done => {}
                }
            }
        }

        stack.pop();
        color.insert(name, Color::Done);
    }

    let mut color: HashMap<&'static str, Color> = by_name
        .keys()
        .map(|name| (*name, Color::Unvisited))
        .collect();
    let mut stack = Vec::new();

    for name in names_sorted(by_name) {
        if matches!(color.get(name), Some(Color::Unvisited)) {
            visit(name, by_name, &mut color, &mut stack, violations);
        }
    }
}

fn validate_category_transitivity(
    by_name: &HashMap<&'static str, AbilityDescriptor>,
    violations: &mut Vec<RegistryViolation>,
) {
    for descriptor in descriptors_sorted(by_name) {
        let sensitive_category = matches!(
            descriptor.category,
            AbilityCategory::Read | AbilityCategory::Transform
        );

        if sensitive_category && !descriptor.mutates.is_empty() {
            violations.push(RegistryViolation::CategoryViolation {
                ability: descriptor.name.to_string(),
                category: descriptor.category,
                transitively_composes: descriptor.category,
            });
            continue;
        }

        for descendant_name in descendant_names(descriptor.name, by_name) {
            let descendant = &by_name[descendant_name];
            if sensitive_category
                && (matches!(
                    descendant.category,
                    AbilityCategory::Publish | AbilityCategory::Maintenance
                ) || !descendant.mutates.is_empty())
            {
                violations.push(RegistryViolation::CategoryViolation {
                    ability: descriptor.name.to_string(),
                    category: descriptor.category,
                    transitively_composes: descendant.category,
                });
                break;
            }

            if descriptor.category == AbilityCategory::Maintenance
                && !descriptor.policy.may_publish
                && descendant.category == AbilityCategory::Publish
            {
                violations.push(RegistryViolation::CategoryViolation {
                    ability: descriptor.name.to_string(),
                    category: descriptor.category,
                    transitively_composes: descendant.category,
                });
                break;
            }
        }
    }
}

fn validate_experimental(
    by_name: &HashMap<&'static str, AbilityDescriptor>,
    violations: &mut Vec<RegistryViolation>,
) {
    for descriptor in descriptors_sorted(by_name) {
        if !descriptor.experimental {
            continue;
        }

        let Some(registered_at) = descriptor.registered_at else {
            violations.push(RegistryViolation::ExperimentalMissingRegisteredAt(
                descriptor.name.to_string(),
            ));
            continue;
        };

        let Ok(parsed) = DateTime::parse_from_rfc3339(registered_at) else {
            violations.push(RegistryViolation::ExperimentalMissingRegisteredAt(
                descriptor.name.to_string(),
            ));
            continue;
        };

        let age_days = Utc::now()
            .signed_duration_since(parsed.with_timezone(&Utc))
            .num_days();
        if age_days > 90 {
            violations.push(RegistryViolation::ExperimentalExpired {
                ability: descriptor.name.to_string(),
                age_days,
            });
        }
    }
}

fn validate_invocation_policy(
    ctx: &AbilityContext<'_>,
    descriptor: &AbilityDescriptor,
) -> Result<(), AbilityError> {
    if !descriptor.policy.allowed_actors.contains(&ctx.actor) {
        return Err(AbilityError {
            kind: AbilityErrorKind::Capability,
            message: format!(
                "actor {:?} is not allowed to invoke `{}`",
                ctx.actor, descriptor.name
            ),
        });
    }

    if !descriptor.policy.allowed_modes.contains(&ctx.mode()) {
        return Err(AbilityError {
            kind: AbilityErrorKind::Capability,
            message: format!(
                "mode {:?} is not allowed to invoke `{}`",
                ctx.mode(),
                descriptor.name
            ),
        });
    }

    let requires_confirmation =
        descriptor.policy.requires_confirmation || descriptor.category == AbilityCategory::Publish;
    if requires_confirmation && ctx.confirmation.is_none() {
        return Err(AbilityError {
            kind: AbilityErrorKind::Capability,
            message: format!("ability `{}` requires confirmation", descriptor.name),
        });
    }

    Ok(())
}

fn descendant_names(
    name: &'static str,
    by_name: &HashMap<&'static str, AbilityDescriptor>,
) -> Vec<&'static str> {
    fn walk(
        current: &'static str,
        by_name: &HashMap<&'static str, AbilityDescriptor>,
        seen: &mut HashSet<&'static str>,
        out: &mut Vec<&'static str>,
    ) {
        let Some(descriptor) = by_name.get(current) else {
            return;
        };

        let mut targets: Vec<&'static str> = descriptor
            .composes
            .iter()
            .filter_map(|entry| by_name.get(entry.ability).map(|target| target.name))
            .collect();
        targets.sort_unstable();

        for target in targets {
            if seen.insert(target) {
                out.push(target);
                walk(target, by_name, seen, out);
            }
        }
    }

    let mut seen = HashSet::new();
    let mut out = Vec::new();
    walk(name, by_name, &mut seen, &mut out);
    out
}

fn descriptors_sorted<'a>(
    by_name: &'a HashMap<&'static str, AbilityDescriptor>,
) -> Vec<&'a AbilityDescriptor> {
    let mut descriptors: Vec<&'a AbilityDescriptor> = by_name.values().collect();
    descriptors.sort_by_key(|descriptor| descriptor.name);
    descriptors
}

fn names_sorted(by_name: &HashMap<&'static str, AbilityDescriptor>) -> Vec<&'static str> {
    let mut names: Vec<&'static str> = by_name.keys().copied().collect();
    names.sort_unstable();
    names
}

fn render_descriptor_doc(
    descriptor: &AbilityDescriptor,
    input_schema: &str,
    output_schema: &str,
) -> String {
    let mut out = String::new();
    out.push_str("---\n");
    push_yaml_string(&mut out, "name", descriptor.name);
    push_yaml_string(&mut out, "version", descriptor.version);
    out.push_str(&format!("schema_version: {}\n", descriptor.schema_version));
    out.push_str(&format!("category: {:?}\n", descriptor.category));
    out.push_str(&format!("experimental: {}\n", descriptor.experimental));
    push_yaml_string_list(
        &mut out,
        "allowed_actors",
        descriptor
            .policy
            .allowed_actors
            .iter()
            .map(|actor| format!("{actor:?}")),
    );
    push_yaml_string_list(
        &mut out,
        "allowed_modes",
        descriptor
            .policy
            .allowed_modes
            .iter()
            .map(|mode| mode.as_str().to_string()),
    );
    out.push_str(&format!(
        "requires_confirmation: {}\n",
        descriptor.policy.requires_confirmation
    ));
    out.push_str(&format!("may_publish: {}\n", descriptor.policy.may_publish));
    push_yaml_string_list(
        &mut out,
        "mutates",
        descriptor.mutates.iter().map(|value| (*value).to_string()),
    );
    out.push_str("composes:");
    if descriptor.composes.is_empty() {
        out.push_str(" []\n");
    } else {
        out.push('\n');
        for entry in descriptor.composes {
            out.push_str(&format!("  - id: {}\n", yaml_string(entry.id.as_str())));
            out.push_str(&format!("    ability: {}\n", yaml_string(entry.ability)));
            out.push_str(&format!("    optional: {}\n", entry.optional));
        }
    }
    out.push_str("signal_policy:\n");
    push_yaml_string_list_indented(
        &mut out,
        "emits_on_output_change",
        descriptor
            .signal_policy
            .emits_on_output_change
            .iter()
            .map(|value| (*value).to_string()),
        "  ",
    );
    out.push_str(&format!(
        "  coalesce: {}\n",
        descriptor.signal_policy.coalesce
    ));
    out.push_str("---\n\n");
    out.push_str(&format!("# {}\n\n", descriptor.name));
    out.push_str("## Policy\n\n");
    out.push_str(&format!("- Category: `{:?}`\n", descriptor.category));
    out.push_str(&format!(
        "- Requires confirmation: `{}`\n",
        descriptor.policy.requires_confirmation
    ));
    out.push_str(&format!(
        "- May publish from maintenance: `{}`\n\n",
        descriptor.policy.may_publish
    ));
    out.push_str("## Input Schema\n\n```json\n");
    out.push_str(input_schema);
    out.push_str("\n```\n\n## Output Schema\n\n```json\n");
    out.push_str(output_schema);
    out.push_str("\n```\n\n## Composition And Mutation Notes\n\n");
    out.push_str(&format!(
        "- Composes: `{}` entries\n",
        descriptor.composes.len()
    ));
    out.push_str(&format!("- Mutates: `{}` entries\n", descriptor.mutates.len()));
    out
}

fn push_yaml_string(out: &mut String, key: &str, value: &str) {
    out.push_str(&format!("{key}: {}\n", yaml_string(value)));
}

fn push_yaml_string_list<I>(out: &mut String, key: &str, values: I)
where
    I: IntoIterator<Item = String>,
{
    push_yaml_string_list_indented(out, key, values, "");
}

fn push_yaml_string_list_indented<I>(out: &mut String, key: &str, values: I, indent: &str)
where
    I: IntoIterator<Item = String>,
{
    let values: Vec<String> = values.into_iter().collect();
    out.push_str(indent);
    out.push_str(key);
    out.push(':');
    if values.is_empty() {
        out.push_str(" []\n");
    } else {
        out.push('\n');
        for value in values {
            out.push_str(indent);
            out.push_str("  - ");
            out.push_str(&yaml_string(&value));
            out.push('\n');
        }
    }
}

fn yaml_string(value: &str) -> String {
    serde_json::to_string(value).expect("serializing a string to JSON cannot fail")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::services::context::{ExternalClients, FixedClock, SeedableRng};
    use chrono::TimeZone;

    fn ok_erased(
        _ctx: &AbilityContext<'_>,
        input: serde_json::Value,
    ) -> Result<serde_json::Value, AbilityError> {
        Ok(input)
    }

    fn empty_schema() -> serde_json::Value {
        serde_json::json!({ "type": "object" })
    }

    inventory::submit! {
        AbilityDescriptor {
            name: "dos210_inventory_fixture",
            version: "0.1.0",
            schema_version: 1,
            category: AbilityCategory::Read,
            policy: AbilityPolicy {
                allowed_actors: &[],
                allowed_modes: &[],
                requires_confirmation: false,
                may_publish: false,
            },
            composes: &[],
            mutates: &[],
            experimental: false,
            registered_at: None,
            signal_policy: SignalPolicy {
                emits_on_output_change: &[],
                coalesce: false,
            },
            invoke_erased: ok_erased,
            input_schema: empty_schema,
            output_schema: empty_schema,
        }
    }

    fn descriptor(name: &'static str, category: AbilityCategory) -> AbilityDescriptor {
        AbilityDescriptor {
            name,
            version: "0.1.0",
            schema_version: 1,
            category,
            policy: AbilityPolicy {
                allowed_actors: &[Actor::Agent, Actor::User, Actor::System],
                allowed_modes: &[
                    ExecutionMode::Live,
                    ExecutionMode::Simulate,
                    ExecutionMode::Evaluate,
                ],
                requires_confirmation: false,
                may_publish: false,
            },
            composes: &[],
            mutates: &[],
            experimental: false,
            registered_at: None,
            signal_policy: SignalPolicy::default(),
            invoke_erased: ok_erased,
            input_schema: empty_schema,
            output_schema: empty_schema,
        }
    }

    fn static_slice<T>(values: Vec<T>) -> &'static [T] {
        Box::leak(values.into_boxed_slice())
    }

    fn push_compose(descriptor: &mut AbilityDescriptor, entry: ComposesEntry) {
        let mut composes = descriptor.composes.to_vec();
        composes.push(entry);
        descriptor.composes = static_slice(composes);
    }

    fn compose(mut descriptor: AbilityDescriptor, target: &'static str) -> AbilityDescriptor {
        let id = CompositionId::new(format!("{}_to_{target}", descriptor.name));
        push_compose(&mut descriptor, ComposesEntry {
            id,
            ability: target,
            optional: false,
        });
        descriptor
    }

    fn with_mutates(
        mut descriptor: AbilityDescriptor,
        mutates: Vec<&'static str>,
    ) -> AbilityDescriptor {
        descriptor.mutates = static_slice(mutates);
        descriptor
    }

    fn with_actor_policy(mut descriptor: AbilityDescriptor, actors: Vec<Actor>) -> AbilityDescriptor {
        descriptor.policy.allowed_actors = static_slice(actors);
        descriptor
    }

    fn experimental(mut descriptor: AbilityDescriptor, registered_at: &'static str) -> AbilityDescriptor {
        descriptor.experimental = true;
        descriptor.registered_at = Some(registered_at);
        descriptor
    }

    fn registry(descriptors: Vec<AbilityDescriptor>) -> AbilityRegistry {
        AbilityRegistry::from_descriptors_checked(descriptors).unwrap()
    }

    fn context<'a>(
        services: &'a ServiceContext<'a>,
        actor: Actor,
        confirmation: Option<&'a ConfirmationToken>,
    ) -> AbilityContext<'a> {
        AbilityContext::new(services, actor, confirmation)
    }

    fn services<'a>(
        clock: &'a FixedClock,
        rng: &'a SeedableRng,
        external: &'a ExternalClients,
    ) -> ServiceContext<'a> {
        ServiceContext::test_live(clock, rng, external)
    }

    fn static_name(prefix: &str, case: usize, index: usize) -> &'static str {
        Box::leak(format!("{prefix}_{case}_{index}").into_boxed_str())
    }

    fn lcg(seed: &mut u64) -> u64 {
        *seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1);
        *seed
    }

    #[test]
    fn registry_collects_inventory_descriptors() {
        let registry = AbilityRegistry::from_inventory_checked().unwrap();

        assert!(registry.by_name.contains_key("dos210_inventory_fixture"));
    }

    #[test]
    fn registry_rejects_duplicate_names_with_clear_error() {
        let violations = AbilityRegistry::from_descriptors_checked(vec![
            descriptor("duplicate", AbilityCategory::Read),
            descriptor("duplicate", AbilityCategory::Transform),
        ])
        .unwrap_err();

        assert!(violations.contains(&RegistryViolation::DuplicateAbilityName(
            "duplicate".to_string()
        )));
    }

    #[test]
    fn registry_rejects_unknown_composes() {
        let violations = AbilityRegistry::from_descriptors_checked(vec![compose(
            descriptor("reader", AbilityCategory::Read),
            "missing",
        )])
        .unwrap_err();

        assert!(violations.contains(&RegistryViolation::UnknownComposes {
            ability: "reader".to_string(),
            target: "missing".to_string(),
        }));
    }

    #[test]
    fn registry_rejects_read_composing_publish_transitively() {
        let violations = AbilityRegistry::from_descriptors_checked(vec![
            compose(descriptor("read", AbilityCategory::Read), "transform"),
            compose(descriptor("transform", AbilityCategory::Transform), "publish"),
            descriptor("publish", AbilityCategory::Publish),
        ])
        .unwrap_err();

        assert!(violations.iter().any(|violation| matches!(
            violation,
            RegistryViolation::CategoryViolation {
                ability,
                category: AbilityCategory::Read,
                transitively_composes: AbilityCategory::Publish,
            } if ability == "read"
        )));
    }

    #[test]
    fn registry_rejects_transform_composing_maintenance_transitively() {
        let violations = AbilityRegistry::from_descriptors_checked(vec![
            compose(descriptor("transform", AbilityCategory::Transform), "read"),
            compose(descriptor("read", AbilityCategory::Read), "maintenance"),
            descriptor("maintenance", AbilityCategory::Maintenance),
        ])
        .unwrap_err();

        assert!(violations.iter().any(|violation| matches!(
            violation,
            RegistryViolation::CategoryViolation {
                ability,
                category: AbilityCategory::Transform,
                transitively_composes: AbilityCategory::Maintenance,
            } if ability == "transform"
        )));
    }

    #[test]
    fn registry_iter_for_agent_hides_maintenance_admin_and_experimental() {
        let registry = registry(vec![
            descriptor("agent_read", AbilityCategory::Read),
            descriptor("agent_maintenance", AbilityCategory::Maintenance),
            with_actor_policy(descriptor("admin_read", AbilityCategory::Read), vec![Actor::Admin]),
            experimental(
                descriptor("experimental_read", AbilityCategory::Read),
                "2999-01-01T00:00:00Z",
            ),
        ]);

        let names: HashSet<&str> = registry
            .iter_for(Actor::Agent)
            .map(|descriptor| descriptor.name)
            .collect();

        assert!(names.contains("agent_read"));
        assert!(!names.contains("agent_maintenance"));
        assert!(!names.contains("admin_read"));
        assert!(!names.contains("experimental_read"));
    }

    #[test]
    fn composition_graph_accepts_random_dag() {
        let mut seed = 7;
        for case in 0..100 {
            let names: Vec<&'static str> = (0..8)
                .map(|index| static_name("dag", case, index))
                .collect();
            let mut descriptors: Vec<AbilityDescriptor> = names
                .iter()
                .map(|name| descriptor(name, AbilityCategory::Read))
                .collect();

            for i in 0..descriptors.len() {
                for target in (i + 1)..descriptors.len() {
                    if lcg(&mut seed) % 4 == 0 {
                        push_compose(&mut descriptors[i], ComposesEntry {
                            id: CompositionId::new(format!("{i}_{target}")),
                            ability: names[target],
                            optional: false,
                        });
                    }
                }
            }

            AbilityRegistry::from_descriptors_checked(descriptors).unwrap();
        }
    }

    #[test]
    fn composition_graph_rejects_random_cycle() {
        let mut seed = 11;
        for case in 0..100 {
            let names: Vec<&'static str> = (0..6)
                .map(|index| static_name("cycle", case, index))
                .collect();
            let mut descriptors: Vec<AbilityDescriptor> = names
                .iter()
                .map(|name| descriptor(name, AbilityCategory::Read))
                .collect();

            for i in 0..descriptors.len() {
                let target = (i + 1) % descriptors.len();
                push_compose(&mut descriptors[i], ComposesEntry {
                    id: CompositionId::new(format!("{i}_{target}")),
                    ability: names[target],
                    optional: false,
                });
                if lcg(&mut seed) % 3 == 0 {
                    let extra = ((lcg(&mut seed) as usize) % descriptors.len()).max(i);
                    push_compose(&mut descriptors[i], ComposesEntry {
                        id: CompositionId::new(format!("{i}_{extra}_extra")),
                        ability: names[extra],
                        optional: false,
                    });
                }
            }

            let violations = AbilityRegistry::from_descriptors_checked(descriptors).unwrap_err();
            assert!(violations
                .iter()
                .any(|violation| matches!(violation, RegistryViolation::CompositionCycle(_))));
        }
    }

    #[test]
    fn composition_graph_folds_transitive_mutation_sets() {
        let violations = AbilityRegistry::from_descriptors_checked(vec![
            compose(descriptor("read", AbilityCategory::Read), "child"),
            with_mutates(
                descriptor("child", AbilityCategory::Read),
                vec!["services::claims::commit_claim"],
            ),
        ])
        .unwrap_err();

        assert!(violations.iter().any(|violation| matches!(
            violation,
            RegistryViolation::CategoryViolation {
                ability,
                category: AbilityCategory::Read,
                transitively_composes: AbilityCategory::Read,
            } if ability == "read"
        )));
    }

    #[test]
    fn experimental_expiry_rejects_over_90_days() {
        let violations = AbilityRegistry::from_descriptors_checked(vec![experimental(
            descriptor("old_experiment", AbilityCategory::Read),
            "2020-01-01T00:00:00Z",
        )])
        .unwrap_err();

        assert!(violations.iter().any(|violation| matches!(
            violation,
            RegistryViolation::ExperimentalExpired { ability, age_days }
                if ability == "old_experiment" && *age_days > 90
        )));
    }

    #[test]
    fn invoke_read_rejects_transform_descriptor() {
        let registry = registry(vec![descriptor("transform", AbilityCategory::Transform)]);
        let clock = FixedClock::new(Utc.with_ymd_and_hms(2026, 5, 1, 12, 0, 0).unwrap());
        let rng = SeedableRng::new(42);
        let external = ExternalClients::default();
        let services = services(&clock, &rng, &external);
        let ctx = context(&services, Actor::User, None);

        let err = registry
            .invoke_read(&ctx, "transform", serde_json::json!({}))
            .unwrap_err();

        assert_eq!(err.kind, AbilityErrorKind::Validation);
    }

    #[test]
    fn publish_requires_confirmation_token() {
        let registry = registry(vec![descriptor("publish", AbilityCategory::Publish)]);
        let clock = FixedClock::new(Utc.with_ymd_and_hms(2026, 5, 1, 12, 0, 0).unwrap());
        let rng = SeedableRng::new(42);
        let external = ExternalClients::default();
        let services = services(&clock, &rng, &external);
        let ctx = context(&services, Actor::User, None);

        let err = registry
            .invoke_publish(&ctx, "publish", serde_json::json!({}))
            .unwrap_err();

        assert_eq!(err.kind, AbilityErrorKind::Capability);
    }
}
