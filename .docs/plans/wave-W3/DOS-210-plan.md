# Implementation Plan: DOS-210

## Revision history
- v1 (2026-05-01) - initial L0 draft.

## 1. Contract restated

DOS-210 creates the first ability substrate: an `#[ability]` proc macro, an inventory-backed `AbilityRegistry`, category-aware invocation, `AbilityContext`, schema/docs generation, and compile-time category checks before DOS-218+ migrate real capabilities. Load-bearing ticket lines: "No ability registry exists in the codebase today"; "`#[ability]` proc macro compiles and generates correct metadata (AbilityDescriptor + type-erased wrapper + JSON input/output schemas)"; "Category classification via proc-macro AST inspection"; "Read and Transform abilities with any detected mutation-path call fail to compile"; and "`AbilityContext` construction API that wraps `ServiceContext` with actor, tracer, confirmation token."

Scope stays substrate-only. DOS-210 does not build the evaluation harness, Tauri/MCP bridges, or a first migrated capability. The wave plan gives W3-A `src-tauri/abilities-macro/`, `src-tauri/src/abilities/registry.rs`, and trybuild fixtures, with done-when "build-time category enforcement; trybuild rejects category violations; registry inspection API lands" (`.docs/plans/v1.4.0-waves.md:466-470`).

ADR-0102 supplies the shape: abilities live under `src-tauri/src/abilities/` (`.docs/decisions/0102-abilities-as-runtime-contract.md:45-74`), category is based on transitive service effects (`:76-95`), signatures return `AbilityResult<T>` (`:96-113`), `AbilityContext` wraps `ServiceContext` (`:114-136`), descriptors/wrappers/inventory are emitted by the macro (`:181-230`), and actor-scoped enumeration hides unauthorized abilities (`:250-258`). Amendment A applies: hard error, soft degradation through provenance warnings, and hard success are the only outcome paths (`:458-483`). Amendment B applies: `experimental = true` waives provenance/fixture/category enforcement for one cycle, but requires `registered_at`, feature gating, expiry CI, no claim writes, no publish, and no MCP exposure (`:485-511`).

ADR-0120 applies directly: every `#[ability]` macro-expanded entry point opens a `tracing` span with invocation identity (`.docs/decisions/0120-observability-contract.md:65-85`), emits no raw user/prompt/completion content (`:126-135`), and the macro wiring is explicitly in v1.4.0 scope (`:252-265`).

DOS-304 is a blocking 2026-04-24 contract constraint. Its load-bearing line is: "Proc-macro AST inspection cannot be the hard safety boundary." The plan still implements the DOS-210 compile-time AST category check, but the hard boundary is `ServiceContext` capability handles: abilities cannot receive raw `ActionDb`, raw SQL/file-write handles, live queues, or direct app state. The single registry choice is: abilities are the operation source of truth; DOS-217 derives Tauri/MCP tools from this registry rather than creating a second operations registry.

Current repo reality: W2-A and W2-B are present. `ServiceContext` exists with `ExecutionMode`, clock/RNG, external clients, and private transaction handle (`src-tauri/src/services/context.rs:271-277`), and `check_mutation_allowed()` rejects non-Live writes (`src-tauri/src/services/context.rs:412-422`). W2 proof says every catalogued service mutator now takes `ServiceContext` first and gates first (`.docs/plans/wave-W2/proof-bundle.md:155-158`). The provider seam already reserves `select_provider(ctx: &AbilityContext, tier)` for W3-A (`src-tauri/src/intelligence/provider.rs:14-22`, `:309-326`).

## 2. Approach

Create `src-tauri/abilities-macro/` as a proc-macro crate and make `src-tauri/Cargo.toml` the workspace root by adding `[workspace] members = [".", "abilities-macro"]`; the current manifest is package-only (`src-tauri/Cargo.toml:1-17`) and CI already invokes `cargo clippy --manifest-path src-tauri/Cargo.toml --workspace` (`.github/workflows/test.yml:74-79`). Add main-crate dependencies for `inventory`, `tracing`, `tracing-subscriber`, `tracing-test`, and non-optional `schemars`; today `schemars` is optional behind `mcp` (`src-tauri/Cargo.toml:82-90`), but ability schemas/docs are not MCP-only.

Create `src-tauri/src/abilities/mod.rs` as the module root, `src-tauri/src/abilities/registry.rs` as W3-A's owned substrate, and add `pub mod abilities;` beside `services`/`signals` in `src-tauri/src/lib.rs:66-67`. `registry.rs` owns `AbilityCategory`, `Actor`, `AbilityPolicy`, `AbilityDescriptor`, `AbilityContext`, `ConfirmationToken`, `AbilityError`, `AbilityResult<T>`, erased wrapper types, `AbilityRegistry`, registry validation, actor filtering, invocation methods, signal-policy metadata, and doc rendering. `AbilityOutput<T>` is the ADR-0102 return wrapper, but its provenance field is supplied by W3-B's `src-tauri/src/abilities/provenance/`; W3-A and W3-B must merge through an integration branch if neither branch wants a temporary stub.

