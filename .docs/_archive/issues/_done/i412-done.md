# I412 — User Context in Enrichment Prompts

**Status:** Open
**Priority:** P1
**Version:** 0.14.0
**Area:** Backend / Intelligence

## Summary

Every entity intelligence prompt includes a user context block assembled from the user entity's declared fields. This is the mechanism that enables the risk→opportunity reframe: an account expressing cost pressure is now framed as an opportunity if the user's value proposition includes cost reduction. No new AI call — the user context is injected into the existing intel_queue enrichment prompt (from I367/I369/I386). The change is what the AI knows, not how enrichment works.

## Acceptance Criteria

1. The enrichment prompt builder (in `intelligence/prompts.rs` or equivalent module responsible for assembling enrichment prompts) includes a `build_user_context_block(db) -> Option<String>` function. When the user entity has at least one non-null field, this function returns a formatted block of approximately 150–300 tokens starting with "## User Context" and including all populated fields.

2. The user context block is injected into every entity enrichment prompt (account, person, project) before the enrichment call is made. The block is clearly delimited and appears after the initial instructions and entity data, before the task. Verify: enable DEBUG logging of assembled prompts, trigger enrichment on a known entity, confirm the user context block appears in logs with the user's value_proposition text if that field is set.

3. When all user entity fields are NULL, no user context block appears in the prompt. The prompt structure is identical to its pre-v0.14.0 shape. Verify: clear all user entity fields, trigger enrichment of multiple entities, confirm via log inspection that zero prompts contain "## User Context".

4. The reframe is observable with real data: set `value_proposition` to a cost-reduction narrative (e.g., "We help media companies reduce infrastructure costs through managed hosting"). Find or create an account entity with active signals indicating cost pressure or budget constraints. Trigger enrichment via the hygiene loop or signal event. Read the resulting `entity_intel.executive_assessment` — it should reference the cost-reduction angle, not just flag cost pressure as a risk. This is a qualitative judgment, but the difference must be visible when comparing an assessment with vs. without user context.

5. User context is injected at enrichment time and stored in the intelligence output. Changes to user entity fields affect new enrichment runs, not retroactive updates. Verify: set `current_priorities` to "grow adoption at Acme." Trigger enrichment of Acme. Observe that the assessment references Acme as a priority opportunity. Update `current_priorities` to remove Acme. Wait for the next enrichment cycle (or manually trigger). The new assessment should be reframed. This verifies that user context is read fresh at enrichment time, not cached.

## Dependencies

- Blocked by I411 (user entity table and commands must exist).

Unblocks I413 (document context requires prompts to exist), I414 (signal scoring uses priority fields), I415 (YouCard surface is optional after enrichment integration).

## Notes / Rationale

From ADR-0089 Decision 3: user context is a mechanical addition to every enrichment prompt. This is not an intelligence architectural change — it's injecting a variable (the user's declared context) into an existing template. The `build_user_context_block` function handles the optional nature gracefully: if all fields are NULL, it returns None, and the prompt is unchanged. This preserves backward compatibility for users who never fill in the user entity. The injection happens at prompt assembly time (not at prompt send time), allowing log inspection and debugging. The re-enrichment behavior (criterion 5) is critical for supporting dynamic priority updates — a user changing `current_priorities` should see the intelligence system respond within one enrichment cycle.
