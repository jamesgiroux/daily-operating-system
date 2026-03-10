# Phase 3 Execution Tracker (v1.0.0)

**Last updated:** 2026-03-10  
**Execution mode:** Umbrella + short-lived wave branches  
**Policy:** No Phase 3 issue closes without production-data parity gate evidence.

## Branch isolation model (locked)

1. Umbrella integration branch: `codex/v1-phase3`
2. Short-lived issue branches from umbrella (examples):
- `codex/v1-phase3-i515`
- `codex/v1-phase3-i427`
- `codex/v1-phase3-i502`
3. Merge path:
- issue branch -> `codex/v1-phase3` (after issue AC + parity gate pass)
- `codex/v1-phase3` -> `main` only after full Phase 3 acceptance matrix pass
4. Isolation rule:
- `i536` track stays separate
- no cherry-picks between tracks unless explicitly approved

## Major-surface parity set (mandatory)

1. Dashboard / briefing
2. Actions
3. Account detail
4. Project detail
5. Person detail
6. Meeting detail
7. Inbox / emails
8. Settings / data
9. Reports

## Wave sequence (locked)

| Wave | Scope | Status |
|---|---|---|
| Wave 0 | Kickoff + parity baseline + tracker + branch model | In progress |
| Wave 1 | I521 definition sprint + frontend contract ownership | In progress |
| Wave 2 | 3a backend cleanup: I515 then I514, plus I538 + I540 reliability fixes | Planned |
| Wave 3 | 3b GA platform: I427, I428, I429, I430, I431, I438 | Planned |
| Wave 4 | 3c then 3d: I502, I493, I447-I453 | Planned |
| Wave 5 | 3e: I529, I530, I537 | Planned |
| Wave 6 | Hardening + signoff + full acceptance matrix | Planned |

## Tracker matrix

| Issue | Depends on | Wave | Status | Validation gate |
|---|---|---|---|---|
| I521 | I536, I503, I508a | 1 | In progress | Contract registry + parity fixtures + `pnpm run test:parity` |
| I515 | I512 | 2 | Planned | ACs in `.docs/issues/i515.md` + pipeline failure/resume tests |
| I514 | I512 | 2 | Planned | ACs in `.docs/issues/i514.md` + boundary check + clippy/test |
| I538 | I511, I512 | 2 | Planned | Meeting refresh rollback ACs in `.docs/issues/i538.md` |
| I540 | I511, I512 | 2 | Planned | Actions pipeline integrity ACs in `.docs/issues/i540.md` |
| I427 | I511 | 3 | Planned | Search latency + parity gate |
| I428 | None | 3 | Planned | Degraded-mode rendering + parity gate |
| I429 | I511 | 3 | Planned | Export correctness + parity gate |
| I430 | None | 3 | Planned | Settings/Data copy + destructive action guardrails + parity gate |
| I431 | I435 | 3 | Planned | Cost model correctness + parity gate |
| I438 | None | 3 | Planned | Onboarding prime flow + parity gate |
| I502 | I499, I503 | 4 | Planned | Health rendering ACs + parity gate |
| I493 | I505, I502 | 4 | Planned | Account detail ACs + parity gate |
| I447 | I521 | 4 | Planned | Token audit ACs + parity gate |
| I454 | I521 | 4 | Planned | Vocabulary ACs + parity gate |
| I448 | I447, I521 | 4 | Planned | Actions editorial ACs + parity gate |
| I449 | I447, I521 | 4 | Planned | Week/emails editorial ACs + parity gate |
| I450 | I447, I521 | 4 | Planned | Portfolio chapter ACs + parity gate |
| I451 | I447, I521 | 4 | Planned | Meeting editorial ACs + parity gate |
| I452 | I447, I521 | 4 | Planned | Settings editorial ACs + parity gate |
| I453 | I447, I521 | 4 | Planned | Onboarding editorial ACs + parity gate |
| I529 | I507, I513 | 5 | Planned | Feedback UI ACs + parity gate |
| I530 | I529 | 5 | Planned | Taxonomy ACs + signal weight assertions |
| I537 | None | 5 | Planned | Feature-flag gate ACs + parity gate |

## Production-data parity gate contract

1. Canonical registry:
- `src/parity/phase3ContractRegistry.ts`
- `.docs/contracts/phase3-ui-contract-registry.json`
2. Fixture datasets:
- `.docs/fixtures/parity/mock/*.json`
- `.docs/fixtures/parity/production/*.json`
3. Test command:
- `pnpm run test:parity`
4. CI gate:
- `.github/workflows/test.yml` includes explicit parity step
5. Fail condition:
- Any major surface that passes mock but fails production-shape is release-blocking

## Release signoff criteria (Phase 3)

1. Every Phase 3 issue marked done has linked acceptance evidence.
2. `pnpm run test:parity` passes on umbrella before merge to `main`.
3. Full frontend tests pass (`pnpm test`).
4. Rust quality gates pass for backend waves (`cargo test`, strict clippy).
5. No unresolved parity exceptions for actions/proposed actions visibility on production-shaped data.
