# Typography tokens

**Tier:** tokens
**Status:** canonical
**Owner:** James
**Last updated:** 2026-05-02
**Design system version introduced:** 0.1.0

## Job

Font families for DailyOS, mapped to semantic roles. Set by ADR-0073.

## Families

- `--font-serif` `'Newsreader', Georgia, serif` — display, headlines, narrative prose, pull quotes
- `--font-sans` `'DM Sans', -apple-system, sans-serif` — body, UI text, labels, controls
- `--font-mono` `'JetBrains Mono', ui-monospace, monospace` — time, dates, code, eyebrow labels, keyboard shortcuts
- `--font-mark` `'Montserrat', sans-serif` — brand mark asterisk (must be weight 800)

## When to use which

- **Serif** — anything that should read like prose: hero titles, lede sentences, chapter headings, pull quotes, in-content headlines, meeting titles, account names. Usually with `font-weight: 300-500`. Sentence case unless mid-sentence.
- **Sans** — UI: button labels, body text, form controls, inline metadata, link copy. Default sans for interactive elements.
- **Mono** — when uniformity matters more than readability: timestamps ("3h ago"), times of day ("10:00"), dates ("APR 23"), eyebrow labels (`SECTION · UPPERCASE`), keyboard shortcut chips (`⌘K`), file names, code samples. Often paired with `text-transform: uppercase` and `letter-spacing: 0.06-0.14em`.
- **Mark** — brand asterisk only. Never for body or headlines.

## Conventions

- **Eyebrow labels** — mono, 10-11px, uppercase, letter-spacing 0.06-0.14em, color `--color-text-tertiary`. Used above section titles and on FolioBar labels.
- **Display** — serif, font-size 26-76px, font-weight 300-500, line-height 1.06-1.2, letter-spacing -0.005 to -0.025em.
- **Body** — sans, font-size 13-15px, line-height 1.4-1.65.
- **Italic** — serif italic for context lines and lede sentences. Sans italic for status text. Mono never italic.

## Reference type pairings

Used widely across editorial surfaces:
- Eyebrow (mono uppercase 10-11px) → Title (serif 26-76px) → Lede (serif italic 16-21px)
- Section heading (mono uppercase 10-12px) → Section summary (serif 17px)
- Meeting time (mono 18px) → Meeting title (serif 26px) → Meeting context (serif italic 16px)

## Source

- **Code:** `src/styles/design-tokens.css` (declarations); `src/styles/fonts.css` (`@font-face`)
- **Mockup substrate:** `.docs/mockups/claude-design-project/mockups/surfaces/_shared/{tokens,fonts}.css`

## History

- 2026-05-02 — Promoted to canonical.
- ADR-0073 — original typography definition.
