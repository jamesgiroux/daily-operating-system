# ADR-0076: Brand Identity

**Date:** 2026-02-14
**Status:** Accepted
**Supersedes (partial):** ADR-0073 section 2 ("Warm Restraint — Color as State")

## Context

ADR-0073 codified the editorial design language — typography as architecture, breathing room, cards only for featured content. That decision is correct and remains binding. But the color palette it inherited (cream, charcoal, gold, sage, peach) was a UI token set, not a brand identity. Five flat colors with functional roles, no hierarchy between them, no story behind them.

A mood board exercise (Feb 2026) revealed a clear direction: warm earth tones, tactile materials, late 1960s/70s techno-humanism (Expo 67, world fair posters, botanical modernism). The consistent thread: things that feel *made*, not rendered. Embossed paper, spice jars, frosted botanicals, postcards on cream stock. This is upstream of design language — it's the identity that the design language serves.

Simultaneously, a brand positioning insight emerged from the product's own name. **DailyOS** carries "OS" — the same suffix as MS-DOS, the operating system that defined the personal computer era. DOS was the last time computing felt truly personal: one user, one machine, a direct relationship with the prompt. Computing has since become networked, then surveilled, then intermediated by platforms. DailyOS reclaims that directness — not by going backward, but by using AI to restore the personal relationship that SaaS eroded.

This decision establishes the brand identity that all visual, verbal, and product decisions inherit from.

## Decision

### 1. Brand Positioning: Personal Computing, Reclaimed

The personal computer was a revolution because it was *yours*. You sat at it. It responded to you. Your files lived on your disk. Nobody was watching, monetizing, or intermediating.

Then computing became corporate. Then cloud. Then surveillance. Your files live in someone else's database. Your tools are subscriptions to platforms that own your data. Your "personal" computer is a thin client to services that serve their shareholders, not you.

**DailyOS reclaims the personal computer.**

Not by going backward to command lines and CONFIG.SYS. By going forward: AI as the engine that makes computing personal again — knowing your context, maintaining your intelligence, operating on your behalf — while keeping everything local, in files you own, on a machine you control.

**Positioning concept:** *The computer is personal again.*

This framing captures the product's core promise without nostalgia for its own sake. It says: computing lost something important in the transition to cloud/SaaS, and AI-native local-first design gets it back. Not a return to the past — a synthesis of what the personal computer got right (ownership, directness, privacy) with what AI makes newly possible (intelligence, automation, synthesis).

**The DOS lineage is a brand asset, not just a naming coincidence.** The prompt. The cursor. The wildcard. The file system. These are heritage marks that DailyOS can reference with warmth and intention — the visual language of a direct relationship between human and machine.

### 2. Color System: Four Families, Material Names

The palette expands from 5 flat tokens to a layered system of color families. Every color is named after something you could touch, smell, or grow. Not hex codes — materials.

#### Paper (Grounds & Surfaces)

The page you're reading. Cream dominates — 80%+ of pixels on any surface.

| Name | Hex | Role |
|------|-----|------|
| **Cream** | `#f5f2ef` | Primary background. The default. |
| **Linen** | `#e8e2d9` | Secondary surface. Sidebar backgrounds, alternate rows, subtle depth. |
| **Warm White** | `#faf8f6` | Elevated surface. Cards, modals, surfaces that float above cream. |

#### Desk (Frame & Structure)

The surface the page sits on. Dark tones that frame warm content. Evolves from pure charcoal toward a warmer, slightly navy-tinged dark.

| Name | Hex | Role |
|------|-----|------|
| **Charcoal** | `#1e2530` | Primary dark. App chrome, sidebar, primary text. Warmer than pure black, slight blue undertone like a dark desk surface. |
| **Ink** | `#2a2b3d` | Blue-black. Deep code backgrounds, secondary dark. Named for the material that makes the page legible — editorial heritage, not UI abstraction. |
| **Espresso** | `#3d2e27` | Warm brown-black. Tertiary dark for depth, hover states on dark surfaces. |

#### Spice (Warm Accents)

Heat, attention, urgency, importance. These are the colors that say "look here" and "act now." Used sparingly — no more than 10-15% of any viewport.

