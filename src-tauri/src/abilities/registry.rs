//! Ability registry, AbilityContext, and typed/erased invocation.
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
use crate::abilities::tracer::AbilityTracer;
use crate::bridges::types::{BridgeActor, ConfirmationToken};
use crate::intelligence::provider::IntelligenceProvider;
use crate::services::context::{ExecutionMode, ServiceContext};

const UNKNOWN_SCHEMA_ABILITY: &str = "<unknown>";

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
    pub invoke_erased: for<'a> fn(
        &'a AbilityContext<'a>,
        serde_json::Value,
    ) -> std::pin::Pin<
        Box<dyn std::future::Future<Output = Result<serde_json::Value, AbilityError>> + Send + 'a>,
    >,
    pub input_schema: fn() -> serde_json::Value,
    pub output_schema: fn() -> serde_json::Value,
}

inventory::collect!(AbilityDescriptor);

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

/// AbilityContext wraps ServiceContext and adds provider/tracer seams,
/// actor, and confirmation.
///
///  hard boundary: this is the ONLY way ability code accesses runtime;
/// raw ActionDb / AppState / SQL handles / fs writers / live queues are NEVER
/// surfaced here.
pub struct AbilityContext<'a> {
    services: &'a ServiceContext<'a>,
    pub provider: &'a dyn IntelligenceProvider,
    pub tracer: &'a dyn AbilityTracer,
    pub actor: Actor,
    pub confirmation: Option<&'a ConfirmationToken>,
}

impl<'a> AbilityContext<'a> {
    pub fn new(
        services: &'a ServiceContext<'a>,
        provider: &'a dyn IntelligenceProvider,
        tracer: &'a dyn AbilityTracer,
        actor: Actor,
        confirmation: Option<&'a ConfirmationToken>,
    ) -> Self {
        Self {
            services,
            provider,
            tracer,
            actor,
            confirmation,
        }
    }

