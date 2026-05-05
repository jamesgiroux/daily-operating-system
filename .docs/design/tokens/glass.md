# Glass tokens

**Tier:** tokens
**Status:** canonical
**Owner:** James
**Last updated:** 2026-05-05
**`data-ds-name`:** `GlassTokens`
**`data-ds-spec`:** `tokens/glass.md`
**Design system version introduced:** 0.4.0

## Job

Glass tokens define the frosted backdrop treatment for fixed app chrome. They make FolioBar and FloatingNavIsland feel anchored above the page without inventing new translucent surfaces.

## Tokens

- `--backdrop-blur` - shared backdrop-filter blur.
- `--frosted-glass-background` - FolioBar background.
- `--frosted-glass-nav` - FloatingNavIsland background.

## Usage

Reserve glass for persistent app chrome and intentional overlays. Content cards and report slides should use surface/background tokens instead of frosted glass.

## Source

- **Runtime CSS:** `src/styles/design-tokens.css`
- **Reference CSS export:** `.docs/design/reference/_shared/styles/design-tokens.css`
- **Legacy grouping:** these tokens were previously documented inside `motion.md`.