| Name | Hex | Role |
|------|-----|------|
| **Turmeric** | `#c9a227` | Primary accent. Active state, customer meetings, priority items. The flagship color — warm gold that reads as intention, not decoration. Inherits gold's functional role. |
| **Saffron** | `#deb841` | Secondary warm. Lighter gold for hover states, highlights, secondary emphasis. |
| **Terracotta** | `#c4654a` | Attention/warning. Overdue items, risk signals, needs-action states. Warmer and more grounded than the previous peach (#e8967a). Reads as "earth alerting you" not "error." |
| **Chili** | `#9b3a2a` | Deep warm red. Critical/destructive actions, severe warnings. The exclamation point of the palette — used rarely, noticed always. |

#### Garden (Cool Accents)

Calm, growth, health, completion. The counterweight to spice — these colors say "you're good" and "take your time."

| Name | Hex | Role |
|------|-----|------|
| **Sage** | `#7eaa7b` | Success/complete/healthy. Slightly earthier than the original (#7fb685) — more garden, less UI green. |
| **Olive** | `#6b7c52` | Secondary cool. Categories, labels, subtle contextual markers. Mossy, understated. |
| **Rosemary** | `#4a6741` | Deep green. Pressed/hover states on green elements, deep accents, the serious side of garden. |
| **Larkspur** | `#8fa3c4` | Light blue. Informational, ephemeral, atmospheric. Named for the dawn-blooming flower (larks are morning birds — "up with the lark"). Adds airiness and lightness to the palette without competing with spice warmth. Used for secondary informational states, background tints, decorative touches. |

#### Entity Color Mapping

Each entity type in DailyOS owns one accent color. This is the color of its accent bar on meeting cards, its icon tint, its page-level accent, and its presence in briefing content. Entity color is **identity** — it answers "what kind of thing is this?" State colors (sage for success, terracotta for attention) still apply independently as **status** — they answer "what state is this thing in?"

| Entity | Color | Why |
|--------|-------|-----|
| **Accounts** | Turmeric | The flagship entity for CS-first development. Warm, important, customer-facing. The color users see most. |
| **Projects** | Olive | Earthy, productive, grounded. Different register from account warmth — projects are about building, not relationship. |
| **People** | Larkspur | Light, relational, human. Blue is universally associated with people/social. Larkspur's airiness fits — people are the connective tissue, not the primary object. |
| **Actions** | Terracotta | Urgency, attention, doing. Actions are the thing that needs your hands. Terracotta says "act" without screaming "error." |

**Rule:** Entity color appears on identity elements (accent bars, icon fills, page headers). State color appears on status elements (pills, badges, progress indicators). They coexist — an account meeting card has a turmeric accent bar *and* a sage "prep ready" pill. The two systems don't conflict because they operate on different visual elements.

#### Core Briefing Palette

Briefing surfaces (daily briefing, weekly briefing, meeting intelligence reports) are the primary reading experience. They use a **constrained subset** of the full palette to maintain editorial calm. The full 14-color system is the brand; the briefing palette is its most disciplined application.

**Briefing-approved colors (7):**

| Color | Briefing Role |
|-------|--------------|
| Cream | Page background |
| Warm White | Card/elevated surfaces |
| Charcoal | Body text, headings |
| Turmeric | Primary accent — priority numbers, accent bars, focus callout border |
| Terracotta | Attention state — overdue items, needs-prep indicators |
| Sage | Success state — complete, ready, healthy |
| Larkspur | Informational — secondary context, time-based labels, atmospheric touches |

**Not on briefing surfaces:** Linen, Ink, Espresso, Saffron, Chili, Olive, Rosemary. These exist for entity detail pages, settings, error states, and extended UI — not the core reading experience.

**The effect:** Every briefing page feels like the same publication. You never wonder "am I in a different app?" when moving from daily → weekly → intelligence report. The constrained palette creates the visual consistency of a single editorial voice. Entity detail pages can be richer — they're reference material, not briefings.

#### Color Family Rules

- **Paper fills the page.** 80%+ of any viewport is Paper family.
- **Desk frames the content.** Sidebar, header chrome, text. The gallery effect: warm content presented against a dark surface.
- **Spice draws attention.** No more than 10-15% of viewport. If everything is spice, nothing communicates.
- **Garden confirms and calms.** Success indicators, health signals, completion states.
- **Every color earns its pixel.** A colored element must communicate state, not decoration. If you remove the color and the meaning is unchanged, the color was decorative — remove it.
- **Cross-family pairing:** Spice on Paper (primary pattern), Garden on Paper (secondary pattern), Spice on Desk (sidebar highlights). Never Spice on Garden or Garden on Spice — the families don't mix directly.

### 3. Material Naming Convention

Every color in the system is named after something organic: a spice, a plant, a natural material. This is not whimsy — it's a design constraint that keeps the palette grounded in the physical world.

**When adding colors in the future, the name must pass the material test:** Can you hold it, grow it, grind it, or brew it? If the name is abstract (e.g., "primary," "accent-warm," "info-blue"), it fails. Colors are materials, not tokens.

This convention reinforces the Expo 67 sensibility: technology in service of the human, the natural, the tangible. The palette should feel like it was derived from a kitchen garden, not a design system generator.

### 4. The Asterisk Mark

The brand mark is a stylized asterisk (`*`). This carries three layers of meaning:

**1. The DOS wildcard.** In MS-DOS, `*.*` means "everything" — every file, every type. DailyOS sees everything in your day: every meeting, every email, every action, every relationship.

**2. The editorial footnote.** In typography, `*` marks "there's more context here." DailyOS is the context engine — the footnote system for your professional life. There's always more to know, and DailyOS knows it.

**3. The sunrise.** Visually, an asterisk is radiant lines from a center point. A starburst. A dawn. DailyOS runs at 6am. Your day starts when the briefing lands. The mark is the moment the sun breaks the horizon and your day is ready.

**Reference glyph: Montserrat ExtraBold asterisk.**

The mark is sourced from (or directly uses) the asterisk glyph in Montserrat ExtraBold. This was selected after evaluating asterisks across Impact (1965), Fraunces, EB Garamond, Cormorant Garamond, and Montserrat at multiple weights. The selection criteria:

- **6 points** — more starburst/sunrise than 5-pointed alternatives. Carries the `*` character association clearly.
- **Pointed petals, not rounded** — straight-edged, diamond-shaped rays that taper to tips. Reads as radiant light, not a flower.
- **Full weight without blobbing** — ExtraBold is the sweet spot. Bold is too thin for presence; Black merges the petals into a shapeless mass. ExtraBold maintains clear negative space between petals while carrying substantial visual weight.
- **Slight geometric composure** — more balanced than Impact's off-axis rotation, but inherits the spirit of a 1960s-era type design (Montserrat is inspired by mid-century Buenos Aires signage). The mark can be intentionally rotated a few degrees in specific brand applications to introduce the organic "helter-skelter" quality when appropriate.
- **Holds at all sizes** — legible as a 6-pointed asterisk from 512px app icon down to 32px favicon. Below 16px, simplify to a filled dot or single `*` character.

**Execution rules:**
- The mark can be used directly as the Montserrat ExtraBold `*` glyph or refined into a custom SVG that preserves the same proportions and petal shape.
- Works in any palette color on any background (see entity color mapping for which color in which context).
- Can be paired with the wordmark "DailyOS" or stand alone.
- In typographic/editorial contexts, a literal `*` in JetBrains Mono serves as the inline version of the mark — the typed character as brand element.
- The asterisk as playground: slight rotation, color variation, scale changes, and context-specific treatments are encouraged. The mark is a living element, not a rigid lockup.

**What the asterisk replaces:** The current app icon (gold lightning bolt on charcoal). The bolt communicates speed/energy but not the product's actual story. The asterisk communicates: everything (`*.*`), context (`*`), and dawn — all core to what DailyOS is.

### 5. DOS Heritage as Brand Texture

The DOS lineage isn't the whole brand, but it's a recurring texture — a secondary visual language that surfaces in specific contexts:

- **The prompt (`>_`):** Can appear as a motif in loading states, empty states, or onboarding. "Ready for you." The cursor blinks. The system is listening.
- **Monospace moments:** JetBrains Mono (already in the type stack) carries terminal heritage. Timestamps, data labels, metadata — anywhere precision matters, the monospace treatment connects to the command-line ancestry.
- **The wildcard:** `*` as a recurring motif beyond the logo. Section breaks. List markers. The finite briefing end-marker could be `* * *` (the typographic section break that also reads as three wildcards).
- **File paths as metaphor:** The workspace structure (`~/Accounts/Acme/`) is literally a file system. This isn't hidden — it's celebrated. Your data has an address. You can visit it. You own the building.

**What to avoid:** Literal green-on-black terminal screens. Pixel fonts. "Hacker aesthetic." DOS is referenced as heritage — warm, nostalgic, human — not as retro kitsch. Think "your grandfather's first computer" not "Mr. Robot."

### 6. Relationship to ADR-0073

This ADR **supersedes ADR-0073 section 2** ("Warm Restraint — Color as State"). The five-color flat palette is replaced by the four-family material system. The color budget rule (10-15% accent) carries forward.

**Everything else in ADR-0073 remains binding:**
- Typography as architecture (Newsreader / DM Sans / JetBrains Mono)
- Breathing room specifications
- Cards for featured content only
- Dark chrome / warm content framing
- Finite briefing pattern
- Pills over badges

The brand identity is upstream. ADR-0073 is the design language that implements it. This ADR answers "what does DailyOS feel like?" — ADR-0073 answers "how does that feeling manifest in UI?"

## Consequences

**Easier:**
- Every design decision has a brand story to reference, not just a hex code
- Material naming makes the palette memorable and communicable ("make that terracotta" vs. "make that #c4654a")
- The four-family structure scales — new colors slot into families with clear rules
- The asterisk mark carries deep meaning at multiple levels, not just "it looks nice"
- DOS heritage gives DailyOS a lineage and a differentiator — no other productivity tool can claim this positioning

**Harder:**
- 14 named colors is more complex than 5 — Tailwind config, component tokens, and documentation need updating
- Material names require discipline — contributors must name within the convention
- The asterisk mark needs careful execution to avoid reading as generic (many products use star/asterisk shapes)
- Brand positioning around "personal computing" must be articulated carefully to avoid political echo (the "Make X Y Again" construction is now culturally loaded — use declarative phrasing: "the computer is personal again" not "make computing personal again")
- Terracotta replacing peach changes the emotional register of warnings from "soft alert" to "grounded urgency" — some UI states may feel heavier

**Trade-offs:**
- Richer palette vs. simpler system: we choose richness. Five colors was limiting expression. Fourteen colors with family rules is manageable.
- Specific hex values vs. vibes: the hex values in this ADR are directional. They need visual testing against actual UI surfaces before becoming canonical in code. The families and names are the decision; exact values are implementation.
- Asterisk vs. bolt: we choose meaning over energy. The bolt said "fast." The asterisk says "everything, context, dawn." The product's story is the latter.
