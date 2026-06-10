# EntityHero

**Tier:** pattern
**Status:** proposed
**Owner:** James
**Last updated:** 2026-06-10
**`data-ds-name`:** `EntityHero`
**`data-ds-spec`:** `patterns/EntityHero.md`
**Variants:** person (avatar + ring); account (type variant); project (phase); avatar optional on account/project
**Design system version introduced:** 0.1.0

## Job

ONE hero block for every entity detail page (Person / Account / Project) —
the first block of the blocks model: pages stop being entity-specific
layouts and become blocks you add/remove/reposition via the customizer,
starting with this hero. Core anatomy is shared; entity differences are
variants, not separate heroes.

Synthesized (James, 2026-06-10) from the people D-composite hero (the
preferred shape), the project D-composite masthead, and the approved
account d-spine composition hero.

## Anatomy (core)

1. **Meta row** — mono/uppercase chrome above the title
   (`EntityHeroBase_heroDate`): freshness/quality badge; never per-line
   provenance (trust rides opacity, not chips).
2. **Identity row** (`heroIdentity`): optional **initials avatar**
   (`heroAvatar`, 72px serif initials; people always, accounts/projects
   optional) beside the serif name (`heroTitle`).
3. **Sub-meta line** (`heroSubMeta`): entity-variant chip + role/phase facts
   + related-entity chip, dot-separated (`heroSubMetaSep`):
   - person: `Champion · VP Digital Merchandising · @Meridian Harbor · NY · GMT−5`
   - account: `Customer · Enterprise · @Parent Co · tracked since Feb 2024`
   - project: `Customer co-build · Beta · 7 workstreams · 4 accounts`
Vitals are NOT part of this block (James, 2026-06-10) — the two-line
vitals strip from the D-composite mockups is a separate pattern
(`VitalsStrip`, extending the shipped component) composed below the hero.

## Variants

- **person**: avatar always, `heroAvatarRing` when actively tracked; tier
  chip (Champion / Detractor / …); related = account.
- **account**: variant chip customer/partner/internal.
- **project**: variant chip = engagement kind; phase + counts in sub-meta.

## When NOT to use it

- Non-entity surfaces (briefing, reports) — they have their own mastheads.
- Tabs/section nav inside the hero (mockups show tabs; superseded by the
  strip-chrome / pages-per-view direction — see `DayStrip.md` future note).

## Substrate

Hero payload comes from the entity composition producer (snapshot identity
fields, per the approved account hero). The avatar/sub-meta facts are
snapshot fields. Trust renders as opacity.

## Source

- **Mockups:** `~/Downloads/DailyOS Design System (2)/mockups/surfaces/{people,project}-detail/variations/D-composite.html`, briefing/account d-spine
- **Reference styles:** `.docs/design/reference/_shared/styles/EntityHeroBase.module.css` (core + 2026-06-10 unified-block additions)
- **Shipped consumer:** `src/components/composition/blocks/AccountHeroBlock.*` (approved account hero composes the base)
