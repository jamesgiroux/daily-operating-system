# ADR-0006: Phase 3 generates JSON (determinism boundary)

**Date:** 2026-02
**Status:** Accepted

## Context

The three-phase pattern (Prepare → Enrich → Deliver) needs a clear boundary between deterministic and non-deterministic operations. Claude (Phase 2) outputs natural language well but structured JSON unreliably.

## Decision

Claude outputs markdown (its strength). Python Phase 3 converts to validated JSON (deterministic, testable). The boundary is: Phase 1 and 3 are deterministic Python; Phase 2 is non-deterministic AI.

## Consequences

- JSON output is always well-formed (Python validates, not AI)
- Phase 3 scripts are fully testable without AI
- If Claude's structured output improves, this boundary could relax — but it's a safe default
- Any new data pipeline must respect this boundary: deterministic phases wrap the AI phase