The macro parses `#[ability(name = "...", category = Read|Transform|Publish|Maintenance, version = "...", schema_version = N, allowed_actors = [...], allowed_modes = [...], requires_confirmation = bool, may_publish = bool, composes = [...], experimental = bool, registered_at = "...", signal_policy = ...)]` on an async function with shape `(&AbilityContext, Input) -> AbilityResult<Output>`. It emits trait bounds requiring `Input: DeserializeOwned + JsonSchema` and `Output: Serialize + JsonSchema`, a static `AbilityDescriptor`, `inventory::submit!`, an erased JSON wrapper, schema builders using `schemars::schema_for!`, a root `tracing` span, and a duplicate-name link symbol.

AST enforcement is implemented in `abilities-macro/src/scoring.rs::mutation_allowlist()`. Do not put this in `src-tauri/src/signals/scoring.rs`: that file is runtime item relevance scoring (`src-tauri/src/signals/scoring.rs:1-7`) and would create a proc-macro dependency cycle. The macro crate build script reads `src-tauri/tests/dos209_mutation_catalog.txt`, whose header documents the Rust scanner and mutation regex method (`src-tauri/tests/dos209_mutation_catalog.txt:1-6`), and generates a static list of fully qualified service mutator paths. The visitor detects `services::*`, `crate::services::*`, and imported aliases that resolve to allowlisted service functions. Read/Transform with a detected mutation emits `compile_error!`; Publish/Maintenance record the inferred mutation set in `AbilityDescriptor::mutates`. Experimental abilities still record the set but do not block.

Registry validation runs in `AbilityRegistry::from_inventory_checked()`: collect `inventory::iter::<AbilityDescriptor>`, reject duplicate names, unknown `composes`, policy contradictions, expired experimental abilities, unauthorized experimental exposure, composition cycles, and category violations through the composition graph. Typed paths are `invoke_read`, `invoke_transform`, `invoke_publish`, and `invoke_maintenance`; each verifies category, actor policy, mode policy, confirmation requirements, and JSON schema before calling the wrapper. `invoke_by_name_json` is the erased path for DOS-217. `iter_for(Actor::Agent)` excludes maintenance, admin-only, and experimental abilities unless the explicit experimental feature is active.

Cycle detection uses a deterministic DFS with three colors (`Unvisited`, `Visiting`, `Done`) over descriptor names. During DFS, it also folds transitive composed categories and mutation sets so Read/Transform cannot compose a Publish/Maintenance or any ability with non-empty `mutates`. Property coverage generates 100 acyclic DAGs and 100 graphs with injected cycles per run.

Documentation generation lives as a pure registry API: `AbilityRegistry::render_docs(out_dir)`. Output is deterministic `.docs/abilities/{name}.md` with YAML front matter (`name`, `version`, `schema_version`, `category`, `experimental`, `allowed_actors`, `allowed_modes`, `requires_confirmation`, `may_publish`, `mutates`, `composes`, `signal_policy`) followed by summary, policy table, input schema JSON, output schema JSON, and composition/mutation notes. Tests render sample abilities into a tempdir; production docs are generated once the first real ability lands.

End-state alignment: this makes ADR-0102's registry the single capability spine that DOS-217 bridges, DOS-216 discovers, DOS-218+ migrate into, and DOS-211 provenance wraps. It forecloses a separate operations registry and forecloses ability code that bypasses `ServiceContext` capability handles.

## 3. Key decisions

Mutation allowlist location: `abilities-macro/src/scoring.rs::mutation_allowlist()` generated from `src-tauri/tests/dos209_mutation_catalog.txt`, not runtime `signals::scoring`. Reason: proc macros cannot depend on the main crate; the W2 catalog is already the audited service mutation source (`src-tauri/src/services/context.rs:21-28`, `.docs/plans/wave-W2/proof-bundle.md:155-158`).

Compile-time category enforcement: the macro does direct AST detection plus declared composition closure. This supersedes ADR-0102's older "declarative metadata and not inference" rationale (`.docs/decisions/0102-abilities-as-runtime-contract.md:187-224`) for DOS-210, but DOS-304 still stands: the macro is not the hard safety boundary. The hard boundary is `AbilityContext` exposing only `ServiceContext` capabilities and never raw DB/app handles.