    pub fn from_bridge(
        services: &'a ServiceContext<'a>,
        provider: &'a dyn IntelligenceProvider,
        tracer: &'a dyn AbilityTracer,
        actor: BridgeActor,
        confirmation: Option<&'a ConfirmationToken>,
    ) -> Self {
        Self::new(
            services,
            provider,
            tracer,
            actor.registry_actor(),
            confirmation,
        )
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
    SchemaClosure(SchemaClosureError),
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
    ExperimentalInProduction,
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

        validate_descriptor_schema_closures(&descriptors, &mut violations);

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

    #[cfg(any(test, feature = "mcp"))]
    #[doc(hidden)]
    pub fn from_descriptors_unchecked_for_runtime_validation_tests(
        descriptors: Vec<AbilityDescriptor>,
    ) -> Self {
        Self {
            by_name: descriptors
                .into_iter()
                .map(|descriptor| (descriptor.name, descriptor))
                .collect(),
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

    pub async fn invoke_read(
        &self,
        ctx: &AbilityContext<'_>,
        name: &str,
        input: serde_json::Value,
    ) -> Result<serde_json::Value, AbilityError> {
        self.invoke_with_category(ctx, name, input, AbilityCategory::Read)
            .await
    }

    pub async fn invoke_transform(
        &self,
        ctx: &AbilityContext<'_>,
        name: &str,
        input: serde_json::Value,
    ) -> Result<serde_json::Value, AbilityError> {
        self.invoke_with_category(ctx, name, input, AbilityCategory::Transform)
            .await
    }

    pub async fn invoke_publish(
        &self,
        ctx: &AbilityContext<'_>,
        name: &str,
        input: serde_json::Value,
    ) -> Result<serde_json::Value, AbilityError> {
        self.invoke_with_category(ctx, name, input, AbilityCategory::Publish)
            .await
    }

    pub async fn invoke_maintenance(
        &self,
        ctx: &AbilityContext<'_>,
        name: &str,
        input: serde_json::Value,
    ) -> Result<serde_json::Value, AbilityError> {
        self.invoke_with_category(ctx, name, input, AbilityCategory::Maintenance)
            .await
    }

    pub async fn invoke_by_name_json(
        &self,
        ctx: &AbilityContext<'_>,
        name: &str,
        input: serde_json::Value,
    ) -> Result<serde_json::Value, AbilityError> {
        let descriptor = self.descriptor(name)?;
        validate_invocation_policy(ctx, descriptor)?;
        (descriptor.invoke_erased)(ctx, input).await
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

    async fn invoke_with_category(
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
        (descriptor.invoke_erased)(ctx, input).await
    }

    fn descriptor(&self, name: &str) -> Result<&AbilityDescriptor, AbilityError> {
        self.by_name.get(name).ok_or_else(|| AbilityError {
            kind: AbilityErrorKind::Validation,
            message: format!("unknown ability `{name}`"),
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SchemaClosureError {
    pub ability_name: String,
    pub pointer: String,
}

impl SchemaClosureError {
    fn new(ability_name: impl Into<String>, pointer: impl Into<String>) -> Self {
        Self {
            ability_name: ability_name.into(),
            pointer: pointer.into(),
        }
    }
}

impl std::fmt::Display for SchemaClosureError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let pointer = if self.pointer.is_empty() {
            "<root>"
        } else {
            self.pointer.as_str()
        };
        write!(
            formatter,
            "ability `{}` input schema object at `{}` must set additionalProperties: false",
            self.ability_name, pointer
        )
    }
}

impl std::error::Error for SchemaClosureError {}

pub fn validate_schema_closure(schema: &serde_json::Value) -> Result<(), SchemaClosureError> {
    validate_schema_closure_for_ability(UNKNOWN_SCHEMA_ABILITY, schema)
}

pub fn validate_schema_closure_for_ability(
    ability_name: &str,
    schema: &serde_json::Value,
) -> Result<(), SchemaClosureError> {
    validate_schema_closure_at(schema, "", ability_name)
}

pub fn close_schema_objects(schema: &mut serde_json::Value) {
    close_schema_objects_at(schema);
}

fn validate_descriptor_schema_closures(
    descriptors: &[AbilityDescriptor],
    violations: &mut Vec<RegistryViolation>,
) {
    for descriptor in descriptors {
        if let Err(error) =
            validate_schema_closure_for_ability(descriptor.name, &(descriptor.input_schema)())
        {
            violations.push(RegistryViolation::SchemaClosure(error));
        }
    }
}

fn validate_schema_closure_at(
    schema: &serde_json::Value,
    pointer: &str,
    ability_name: &str,
) -> Result<(), SchemaClosureError> {
    let Some(object) = schema.as_object() else {
        return Ok(());
    };

    if is_object_schema(object)
        && object.get("additionalProperties") != Some(&serde_json::Value::Bool(false))
    {
        return Err(SchemaClosureError::new(ability_name, pointer));
    }

    walk_schema_children(object, pointer, |child, child_pointer| {
        validate_schema_closure_at(child, &child_pointer, ability_name)
    })
}

fn close_schema_objects_at(schema: &mut serde_json::Value) {
    let Some(object) = schema.as_object_mut() else {
        return;
    };

    if is_object_schema(object) {
        object.insert(
            "additionalProperties".to_string(),
            serde_json::Value::Bool(false),
        );
    }

    walk_schema_children_mut(object);
}

fn is_object_schema(object: &serde_json::Map<String, serde_json::Value>) -> bool {
    has_object_type(object) || (object.get("type").is_none() && object.contains_key("properties"))
}

fn has_object_type(object: &serde_json::Map<String, serde_json::Value>) -> bool {
    match object.get("type") {
        Some(serde_json::Value::String(schema_type)) => schema_type == "object",
        Some(serde_json::Value::Array(schema_types)) => schema_types
            .iter()
            .any(|schema_type| schema_type.as_str() == Some("object")),
        _ => false,
    }
}

fn walk_schema_children<F>(
    object: &serde_json::Map<String, serde_json::Value>,
    pointer: &str,
    mut walk: F,
) -> Result<(), SchemaClosureError>
where
    F: FnMut(&serde_json::Value, String) -> Result<(), SchemaClosureError>,
{
    for keyword in [
        "properties",
        "patternProperties",
        "definitions",
        "$defs",
        "dependentSchemas",
    ] {
        if let Some(serde_json::Value::Object(children)) = object.get(keyword) {
            for (name, child) in children {
                walk(child, pointer_child(pointer, keyword, name))?;
            }
        }
    }

    for keyword in [
        "items",
        "additionalItems",
        "contains",
        "propertyNames",
        "not",
        "if",
        "then",
        "else",
    ] {
        if let Some(child) = object.get(keyword) {
            walk_schema_or_schema_array(child, &pointer_segment(pointer, keyword), &mut walk)?;
        }
    }

    for keyword in ["oneOf", "anyOf", "allOf", "prefixItems"] {
        if let Some(child) = object.get(keyword) {
            walk_schema_array(child, &pointer_segment(pointer, keyword), &mut walk)?;
        }
    }

    Ok(())
}

fn walk_schema_or_schema_array<F>(
    value: &serde_json::Value,
    pointer: &str,
    walk: &mut F,
) -> Result<(), SchemaClosureError>
where
    F: FnMut(&serde_json::Value, String) -> Result<(), SchemaClosureError>,
{
    match value {
        serde_json::Value::Array(items) => {
            for (index, item) in items.iter().enumerate() {
                walk(item, pointer_segment(pointer, &index.to_string()))?;
            }
            Ok(())
        }
        _ => walk(value, pointer.to_string()),
    }
}

fn walk_schema_array<F>(
    value: &serde_json::Value,
    pointer: &str,
    walk: &mut F,
) -> Result<(), SchemaClosureError>
where
    F: FnMut(&serde_json::Value, String) -> Result<(), SchemaClosureError>,
{
    let serde_json::Value::Array(items) = value else {
        return Ok(());
    };

    for (index, item) in items.iter().enumerate() {
        walk(item, pointer_segment(pointer, &index.to_string()))?;
    }

    Ok(())
}

fn walk_schema_children_mut(object: &mut serde_json::Map<String, serde_json::Value>) {
    for keyword in [
        "properties",
        "patternProperties",
        "definitions",
        "$defs",
        "dependentSchemas",
    ] {
        if let Some(serde_json::Value::Object(children)) = object.get_mut(keyword) {
            for child in children.values_mut() {
                close_schema_objects_at(child);
            }
        }
    }

    for keyword in [
        "items",
        "additionalItems",
        "contains",
        "propertyNames",
        "not",
        "if",
        "then",
        "else",
    ] {
        if let Some(child) = object.get_mut(keyword) {
            close_schema_or_schema_array(child);
        }
    }

    for keyword in ["oneOf", "anyOf", "allOf", "prefixItems"] {
        if let Some(serde_json::Value::Array(children)) = object.get_mut(keyword) {
            for child in children {
                close_schema_objects_at(child);
            }
        }
    }
}

fn close_schema_or_schema_array(value: &mut serde_json::Value) {
    match value {
        serde_json::Value::Array(items) => {
            for item in items {
                close_schema_objects_at(item);
            }
        }
        _ => close_schema_objects_at(value),
    }
}

fn pointer_child(pointer: &str, keyword: &str, child: &str) -> String {
    pointer_segment(&pointer_segment(pointer, keyword), child)
}

fn pointer_segment(pointer: &str, segment: &str) -> String {
    let escaped = segment.replace('~', "~0").replace('/', "~1");
    if pointer.is_empty() {
        format!("/{escaped}")
    } else {
        format!("{pointer}/{escaped}")
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
            other => Err(D::Error::custom(format!(
                "unknown execution mode `{other}`"
            ))),
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
                            let mut cycle: Vec<String> = stack[pos..]
                                .iter()
                                .map(|entry| (*entry).to_string())
                                .collect();
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

        if !cfg!(feature = "experimental") {
            violations.push(RegistryViolation::ExperimentalInProduction);
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
    out.push_str(&format!(
        "- Mutates: `{}` entries\n",
        descriptor.mutates.len()
    ));
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
#[allow(clippy::manual_is_multiple_of, clippy::needless_range_loop)]
mod tests {
    use super::*;
    use crate::abilities::tracer::{AbilityTracer, SpanHandle};
    use crate::bridges::BridgeActor;
    use crate::intelligence::provider::{ModelName, ModelTier, ProviderKind, ReplayProvider};
    use crate::services::context::{ExternalClients, FixedClock, SeedableRng};
    use chrono::TimeZone;
    use std::sync::Mutex;

    fn ok_erased<'a>(
        _ctx: &'a AbilityContext<'a>,
        input: serde_json::Value,
    ) -> std::pin::Pin<
        Box<dyn std::future::Future<Output = Result<serde_json::Value, AbilityError>> + Send + 'a>,
    > {
        Box::pin(async move { Ok(input) })
    }

    fn empty_schema() -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "additionalProperties": false
        })
    }

    fn open_object_schema() -> serde_json::Value {
        serde_json::json!({ "type": "object" })
    }

    fn with_input_schema(
        mut descriptor: AbilityDescriptor,
        input_schema: fn() -> serde_json::Value,
    ) -> AbilityDescriptor {
        descriptor.input_schema = input_schema;
        descriptor
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
        push_compose(
            &mut descriptor,
            ComposesEntry {
                id,
                ability: target,
                optional: false,
            },
        );
        descriptor
    }

    fn with_mutates(
        mut descriptor: AbilityDescriptor,
        mutates: Vec<&'static str>,
    ) -> AbilityDescriptor {
        descriptor.mutates = static_slice(mutates);
        descriptor
    }

    fn with_actor_policy(
        mut descriptor: AbilityDescriptor,
        actors: Vec<Actor>,
    ) -> AbilityDescriptor {
        descriptor.policy.allowed_actors = static_slice(actors);
        descriptor
    }

    fn experimental(
        mut descriptor: AbilityDescriptor,
        registered_at: &'static str,
    ) -> AbilityDescriptor {
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
        provider: &'a ReplayProvider,
        tracer: &'a dyn AbilityTracer,
    ) -> AbilityContext<'a> {
        AbilityContext::new(services, provider, tracer, actor, confirmation)
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

    #[derive(Default)]
    struct RecordingTracer {
        events: Mutex<Vec<String>>,
    }

    impl AbilityTracer for RecordingTracer {
        fn start_span(&self, name: &str) -> SpanHandle {
            self.events.lock().unwrap().push(format!("span:{name}"));
            SpanHandle { id: 217 }
        }

        fn record_event(&self, span: &SpanHandle, name: &str, _fields: serde_json::Value) {
            self.events
                .lock()
                .unwrap()
                .push(format!("event:{}:{name}", span.id));
        }
    }

    fn fixture_provider() -> ReplayProvider {
        ReplayProvider::new(std::collections::HashMap::new())
            .with_provider_kind(ProviderKind::Other("registry-fixture"))
            .with_model_for_tier(ModelTier::Synthesis, ModelName::new("registry-model"))
    }

    #[test]
    fn validate_schema_closure_passes_for_closed_object_schema() {
        let schema = serde_json::json!({
            "type": "object",
            "additionalProperties": false,
            "properties": {
                "child": {
                    "type": "object",
                    "additionalProperties": false,
                    "properties": {
                        "value": { "type": "string" }
                    }
                },
                "items": {
                    "type": "array",
                    "items": {
                        "type": "object",
                        "additionalProperties": false
                    }
                }
            },
            "$defs": {
                "choice": {
                    "oneOf": [
                        {
                            "type": "object",
                            "additionalProperties": false
                        }
                    ]
                }
            }
        });

        validate_schema_closure(&schema).unwrap();
    }

    #[test]
    fn validate_schema_closure_fails_for_top_level_object_missing_additional_properties() {
        let error = validate_schema_closure(&serde_json::json!({
            "type": "object"
        }))
        .unwrap_err();

        assert_eq!(error.pointer, "");
    }

    #[test]
    fn validate_schema_closure_fails_for_nested_object_in_properties() {
        let error = validate_schema_closure(&serde_json::json!({
            "type": "object",
            "additionalProperties": false,
            "properties": {
                "child": { "type": "object" }
            }
        }))
        .unwrap_err();

        assert_eq!(error.pointer, "/properties/child");
    }

    #[test]
    fn validate_schema_closure_fails_for_nested_object_in_array_items() {
        let error = validate_schema_closure(&serde_json::json!({
            "type": "object",
            "additionalProperties": false,
            "properties": {
                "items": {
                    "type": "array",
                    "items": { "type": "object" }
                }
            }
        }))
        .unwrap_err();

        assert_eq!(error.pointer, "/properties/items/items");
    }

    #[test]
    fn validate_schema_closure_fails_for_object_in_one_of() {
        let error = validate_schema_closure(&serde_json::json!({
            "type": "object",
            "additionalProperties": false,
            "oneOf": [
                { "type": "object" }
            ]
        }))
        .unwrap_err();

        assert_eq!(error.pointer, "/oneOf/0");
    }

    #[test]
    fn validate_schema_closure_includes_violating_path_in_error() {
        let error = validate_schema_closure_for_ability(
            "path_fixture",
            &serde_json::json!({
                "type": "object",
                "additionalProperties": false,
                "properties": {
                    "child": { "type": "object" }
                }
            }),
        )
        .unwrap_err();

        assert_eq!(error.ability_name, "path_fixture");
        assert_eq!(error.pointer, "/properties/child");
        assert!(error.to_string().contains("path_fixture"));
        assert!(error.to_string().contains("/properties/child"));
    }

    #[test]
    #[should_panic(expected = "schema closure")]
    fn registry_build_panics_on_descriptor_with_open_input_schema() {
        let result = AbilityRegistry::from_descriptors_checked(vec![with_input_schema(
            descriptor("open_schema", AbilityCategory::Read),
            open_object_schema,
        )]);

        if let Err(violations) = result {
            panic!("schema closure violation rejected registry build: {violations:?}");
        }
    }

    #[test]
    #[should_panic(expected = "schema closure")]
    fn registry_build_panics_on_descriptor_without_additional_properties_false() {
        let result = AbilityRegistry::from_descriptors_checked(vec![with_input_schema(
            descriptor(
                "open_schema_without_additional_properties_false",
                AbilityCategory::Read,
            ),
            open_object_schema,
        )]);

        if let Err(violations) = result {
            panic!("schema closure violation rejected registry build: {violations:?}");
        }
    }

    #[test]
    fn ability_context_exposes_provider_and_tracer() {
        let clock = FixedClock::new(Utc.with_ymd_and_hms(2026, 5, 4, 12, 0, 0).unwrap());
        let rng = SeedableRng::new(217);
        let external = ExternalClients::default();
        let services = services(&clock, &rng, &external);
        let provider = fixture_provider();
        let tracer = RecordingTracer::default();

        let ctx = AbilityContext::new(&services, &provider, &tracer, Actor::User, None);
        let span = ctx.tracer.start_span("ability_context");
        ctx.tracer
            .record_event(&span, "provider_visible", serde_json::json!({}));

        assert_eq!(
            ctx.provider.provider_kind(),
            ProviderKind::Other("registry-fixture")
        );
        assert_eq!(
            ctx.provider.current_model(ModelTier::Synthesis).as_str(),
            "registry-model"
        );
        assert_eq!(
            tracer.events.lock().unwrap().as_slice(),
            ["span:ability_context", "event:217:provider_visible"]
        );
    }

    #[test]
    fn ability_context_constructed_via_bridge_safe_path_does_not_require_action_db() {
        let clock = FixedClock::new(Utc.with_ymd_and_hms(2026, 5, 4, 12, 0, 0).unwrap());
        let rng = SeedableRng::new(217);
        let external = ExternalClients::default();
        let services = services(&clock, &rng, &external);
        let provider = fixture_provider();
        let tracer = RecordingTracer::default();

        let ctx =
            AbilityContext::from_bridge(&services, &provider, &tracer, BridgeActor::Agent, None);

        assert_eq!(ctx.actor, Actor::Agent);
        assert_eq!(ctx.mode(), ExecutionMode::Live);
        assert_eq!(
            ctx.provider.provider_kind(),
            ProviderKind::Other("registry-fixture")
        );
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

        assert!(
            violations.contains(&RegistryViolation::DuplicateAbilityName(
                "duplicate".to_string()
            ))
        );
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
            compose(
                descriptor("transform", AbilityCategory::Transform),
                "publish",
            ),
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
    fn registry_iter_for_agent_hides_maintenance_and_admin() {
        let registry = registry(vec![
            descriptor("agent_read", AbilityCategory::Read),
            descriptor("agent_maintenance", AbilityCategory::Maintenance),
            with_actor_policy(
                descriptor("admin_read", AbilityCategory::Read),
                vec![Actor::Admin],
            ),
        ]);

        let names: HashSet<&str> = registry
            .iter_for(Actor::Agent)
            .map(|descriptor| descriptor.name)
            .collect();

        assert!(names.contains("agent_read"));
        assert!(!names.contains("agent_maintenance"));
        assert!(!names.contains("admin_read"));
    }

    #[cfg(not(feature = "experimental"))]
    #[test]
    fn registry_rejects_experimental_descriptor_in_production() {
        let violations = AbilityRegistry::from_descriptors_checked(vec![experimental(
            descriptor("experimental_read", AbilityCategory::Read),
            "2999-01-01T00:00:00Z",
        )])
        .unwrap_err();

        assert!(violations.contains(&RegistryViolation::ExperimentalInProduction));
    }

    #[cfg(feature = "experimental")]
    #[test]
    fn registry_iter_for_agent_hides_experimental_from_agent_when_feature_enabled() {
        let registry = registry(vec![experimental(
            descriptor("experimental_read", AbilityCategory::Read),
            "2999-01-01T00:00:00Z",
        )]);

        let names: HashSet<&str> = registry
            .iter_for(Actor::Agent)
            .map(|descriptor| descriptor.name)
            .collect();

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
                        push_compose(
                            &mut descriptors[i],
                            ComposesEntry {
                                id: CompositionId::new(format!("{i}_{target}")),
                                ability: names[target],
                                optional: false,
                            },
                        );
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
                push_compose(
                    &mut descriptors[i],
                    ComposesEntry {
                        id: CompositionId::new(format!("{i}_{target}")),
                        ability: names[target],
                        optional: false,
                    },
                );
                if lcg(&mut seed) % 3 == 0 {
                    let extra = ((lcg(&mut seed) as usize) % descriptors.len()).max(i);
                    push_compose(
                        &mut descriptors[i],
                        ComposesEntry {
                            id: CompositionId::new(format!("{i}_{extra}_extra")),
                            ability: names[extra],
                            optional: false,
                        },
                    );
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

    #[tokio::test]
    async fn invoke_read_rejects_transform_descriptor() {
        let registry = registry(vec![descriptor("transform", AbilityCategory::Transform)]);
        let clock = FixedClock::new(Utc.with_ymd_and_hms(2026, 5, 1, 12, 0, 0).unwrap());
        let rng = SeedableRng::new(42);
        let external = ExternalClients::default();
        let services = services(&clock, &rng, &external);
        let provider = fixture_provider();
        let tracer = RecordingTracer::default();
        let ctx = context(&services, Actor::User, None, &provider, &tracer);

        let err = registry
            .invoke_read(&ctx, "transform", serde_json::json!({}))
            .await
            .unwrap_err();

        assert_eq!(err.kind, AbilityErrorKind::Validation);
    }

    #[tokio::test]
    async fn publish_requires_confirmation_token() {
        let registry = registry(vec![descriptor("publish", AbilityCategory::Publish)]);
        let clock = FixedClock::new(Utc.with_ymd_and_hms(2026, 5, 1, 12, 0, 0).unwrap());
        let rng = SeedableRng::new(42);
        let external = ExternalClients::default();
        let services = services(&clock, &rng, &external);
        let provider = fixture_provider();
        let tracer = RecordingTracer::default();
        let ctx = context(&services, Actor::User, None, &provider, &tracer);

        let err = registry
            .invoke_publish(&ctx, "publish", serde_json::json!({}))
            .await
            .unwrap_err();

        assert_eq!(err.kind, AbilityErrorKind::Capability);
    }
}
