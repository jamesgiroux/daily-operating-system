# I521 — Frontend Structural Cleanup + Production-Data Parity Gate

**Priority:** P1  
**Area:** Frontend / Architecture  
**Version:** v1.0.0 (Phase 3c)  
**Depends on:** I536 (mock data migration), I503 + I508a (schema/type changes landed), I511 (stable schema baseline)  
**Blocks:** I447-I453 editorial polish and any Phase 3 UI signoff

## Problem

Phase 3 frontend work is currently vulnerable to a false-green pattern:
1. Surfaces render with mock data but fail or degrade with production-shaped payloads.
2. Ghost components and duplicate implementations create split ownership and inconsistent behavior.
3. Command payload assumptions are implicit in component code, so field drift is detected late.
4. Actions/suggested actions parity is fragile: UI appears healthy in mocks while production data can suppress expected content.

Without a hard parity gate, Phase 3 can ship UI that passes local mock flows and still fails in production.

## Design

### 1. Ghost removal + single ownership

- Remove dead/unused components and hooks that are no longer reachable from routes.
- For each major surface, establish one owner path (hook + page/component pair) and remove duplicate rendering paths.
- Produce an explicit ownership map for all major surfaces:
  - dashboard/briefing
  - actions
  - account/project/person detail
  - meeting detail
  - inbox/emails
  - settings/data
  - reports

### 2. Duplicate pattern consolidation

- Consolidate duplicate loading/error/empty-state patterns into shared primitives where behavior is equivalent.
- Keep component responsibilities narrow:
  - hooks fetch/normalize command data
  - pages compose sections and route actions
  - leaf components render only
- Remove page-level ad hoc transforms that duplicate existing view-model logic.

### 3. Command/field contract map (decision-complete artifact)

- Maintain a command contract registry that maps:
  - surface -> command -> required response fields -> consuming components
- Registry is versioned and committed so contract changes are review-visible.
- Contract registry becomes a release artifact for Phase 3 acceptance.

### 4. Production-shape parity gate (mandatory merge gate)

- Add deterministic fixture sets for each major surface:
  - `mock`
  - sanitized `production` shape
- Add automated parity tests that enforce:
  - every declared command payload exists for both datasets
  - required field paths exist in both datasets
  - no mock-only response shape dependency
  - consistent error payload shape for degraded rendering
- Gate rule: a feature is not done if it only passes mock fixtures.

### 5. Actions parity hard requirement

- Explicitly validate actions + proposed actions for both datasets:
  - data is present and renderable
  - required metadata fields exist for lifecycle actions
  - no fallback-only rendering path that hides real DB-backed actions

## Files to Modify

| File | Change |
|---|---|
| `src/parity/phase3ContractRegistry.ts` | Canonical surface/command/required-field map |
| `src/parity/phase3ParityGate.test.ts` | Mandatory mock vs production-shape parity tests |
| `.docs/contracts/phase3-ui-contract-registry.json` | Committed contract registry artifact |
| `.docs/fixtures/parity/mock/*.json` | Deterministic mock fixture set |
| `.docs/fixtures/parity/production/*.json` | Sanitized deterministic production-shape fixture set |
| `package.json` + CI workflow | Named parity gate command and CI enforcement |

## Acceptance Criteria

1. All major surfaces listed above have contract entries and both fixture datasets.
2. `pnpm run test:parity` passes locally and in CI.
3. Any mock-only field dependency fails parity tests.
4. Actions and proposed actions render correctly for production-shaped fixtures (not mocks alone).
5. Surface error payloads follow one consistent contract shape (`code`, `message`, `retryable`).
6. No Phase 3 UI issue (I502, I493, I447-I453, I529, I537) can be marked done without parity gate pass evidence.
7. Contract map updates are required in the same PR when command payload fields change.

## Out of Scope

- New feature implementation for search/offline/export/privacy (I427-I431, I438)
- Editorial visual redesign itself (I447-I453)
- Backend reliability and module decomposition work (I514, I515)

## Relationship to Other Issues

- **I521 first in Phase 3c:** structural cleanup and parity baseline before UI polish.
- **I536 dependency:** mock data migration is required but not sufficient; production-shape parity adds the missing gate.
- **I502 + I493 + I447-I453 + I529 + I537:** all consume this contract/parity infrastructure and cannot bypass it.
