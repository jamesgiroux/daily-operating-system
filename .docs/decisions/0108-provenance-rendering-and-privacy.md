# ADR-0108: Provenance Rendering and Privacy

**Status:** Proposed  
**Date:** 2026-04-18  
**Target:** v1.4.0  
**Extends:** [ADR-0105](0105-provenance-as-first-class-output.md), [ADR-0107](0107-source-taxonomy-alignment.md)  
**Related:** [ADR-0094](0094-audit-log-and-enterprise-observability.md), [ADR-0093](0093-prompt-injection-hardening.md), [ADR-0111](0111-surface-independent-ability-invocation.md)

## Context

[ADR-0105](0105-provenance-as-first-class-output.md) defines the `Provenance` envelope and requires surfaces to render it. It defers three concerns to this ADR:

1. **Rendering rules per surface** — what the Tauri app, MCP, P2 publications, and logs show.
2. **Privacy and safety** — what gets masked, redacted, or filtered when rendered to different actors or published externally.
3. **The `ProvenanceMasked` shape** referenced by [ADR-0105](0105-provenance-as-first-class-output.md) §8 and used when sources are revoked.

[ADR-0094](0094-audit-log-and-enterprise-observability.md) governs the append-only JSONL security audit with strict PII hygiene. [ADR-0093](0093-prompt-injection-hardening.md) established that LLM outputs can contain adversarial content. This ADR integrates both: provenance must be renderable to users and agents without leaking internal graph structure or PII, and sanitization of LLM-generated `explanation` text must prevent re-injection.

## Decision

### 1. Per-Surface Rendering Rules

**Tauri app (first-party).**

