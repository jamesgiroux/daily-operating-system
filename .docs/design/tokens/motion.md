# Motion tokens

**Tier:** tokens
**Status:** canonical
**Owner:** James
**Last updated:** 2026-05-02
**Design system version introduced:** 0.1.0

## Job

DailyOS motion vocabulary — durations, easings, transitions, animations. The system is restrained: editorial surfaces don't dance.

## Transitions

- `--transition-fast`   `0.12s ease`  — hover state changes, button interactions
- `--transition-normal` `0.15s ease`  — color and background transitions, default

## Backdrop & glass

- `--backdrop-blur` `blur(12px)` — frosted glass effect (FolioBar, FloatingNavIsland)
- `--frosted-glass-background` `rgba(245, 242, 239, 0.85)` — FolioBar background
- `--frosted-glass-nav` `rgba(250, 248, 246, 0.8)` — FloatingNavIsland background

## Z-index stack

- `--z-atmosphere`    `0`    — background AtmosphereLayer
- `--z-page-content`  `1`    — main content above atmosphere
- `--z-app-shell`     `100`  — FolioBar, FloatingNavIsland, top-level UI
- `--z-lock`          `1000` — app lock overlay (above everything, per I465)

## Shadows

Treated as motion-adjacent because shadow size signals elevation/interactivity:

- `--shadow-sm`        `0 2px 8px rgba(30, 37, 48, 0.1)`
- `--shadow-md`        `0 2px 12px rgba(30, 37, 48, 0.06), 0 0 0 1px rgba(30, 37, 48, 0.04)`
- `--shadow-lg`        `0 4px 12px rgba(0, 0, 0, 0.08)`
- `--shadow-xl`        `0 4px 16px rgba(0, 0, 0, 0.12)`
- `--shadow-2xl`       `0 8px 32px rgba(0, 0, 0, 0.12)`
- `--shadow-modal`     `0 20px 60px rgba(30, 37, 48, 0.15)`
- `--shadow-dropdown`  `0 4px 24px rgba(0, 0, 0, 0.08)`

## Keyframe animations

Defined in `src/styles/design-tokens.css`:

- `atmosphere-breathe` — slow opacity pulse for AtmosphereLayer (0.8 → 1)
- (`mark-pulse` lives in chrome substrate — slow opacity pulse for FolioBar brand mark when "live" status is on)

## When to use which

- **Hover effects** → `--transition-fast`
- **Color/background changes (selection, focus)** → `--transition-normal`
- **Frosted-glass surfaces** → use `--frosted-glass-*` tokens, not raw rgba
- **Layered surfaces** → use `--z-*` tokens; never magic z-index numbers
- **Shadows** → semantic name (`shadow-modal` for modals, `shadow-dropdown` for dropdowns); avoid raw `box-shadow` rules where a token applies

## Conventions

- DailyOS motion is **calm**. No bouncing, springing, or attention-seeking animations on editorial surfaces.
- Only one element at a time should animate (atmosphere breathing is fine; chrome motion should be discrete state changes, not continuous).
- Prefer reduced motion respect: animations should be subtle enough that disabling them doesn't change the experience materially.

## Source

- **Code:** `src/styles/design-tokens.css`
- **Mockup substrate:** `.docs/mockups/claude-design-project/mockups/surfaces/_shared/tokens.css` + `chrome.css`

## History

- 2026-05-02 — Promoted to canonical.
- I465 — lock z-index established.
