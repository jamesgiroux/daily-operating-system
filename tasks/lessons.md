# Lessons

## 2026-05-09 - Treat headless DailyOS as product positioning

When writing DailyOS GTM, do not reduce the product to meeting prep, Customer Success, or the Tauri app. Position DailyOS as the personal intelligence layer for work: memory plus judgment, exposed through both the app and the MCP/headless surface so agents can work from the user's priorities, trust context, and current working understanding.

## 2026-05-09 - Sell "it should just know," not the proof machinery

When writing DailyOS positioning, treat trust, provenance, claims, receipts, and abilities as table-stakes infrastructure rather than the main buyer promise. The public promise should cash out as "it should just know": the system prepares before prompts, remembers corrections, ranks what matters, stays current, and feeds agents the user's already-assembled working context.

## 2026-05-09 - Name maintenance tax as the enemy

For DailyOS GTM, do not limit the contrast to second-brain tools or meeting-prep apps. The broader enemy is any work tool whose use increases upkeep: project management systems, to-do apps, databases, dashboards, and AI tools that make the user organize, prompt, or reconcile before getting value. Keep BYOT/BYOM as a model-agnostic proof point, not the whole pitch.

## 2026-05-09 - Let DailyOS positioning pages be compact promise essays

For the DailyOS website, avoid a heavy build-log or generic docs feel. Use the working app/demo, a public-beta interest form, and short statement-led positioning pages or posts around core promises like "memory and judgment," "it should just know," and "start from what matters." Keep them crisp enough to communicate the idea without turning every page into a manifesto.

## 2026-05-09 - Keep foundation docs company-agnostic

When foundation docs describe product positioning, avoid naming the user's employer, internal operating model, or one current workplace tool as the default architecture. Describe the category first, then mention vendors only as examples when needed.

## 2026-05-09 - Distinguish platform names from customer identifiers

When sweeping redactions in docs or generated reference material, judge whether the term names a software platform category/source system or a specific customer/account. Salesforce is acceptable when the sentence clearly refers to the CRM platform; keep redaction discipline for actual customer identifiers or account-specific details.

## 2026-05-09 - QA native macOS app surfaces as desktop software

DailyOS is a native macOS app, so briefing surface design gates should use macOS window-size QA instead of mobile responsive requirements. Do not import web/mobile acceptance criteria unless the ticket explicitly names a mobile or web target.

## 2026-05-09 - Prefer reuse over regeneration

When migrating or redesigning an existing surface, first inventory the current producers, services, abilities, DTOs, tests, and UI primitives. Reuse working pieces and adapt them to the abilities runtime instead of recreating parallel commands, composers, or data paths. Treat new bypasses around the abilities runtime as plan failures unless an explicit architecture review approves them.

## 2026-05-09 - Keep contextual evidence separate from target scope

When the user shares work from a later version or adjacent PR as background, use it to inform the current version's plan without shifting the target release. Explicitly ask what v1.4.2 can do to reduce downstream v1.4.3 risk instead of treating the referenced PR as the immediate implementation destination.

## 2026-05-09 - Choose proof surfaces by claim density and priority

When proving an intelligence contract, pick the surface where the product already depends on the contract most heavily. For v1.4.2, Account Detail is the right first proof surface because most claims are born, rendered, corrected, and trusted there. Reports are a compatibility consumer, not the primary validation surface.

## 2026-05-09 - Keep reports in the reports release

Do not let report compatibility become the first proof surface for entity intelligence. Keep report-specific source contracts, generation migration, authoring, shareability, export, and publish work in v1.4.8 unless an earlier release has a concrete regression to prevent. Earlier versions should expose reusable contracts that reports can later consume, not build report-specific adapters prematurely.

## 2026-05-09 - Expand existing roadmap premises instead of overwriting them

When a later discovery adds substrate or scope to an already-sound project, preserve the original project thesis and expand it rather than replacing the description wholesale. For version planning, call out what the original premise still owns and what the new upstream work makes newly possible.

## 2026-05-09 - Primary proof surface does not collapse sibling scope

When a version names one surface as the primary proof surface, verify whether sibling surfaces are also in the release title or acceptance scope. Make the primary surface the richest proof, but create explicit cutover criteria for every in-scope surface so agents do not downgrade them to backend-only compatibility checks.

## 2026-05-09 - Be explicit about review-only vs review-and-edit

When asked to review a local instruction or planning document, state whether the pass is read-only or whether straightforward fixes will be applied. If the user expects the document to be updated and the findings are small, patch the document after presenting or identifying the issues rather than leaving the cleanup implicit.

## 2026-05-09 - Mirror paired instruction files

When updating repo-level agent guidance, check for paired instruction files such as `AGENTS.md` and `CLAUDE.md`. If they carry the same operational policy, update both in the same pass or explicitly call out why one should diverge.

## 2026-05-09 - Use AGENTS.md as the review ladder authority

When running DailyOS L0/L2/L3 reviews, use AGENTS.md as the source of truth for lanes, approval rules, and authority surfaces. gstack skills such as plan-eng-review can support a lane, but they do not replace the required review ladder or unanimity standard.

## 2026-05-09 - Keep agent instruction files local-only

Treat `CLAUDE.md` and `AGENTS.md` as local agent instruction files for this workspace, not source-controlled project documentation. If either file appears as untracked or modified in git status, ignore it via `.gitignore` rather than staging it.

## 2026-05-09 - Keep Figma artifacts out of runtime source unless shipped

When Figma MCP produces design-only exports, screenshots, mapping notes, or reference assets, keep them under `.docs/design/figma/`. Use `src/assets/` only for assets imported by the shipped app, and `public/` only for assets that must be served directly.

## 2026-05-09 - Mirror Linear wave changes into local plans

When Linear becomes canonical for a new wave, milestone, or stop gate, update the local `.docs/plans/` wave plan too. Local plans are still used by implementation sessions, so stale wave tables can cause agents to skip newly added pause/review points.