- Every user-facing ability output shows an "About this" affordance (previously described as "Why?"). The affordance is conditional: display it when the output is a composed brief, a narrative, a risk assessment, or any LLM-synthesized content. Do not display it on compact badges, tooltip strings, or single-field projections.
- Expanding the affordance shows: ability name, produced_at timestamp (relative + absolute), number of sources, trust assessment (Trusted / Untrusted with reason), composition depth summary.
- A secondary "Details" expansion shows field-level attributions: for each significant output field, the sources that contributed, their `data_source` (using human-readable names via [ADR-0107](0107-source-taxonomy-alignment.md)'s `DataSource::display_name()`), and any `confidence` score.
- A third-level "Full provenance" expansion shows the JSON envelope for power users and debugging.
- Sources whose `SynthesisMarker.producer_ability` is set display a visible "AI-generated" badge.

**MCP server (agent-facing).**

- Every tool response includes provenance by default. The `Provenance` envelope serializes as a structured field alongside the domain output.
- Actor-filtered contents: the MCP wrapper strips content from provenance that is not permitted for `Actor::Agent`:
  - Source identifiers that reference internal graph IDs (signal_id, entity_id, document chunk IDs) are replaced with `SourceIdRedacted { data_source, scoring_class }`.
  - Child provenance trees are collapsed to `ChildElided { ability_name, data_sources_summary }` beyond depth 2.
  - Prompt fingerprints include `provider`, `model`, `prompt_template_id`, `prompt_template_version`, but NOT `canonical_prompt_hash` (internal) or `seed`.
  - `FieldAttribution.explanation` text is returned as-is but is sanitized per §3.
- An MCP tool `get_provenance(invocation_id)` provides a fuller view for authorized agents if needed; default response is summary-sized.

**P2 publications.**

- Publications carry a provenance footnote listing the top-level sources by `data_source` class (e.g., "Based on Salesforce, recent meetings, and team activity") — **no source identifiers, no entity names, no attribution text**.
- Expandable "Details" block is available in the publication renderer but only if the publishing user confirms at publish time that the detail level is appropriate for the P2 audience. This is an explicit confirmation in the publish dialog, not a default.
- Synthesis markers render as "AI-generated summary" prefix on the publication body.
- Trust classification does not render to external readers; it is internal operational metadata.

**Logs and structured telemetry.**

- The `maintenance_audit` table stores the full `ProvenanceOrMasked` per invocation.
- Application logs reference `invocation_id` and do not include provenance content; to retrieve a run's provenance, query the audit table.
- [ADR-0094](0094-audit-log-and-enterprise-observability.md)'s append-only JSONL security audit remains unchanged — it captures security events (auth, permissions, source revocations, etc.) with its existing hygiene rules. Provenance lives in the operational `maintenance_audit` SQLite table, separate from the security audit.

### 2. Actor-Filtered Rendering

A universal renderer transforms `Provenance` into an actor-appropriate view:

```rust
pub fn render_provenance_for(prov: &Provenance, actor: Actor, surface: Surface) -> RenderedProvenance { ... }

pub enum Surface {
    TauriApp,         // Full detail, three-level expansion
    McpTool,          // Actor-filtered, default-summary
    McpToolDetail,    // Actor-filtered, full-detail (invoked via get_provenance)
    P2Publication,    // External audience, heavy redaction
    LogStructured,    // Operator view, no PII
}
```

The renderer consults `actor` and `surface` to determine what to include. A user rendering provenance in the app sees the full graph; an agent accessing the same ability's output via MCP sees a filtered version; a P2 publication sees source classes only.

**Agents never see:** internal signal IDs, entity IDs (unless explicitly part of the response schema), raw `prompt_template` text, internal `canonical_prompt_hash`, internal entity version watermarks. These are operational internals; surfacing them to agents gives attackers a map of the user's graph.

### 3. Sanitization of `FieldAttribution.explanation`

LLM-generated `explanation` text is untrusted per [ADR-0093](0093-prompt-injection-hardening.md). It can contain prompt injection attempts, PII from other sources, or links to attacker-controlled content. Every explanation is passed through a sanitizer before rendering:

1. **HTML entities encoded.** Any `<`, `>`, `&`, `"`, `'` is entity-escaped. No raw HTML or Markdown links rendered from explanations.
2. **Length bounded.** Explanations over 500 characters are truncated with an ellipsis. Long explanations are a smell.
3. **URL stripping.** Anything resembling a URL (regex-based) is removed and replaced with `[url removed]`. If the explanation wants to cite a URL, it must come through a typed `SourceAttribution` with a URL field, not through free text.
4. **Banned token list.** Explanations containing instruction-like phrases common in prompt injection attempts (e.g., "ignore previous", "you are now", "system:") are filtered: the whole explanation is replaced with `[explanation removed by sanitizer]` and a `ProvenanceWarning::ExplanationFiltered` is added.
5. **No executable content.** Regardless of surface, explanations are never passed to a markdown renderer, a JS `eval` context, or any other dynamic evaluator.

The sanitizer is shared across all surfaces; the rendering layer calls it once per `FieldAttribution` before composing the rendered output.

### 4. The `ProvenanceMasked` Shape

When a source is revoked and its associated provenance records are masked per [ADR-0107](0107-source-taxonomy-alignment.md) §5:

```rust
pub struct ProvenanceMasked {
    pub original_invocation_id: InvocationId,
    pub original_ability_name: &'static str,
    pub original_produced_at: DateTime<Utc>,
    pub masked_at: DateTime<Utc>,
    pub mask_reason: MaskReason,
    pub sources_masked: Vec<DataSource>,     // Which sources triggered the masking
}

pub enum MaskReason {
    SourceRevoked { data_source: DataSource },
    GleanDisconnected,
    UserDeletedEntry,
    RetentionExpired,
}
```

The `ProvenanceOrMasked` enum in [ADR-0103](0103-maintenance-ability-safety-constraints.md) §8's `MaintenanceAuditRecord` is either the full envelope or this masked placeholder. Consumers handle both.

**Masking is irreversible.** Once a provenance is masked, the original content is discarded. Re-running the ability after the source is re-connected produces a new invocation with its own provenance. This is the cost of the mask — diagnosis of past runs becomes unavailable for revoked sources, but user privacy is protected.

### 5. Size Budgets for Rendering

[ADR-0105](0105-provenance-as-first-class-output.md) §9 specifies storage size budgets. Rendering has its own:

- **Tauri app initial render:** top-level summary ≤ 2KB. Expansion fetches additional detail on demand.
- **MCP default response:** provenance payload ≤ 10KB. Full detail available via `get_provenance(invocation_id)`.
- **P2 publication footnote:** ≤ 500 characters of human-readable text summarizing sources.

When rendering exceeds a budget, the renderer truncates deepest-first (children elided first, sources last) and surfaces a `ProvenanceWarning::TruncatedForRender { surface, budget_bytes, original_bytes }`.

### 6. Rendering-Required vs. Rendering-Optional Outputs

[ADR-0105](0105-provenance-as-first-class-output.md) §7 declared "every surface that displays ability output MUST be capable of rendering provenance." This ADR narrows the "MUST" to outputs where provenance rendering is meaningful:

**Rendering required:**

- Meeting prep, daily briefing, weekly narrative, risk assessment, entity context detail pages
- MCP tool responses
- P2 publications
- Maintenance audit detail views in Settings

**Rendering not required (summary or implicit acknowledgment sufficient):**

- Badges, chips, count displays, single-value tooltips
- Navigation items showing ability-derived names
- Settings list views (each row can show a "source count" indicator; full provenance on tap)

The test: if the output is a *composed* or *synthesized* artifact the user will reason about, render provenance. If it is a *presentation affordance* showing a value, a count, or a name, the rendering is optional (but the provenance still exists in storage and can be queried).

## Consequences

### Positive

1. **Rendering rules are surface-specific.** The Tauri app gets rich detail; MCP gets actor-filtered; P2 gets heavy redaction; logs get operational metadata. No one-size-fits-all.
2. **Actor filtering prevents internal structure leakage to agents.** Agents do not learn signal IDs or internal entity IDs unless the response schema explicitly exposes them.
3. **LLM-generated explanations are sanitized.** Prompt injection through `explanation` fields is defanged before rendering.
4. **Masking has explicit shape.** `ProvenanceMasked` is defined; consumers handle `ProvenanceOrMasked` uniformly.
5. **Size budgets bounded per surface.** Rendering cost is predictable; runaway provenance payloads are truncated with an explicit warning.
6. **Rendering-required boundary is clear.** Not every UI element bears the "MUST render" burden; composed and synthesized artifacts do.

### Negative

1. **Renderer complexity grows.** A shared `render_provenance_for(prov, actor, surface)` function with branching logic per surface and actor is non-trivial. Worth it.
2. **Sanitizer maintenance.** Banned-token lists and URL regexes need upkeep as prompt-injection patterns evolve.
3. **MCP `get_provenance(invocation_id)` tool adds surface.** Agents can request detail they did not receive by default.
4. **P2 publication confirmation dialog adds friction.** Users publishing with full detail must opt in each time.

### Risks

1. **Sanitizer bypass.** A novel prompt-injection pattern slips past the filter. Mitigation: sanitizer is shared and regularly reviewed; any bypass is a CVE-class issue handled with high priority.
2. **Renderer drift across surfaces.** The Tauri app diverges from MCP in how it interprets provenance. Mitigation: `render_provenance_for` is the single entry point; surfaces cannot bypass it.
3. **Masked provenance hides legitimate debugging.** User reports data issue; the relevant maintenance audit is masked. Mitigation: masking is irreversible as a matter of privacy. Live re-run reproduces the data issue with fresh provenance.
4. **P2 publication leaks through "Details" toggle.** User opts in to full detail and leaks internal IDs. Mitigation: publication "Details" renders through the same actor filter as MCP when the P2 audience includes non-team members; team-internal publishes get full detail.
5. **Tauri `canonical_prompt_hash` leakage in DevTools.** Power users inspecting the Tauri bridge can see internal details. Mitigation: accepted — the Tauri app runs as the user's process; there is no threat model where the user is attacking themselves.

## References

- [ADR-0105: Provenance as First-Class Output](0105-provenance-as-first-class-output.md) — Defines the envelope; this ADR specifies rendering.
- [ADR-0107: Source Taxonomy Alignment](0107-source-taxonomy-alignment.md) — Specifies `DataSource::display_name()` and `ScoringClass`; rendering uses both.
- [ADR-0094: Audit Log and Enterprise Observability](0094-audit-log-and-enterprise-observability.md) — Security audit remains separate from provenance rendering.
- [ADR-0093: Prompt Injection Hardening](0093-prompt-injection-hardening.md) — Explanation sanitizer aligns with ADR-0093's untrusted-content rules.
- [ADR-0103: Maintenance Ability Safety Constraints](0103-maintenance-ability-safety-constraints.md) — `MaintenanceAuditRecord` carries `ProvenanceOrMasked` per this ADR's §4.
- **ADR-0111 (forthcoming): Surface-Independent Ability Invocation** — Consumes the per-surface rendering rules specified here.

---

## Amendment — 2026-04-20 — Enforce 64 KB provenance size cap

Addresses persona-review finding S3 (size budgets named but not enforced).

§6 originally introduced size budgets as a soft guideline. In practice a deeply-composed ability (meeting prep composing context which composes claims which compose trajectories) can construct a provenance envelope of several hundred KB. Tauri IPC has serialization limits; this will hit them. Persona review flagged: named limits without enforcement become silent bloat.

**Hard cap: 64 KB serialized provenance per ability output.**

- The cap applies to the full `Provenance` envelope after JSON serialization, before any surface rendering.
- Abilities that would exceed the cap return `Err(AbilityError::ProvenanceTooLarge { size_bytes, cap_bytes })`. No silent truncation.
- The cap is enforced at `AbilityOutput<T>` construction time. The error surfaces to the caller as a hard error (per [ADR-0102](0102-abilities-as-runtime-contract.md) error-handling amendment).

**Forces one of three redesigns when hit:**

1. **Provenance summarization** — collapse deep composition trees into summary nodes marked `ProvenanceWarning::DepthElided`. [ADR-0105](0105-provenance-as-first-class-output.md) §1 already declares this warning class; this amendment activates it.
2. **Shallower composition** — refactor the ability to compose fewer children or to use an alternative data source.
3. **Conscious cap increase** — via ADR amendment, after measuring real production composition shapes. Not an automatic bump.

**Starting value of 64 KB is conservative.** Measure during the first end-to-end slice (strategy doc § Path forward action 1). If real meeting-prep provenance is larger than 64 KB on representative entities, raise the cap via amendment with the measurement as justification. Do not raise silently.

**Rendering budget (original §6) vs serialization budget (this amendment):** different concerns. The rendering budget limits what the renderer emits for a given surface; this cap limits what the envelope itself can carry regardless of surface. Both exist independently; rendering truncation does not fix a too-large envelope because the envelope is already over-cap before the renderer sees it.
