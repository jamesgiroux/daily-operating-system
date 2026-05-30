# v1.4.5 W2 — Ingestion Pipeline retro

**Wave version:** v1.4.5 W2 — Ingestion Pipeline
**Closed:** 2026-05-22 (PR [#357](https://github.com/jamesgiroux/daily-operating-system/pull/357) W2-D merged last)
**Author:** orchestrator (Claude Code)
**Linear project:** [v1.4.5 — Workspace Memory Refactor](https://linear.app/a8c/project/v145-workspace-memory-refactor-cdb9d2c17102)
**Sourced from:** W2 L0 packets + L2 review transcripts + git history + wave-shared contract doc

## What shipped

| Lane | Spec | PR | L1 | L2 |
|---|---|---|---|---|
| **W1-extension** `IngestionMode::EntitySeeded + Realtime` | (W2 L1 precondition; no separate Linear) | [#350](https://github.com/jamesgiroux/daily-operating-system/pull/350) | PASS | UNANIMOUS APPROVE |
| **W2-A** `IngestPipeline` + abilities-runtime bridge | [DOS-466](https://linear.app/a8c/issue/DOS-466) | [#352](https://github.com/jamesgiroux/daily-operating-system/pull/352) | PASS (2 commits: feat + L2 cycle-1 codex BLOCK fold) | UNANIMOUS APPROVE after cycle-1 BLOCK fold |
| **W2-B** Route mutations through `IngestPipeline` | [DOS-467](https://linear.app/a8c/issue/DOS-467) | [#353](https://github.com/jamesgiroux/daily-operating-system/pull/353) | PASS (2 commits: feat + L2 cycle-1 BLOCK fold) | UNANIMOUS APPROVE after cycle-1 BLOCK fold |
| **W2-C** `entity_intake` ability + WP block scaffold | [DOS-468](https://linear.app/a8c/issue/DOS-468) | [#356](https://github.com/jamesgiroux/daily-operating-system/pull/356) | PASS (4 commits: feat + 3 CI gate fix passes) | UNANIMOUS APPROVE; 3 cycles on CI gates |
| **W2-D** `assign_inbox_entity` + full `_inbox` refactor | [DOS-469](https://linear.app/a8c/issue/DOS-469) | [#357](https://github.com/jamesgiroux/daily-operating-system/pull/357) | PASS (3 commits: feat + L2 cycle-1 BLOCK fold + ability allowlist) | UNANIMOUS APPROVE after cycle-1 BLOCK fold |

**Stage 2a** (DOS-466 alone) → **Stage 2b** (DOS-467/468/469 parallel). Sequencing held — no cross-lane file conflicts, no stage 2b PR opened before stage 2a merged.

### W2 architectural deliverables

- **`IngestPipeline::run`** is the single workspace mutation path. Stages: canonicalize → register → classify → link → extract → propose → index → signal → run-status. Each stage idempotent.
- **`IngestionMode`** extended in W1-ext PR #350 with `EntitySeeded` + `Realtime` variants (W2 L1 precondition).
- **Mutation paths consolidated:** inbox, Drive, watcher, attachments all route through the service. Two explicit allowlist exceptions remain (transcript writes + Drive remote-bytes staging) — both filed to v1.4.6: [DOS-767](https://linear.app/a8c/issue/DOS-767), [DOS-768](https://linear.app/a8c/issue/DOS-768).
- **`entity_intake` ability + Gutenberg block scaffold** for entity-seeded source intake on account/person/project pages. Ships read-only — editor write transport deferred to v1.4.6: [DOS-769](https://linear.app/a8c/issue/DOS-769).
- **`_inbox` refactored** into unresolved-source queue with `assign_inbox_entity` command driving user lifecycle decisions. Legacy `processor/mod.rs` + `processor/router.rs` consolidation deferred: [DOS-770](https://linear.app/a8c/issue/DOS-770).

## What broke + how we recovered

### W2-A codex L2 cycle-1 BLOCK: frontmatter doc_type terminal

**Original framing:** `IngestPipeline` extraction stage classified file `doc_type` from frontmatter and bridged to claim production via `extract_stub`. Frontmatter parsing tolerant of variant casing/whitespace.

**Reality:** Codex L2 found the frontmatter `doc_type` detection was not terminal — when present in YAML frontmatter the value could still be overwritten by content-sniffing fallback later in the pipeline. Closed by treating frontmatter `doc_type` as authoritative when well-formed. Fixed in cycle-1 fold (commit `d580896c`); doc comment corrected in `b23d558d`.

### W2-B L2 cycle-1 BLOCK: user-attachment processor trigger regressed

**Original framing:** Refactor every mutation path to route through `IngestPipeline::run`. User attachments go through pipeline like everything else.

**Reality:** The naive refactor accidentally removed the user-attachment processor trigger that classifies attachments into `_inbox` for user review. L2 caught it; restored the trigger as part of the pipeline path while keeping the underlying mutation through the service. Fixed in commit `82808606`.

### W2-C three CI-gate cycles

**Original framing:** Ship `entity_intake` ability + WP block scaffold; PHPCS, ability inventory, block fixture, and token mapping all pass first cycle.

**Reality:** Three CI gate fix passes (`132cb6b7`, `c2e9df24`, `4fdd1ce1`). Findings layered:
- Pass 1: PHPCS docblocks missing on new PHP, ability inventory drift, block fixture missing.
- Pass 2: token mapping incomplete, remaining pattern PHPCS errors.
- Pass 3: `phpcbf` auto-fixes + remaining manual docblock additions.

L2 unanimous APPROVE held throughout; the cycles were structural CI fixes against the package scaffold, not L2 findings. **Class pattern:** new WP-block scaffold work needs the full PHPCS + ability inventory + fixture gate run locally before the first push. Worth a learning entry — three cycles of CI thrash to fix what a `pnpm wp:scaffold-check` (if it existed) would catch pre-push.

### W2-D L2 cycle-1 BLOCK: inbox-refactor scope expansion

**Original framing:** Add `assign_inbox_entity` command; surface affordance in the unresolved queue UI.

**Reality:** L2 cycle-1 BLOCK landed on a broader pattern — the lifecycle transitions, ability-surface drift allowlist, and underlying refactor of `_inbox` semantics needed to ship together to satisfy the AC, not as a Phase-2 deferral. Folded in cycle-1 (commit `57693adb`). Ability surface drift allowlist entry added in `b5f9d307` to close the static-check.

## K-out class patterns

Four patterns worth capturing:

1. **L2 cycle-1 BLOCK folded on every W2 lane.** W2-A, W2-B, and W2-D all needed a cycle-1 BLOCK fold before unanimous APPROVE. The pattern was acceptance criteria implied richer behavior than the first L1 pass shipped (frontmatter-terminal, processor trigger restoration, fuller inbox refactor). Suggests W2 L0 packets should pin "what counts as cycle-1 PASS" more sharply — fewer but higher-bar AC items.
2. **WP block scaffold needs a pre-push gauntlet.** W2-C's 3-cycle CI thrash was entirely PHPCS + ability inventory + block fixture issues a pre-push check would catch. A `pnpm wp:scaffold-check` script that runs the same gates locally would prevent the burn.
3. **Stage ordering held.** Cycle-2 amendment's W2 internal staging (2a alone, then 2b parallel) was respected end-to-end. The file-disjoint claim from L0 held — no cross-lane file conflicts during stage 2b parallel implementation. **The cycle 4 trait-relocation discipline from W1 paid off here:** because `Extractor` and `SignalEmitter` live in W1-A's `contracts.rs`, W2-B/C/D could implement against a stable trait surface without W2-A's pipeline file becoming a contended write.
4. **Allowlist exceptions are debt, not solutions.** W2-B explicitly allowlisted transcript and Drive direct writes (`dos7-allowed: transcript-direct-write-v146`, `dos7-allowed: drive-staging-v146`) rather than refactor. Both filed as v1.4.6 follow-ups (DOS-767, DOS-768). The allowlist mechanism is the right escape hatch for known-bypass-with-known-resolution, but every exception is a known bypass and the resolution ticket should land in the same wave that adds the exception.

## v1.4.6 follow-ups filed

Four tickets carry forward from W2:

| Ticket | Source | Priority |
|---|---|---|
| [DOS-767](https://linear.app/a8c/issue/DOS-767) | W2-B transcript content staging | Medium |
| [DOS-768](https://linear.app/a8c/issue/DOS-768) | W2-B Drive remote-bytes staging | Medium |
| [DOS-769](https://linear.app/a8c/issue/DOS-769) | W2-C editor write-transport for entity-intake block | High |
| [DOS-770](https://linear.app/a8c/issue/DOS-770) | W2-D legacy processor/router consolidation | Medium |

## What's next

**W3 (DOS-470, 471, 489) is now unblocked.** W3-A (DOS-470) replaces the `extract_stub` W2-A shipped with the real `WorkspaceExtractor` impl. W3-A merges alone in stage 3a; W3-B + W3-C run in parallel in stage 3b. W3 L0 packet drafting starts next.

## Related

- W2 L0 packets: `L0-packet-W2-A-DOS-466.md`, `L0-packet-W2-B-DOS-467.md`, `L0-packet-W2-C-DOS-468.md`, `L0-packet-W2-D-DOS-469.md`
- W2 shared contract: `W2-shared-contract.md`
- W2 L2 review transcripts under `l2-reviews/`
- W1 retro: `retro-W1.md`
- v1.4.6 follow-up notes: `v1.4.6-followups-W2-{B,C,D}.md`