Duplicate names: use both build-time and startup checks. The macro emits a `#[used]` exported symbol named from the ability name, making duplicate names fail at link/build time. The registry also checks duplicates at startup/test time to return a clear `RegistryViolation::DuplicateAbilityName` instead of relying only on a linker message.

Composition cycles: registry DFS is the authoritative algorithm; CI runs it through `cargo test --test ability_registry_graph`. Because inventory is only visible after link, cycle validation is a startup/CI build-gate check, not a per-macro invocation check.

Experimental: compile all experimental registrations only under `cfg(feature = "experimental")`, require `registered_at`, force trust/visibility restrictions from ADR-0102 Amendment B, and add `experimental_expiry` test with a 90-day threshold (`.docs/decisions/0102-abilities-as-runtime-contract.md:500-511`). Main manifest currently has only `mcp` feature (`src-tauri/Cargo.toml:89-90`), so W3-A adds `experimental = []`.

Signal policy: include descriptor fields now, but do not implement durable invalidation. Linear's DOS-210 comment requires per-ability emit-on-output-change declarations; ADR-0115 defines `AbilityOutputChanged` as ability-output granularity (`.docs/decisions/0115-signal-granularity-audit.md:32-39`) and `PropagateAsync { coalesce: true }` as its policy (`:54-57`). ADR-0115 R1.1/R1.3 says typed `SignalType` and function-form policy registry are prerequisites (`:224-234`, `:249-269`), so DOS-210 records metadata and leaves emission wiring to the signal-policy issue.

Documentation output: one file per ability, stable key order, pretty JSON schemas, no prose generated from runtime data, and no customer content. The generator is an explicit command/API, not a build-script side effect, because `src-tauri/build.rs` currently only calls `tauri_build::build()` (`src-tauri/build.rs:1-3`) and build scripts cannot inspect the final inventory.

Single registry: abilities are operations. DOS-217 must derive Tauri/MCP tools from `AbilityRegistry`, not from a parallel operations array. This closes DOS-304's "do not build two registries" blocker.

## 4. Security

New attack surfaces are schema-driven JSON invocation, actor-filtered introspection, docs output, and macro-enforced metadata. JSON input is deserialized only through the registry wrapper and schema-validated before and after invocation. Actor filtering is enforced in `iter_for` and every invoke path; enumeration does not leak maintenance/admin-only names or schemas to `Actor::Agent`, matching ADR-0102 (`.docs/decisions/0102-abilities-as-runtime-contract.md:250-258`).

Ability implementations receive `AbilityContext`, not `AppState`, raw `ActionDb`, direct SQL, filesystem, live queue, or direct provider construction. `AbilityContext.services` wraps the W2 `ServiceContext`, whose public fields are mode/clock/rng/external while transaction internals are service-private (`src-tauri/src/services/context.rs:271-277`). This is the DOS-304 hard boundary. Read/Transform category failures are caught by the macro and backed by service-layer non-Live write rejection.

Logs/spans must follow ADR-0120 redaction: no raw user content, prompt text, completion text, or untyped error messages (`.docs/decisions/0120-observability-contract.md:128-135`). Experimental abilities are not exposed through MCP and not through Tauri unless dev/experimental feature gates are enabled (`.docs/decisions/0102-abilities-as-runtime-contract.md:489-511`).

## 5. Performance

Startup cost is O(N + E) for inventory collection, duplicate-name HashMap insertion, schema builder registration, and composition DFS. N is expected to be dozens in v1.4.0, so this is below app startup noise; validation runs once through `OnceLock`. Invocation adds one registry lookup, actor/mode/category checks, one JSON deserialize/serialize round trip for erased calls, and no round trip for direct typed ability calls.

Macro AST scanning is compile-time only. The mutation allowlist generated from the 228-mutator catalog is a static slice; visitor cost is proportional to function body AST size. Link-symbol uniqueness adds no runtime cost. Documentation generation is offline and deterministic.

## 6. Coding standards

Services-only mutations are preserved: abilities compose services and never write DB/file/signal state directly. The macro denies detected service mutators in Read/Transform, and ServiceContext denies writes in non-Live modes. Intelligence Loop check: no new signal type is emitted by W3-A; no health scoring rule changes; no intel context builder migration; no briefing callout; no feedback hook. Only signal-policy metadata is recorded.

No direct `Utc::now()` or `thread_rng()` in abilities or macro-generated ability code. `ServiceContext` already provides clock/RNG seams (`src-tauri/src/services/context.rs:64-80`, `:109-149`). Existing provider lint covers provider modules only and says W2-A covers services/abilities (`scripts/check_no_direct_clock_rng_in_provider_modules.sh:1-9`); W3-A extends the no-direct-clock/RNG lint to the new abilities directory. Trybuild fixtures contain synthetic data only. Clippy budget is `cargo clippy --manifest-path src-tauri/Cargo.toml --workspace --all-features --lib --bins -- -D warnings` (`.github/workflows/test.yml:74-75`).

