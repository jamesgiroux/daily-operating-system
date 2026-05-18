# Figma Design System Rules

Use these rules for every Figma-driven UI implementation, Figma-to-code pass, or design-system sync.

## Authority order

- Linear issue/spec acceptance criteria are the task contract.
- `.docs/design/` is the DailyOS design-system contract: tokens, primitives, patterns, surfaces, naming, inventory, and audits.
- Shipped `src/` is the current behavior source when implementing or preserving existing UI. If docs and source disagree, read `.docs/design/_audits/shipped-component-inventory.md` before deciding whether source is behind or the docs over-promoted a prototype.
- `.docs/design/reference/` is the visual parity/reference layer. Use it to compare layout, typography, spacing, chrome, and state, but do not treat reference HTML as a replacement for the TSX/CSS module source.
- Existing Figma files are helpful but not authoritative until they are complete and reconciled with `.docs/design/` and shipped source.

## Required Figma MCP flow

- Run `get_design_context` for the exact node(s) before implementing from Figma.
- If context is too large or truncated, run `get_metadata`, identify the needed child node(s), then re-run `get_design_context` narrowly.
- Run `get_screenshot` for the same node/variant before coding.
- Treat MCP React/Tailwind output as a design representation, not project-ready code.
- When writing back to Figma, use `.docs/design/` specs and `.docs/design/reference/` renders as the seed; search existing Figma design-system assets first and repair/reuse them instead of recreating duplicates.

## Implementation rules

- Reuse existing components before creating new ones. Start in `src/components/ui`, `src/components/shared`, `src/components/editorial`, `src/components/layout`, `src/components/entity`, domain component folders, and `src/features/settings-ui`.
- New routed surfaces live in `src/pages`; shared primitives/patterns live under the appropriate `src/components/*` owner, with co-located CSS modules unless the existing component family uses Tailwind utilities.
- Prefer CSS modules and design tokens for editorial/product UI. Existing shadcn-style primitives may keep Tailwind/CVA, but do not paste raw Tailwind from Figma when a DailyOS primitive/pattern exists.
- Use `@/` imports, strict TypeScript props, functional components, `clsx` or `cn` for class composition, and existing hooks/services.
- Do not use a `proposed` primitive or pattern unless the issue explicitly promotes it. If promotion is required, update the markdown spec, source component/CSS, reference render where applicable, inventory/index, and tests together.
- Promoted design-system elements must expose `data-ds-tier`, `data-ds-name`, and `data-ds-spec`; add `data-ds-variant` or `data-ds-state` when the variant/state is meaningful.

## DailyOS visual rules

- DailyOS is a magazine, not a dashboard. Typography, spacing, and reading order should do most of the structural work.
- Cards are for featured content only; most content should be editorial rows, rules, lists, and sections.
- Color communicates state, entity identity, trust, or action. Do not add decorative color.
- Editorial pages should have finite endings and use `FinisMarker` unless the surface spec says otherwise.
- Preserve the magazine shell: `FolioBar`, `FloatingNavIsland`, `AtmosphereLayer`, and `MagazinePageLayout` conventions.
- Do not introduce raw pipeline vocabulary in user-facing copy (`enrichment`, `AI enrichment`, `intelligence pipeline`). Preserve canonical labels from specs/source when already established.

## Tokens and assets

- Runtime tokens live in `src/styles/design-tokens.css`; markdown specs live in `.docs/design/tokens/`; the reference mirror lives in `.docs/design/reference/_shared/styles/design-tokens.css`. Keep all three in sync when changing tokens.
- Never hardcode colors, fonts, spacing scales, shadows, radii, trust bands, or entity colors when a token exists.
- Design-only Figma exports, MCP notes, screenshots, and mapping artifacts belong under `.docs/design/figma/`.
- Runtime assets belong in `src/assets/` only when the app imports them; public-root assets belong in `public/` only when they must be served directly.
- Use Figma MCP localhost asset URLs directly during implementation. Do not commit placeholder assets, and do not add new icon packages; use existing `lucide-react` icons or the specific Figma-provided asset.