## 7. Integration with parallel wave-mates

DOS-209 is consumed, not edited: `src-tauri/src/services/context.rs` is frozen W2-A territory. W3-A reads its public `ServiceContext` shape and wraps it in `AbilityContext`.

DOS-259 is consumed, not edited: provider selection is waiting for W3-A's `AbilityContext` (`src-tauri/src/intelligence/provider.rs:14-22`, `:309-326`). W3-A defines the context field that lets a later cleanup replace `select_provider_stub`.

DOS-211 owns `src-tauri/src/abilities/provenance/`. W3-A should not define final provenance internals; it references W3-B's `Provenance` and enforces the `AbilityOutput<T>` envelope once both branches are integrated.

DOS-216 consumes registry enumeration for fixture harness discovery. DOS-217 consumes `invoke_by_name_json`, `iter_for(Actor::Agent)`, and descriptors for Tauri/MCP bridges. DOS-218+ consume the macro for real abilities. W3-C/D/E/F/G/H do not share files with W3-A, but W3-C's `services::claims::commit_claim` will be picked up by the generated mutation allowlist once the W2 mutation catalog is regenerated.

## 8. Failure modes + rollback

If macro parsing rejects valid ability syntax, only new abilities fail to compile; rollback is to remove the macro attribute or revert the proc-macro crate. If the mutation allowlist under-detects, ServiceContext capability restrictions and `check_mutation_allowed()` still block unauthorized writes in Simulate/Evaluate; Live-mode helper-call gaps are covered by fixture trace tests and DOS-304's no-raw-handle rule. If registry validation fails at startup, fail closed with a typed registry error before bridges enumerate tools.

If docs generation fails, it does not block runtime invocation unless CI is running the docs snapshot test. If inventory registration breaks, registry count is zero and `ability_registry_self_check` fails. No SQL migration is involved. W1-B universal write fence is honored because W3-A introduces no new DB/file write path; future Maintenance abilities still mutate only through services and the existing fence.

## 9. Test evidence to be produced

Trybuild compile-fail/pass: `read_ability_direct_mutation_fails`, `transform_ability_imported_mutation_alias_fails`, `publish_ability_records_mutation_set_passes`, `maintenance_ability_records_mutation_set_passes`, `experimental_read_mutation_warns_not_fails`, `ability_signature_must_return_ability_result`, `ability_input_must_deserialize_and_schema`, `ability_output_must_serialize_and_schema`, `duplicate_ability_name_link_fails`, `experimental_missing_registered_at_fails`, and `experimental_may_publish_fails`.

Registry tests: `registry_collects_inventory_descriptors`, `registry_rejects_duplicate_names_with_clear_error`, `registry_rejects_unknown_composes`, `registry_rejects_read_composing_publish_transitively`, `registry_rejects_transform_composing_maintenance_transitively`, `registry_iter_for_agent_hides_maintenance_admin_and_experimental`, `invoke_by_name_json_validates_input_schema`, `invoke_read_rejects_transform_descriptor`, `publish_requires_confirmation_token`, `experimental_expiry_rejects_over_90_days`, and `documentation_generator_renders_stable_markdown`.

Property tests: `composition_graph_accepts_100_random_dags`, `composition_graph_rejects_100_random_cycles`, and `composition_graph_folds_transitive_mutation_sets`.

Wave merge-gate contribution: W3-A supplies the substrate/architect-reviewer L0 and L2 evidence, clippy/test green, trybuild artifacts, registry self-check report, and Suite E substrate discovery proof for bundles 1+5 once W4/W5 fixtures exist. Suite S contribution is actor-filtered enumeration and no raw content in spans; Suite P contribution is registry startup/invocation overhead measurement, expected O(N + E) startup and one lookup per erased invocation.

## 10. Open questions

1. Production ability count: DOS-210's done checklist asks for at least one non-experimental ability, but the scope limits say DOS-218 is the first capability migration. Should W3-A ship only test/sample abilities, or add a real internal registry-inspection ability?

2. Provenance merge order: should W3-A wait for DOS-211 to land `Provenance`/`AbilityOutput<T>` first, or should the integration branch carry a temporary compile seam that W3-B replaces?

3. Cycle rejection wording: inventory graph cycles can be rejected by startup registry validation and CI before merge, but not by a single proc-macro expansion. Is that acceptable as "build time" for the ticket, or does architect-reviewer require a centralized non-inventory catalog?
