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
4. **Vitals strip** (`heroVitals`): two-line cells — value line with optional
   semantic tint (`heroVitalText[data-tint]`) over a quiet mono source line
   (`heroVitalSource`): `$185K ARR / defends internally`,
   `Health: Watch / 1 workstream at risk`. Tint is semantic state, never
   decoration.

## Variants

- **person**: avatar always, `heroAvatarRing` when actively tracked; tier
  chip (Champion / Detractor / …); related = account.
- **account**: variant chip customer/partner/internal; vitals = ARR,
  contract end, NPS, lifecycle (snapshot-fed, editable).
- **project**: variant chip = engagement kind; phase + counts in sub-meta;
  vitals = phase, health, target date, owner.

## When NOT to use it

- Non-entity surfaces (briefing, reports) — they have their own mastheads.
- Tabs/section nav inside the hero (mockups show tabs; superseded by the
  strip-chrome / pages-per-view direction — see `DayStrip.md` future note).

## Substrate

Hero payload comes from the entity composition producer (snapshot identity +
vitals with per-field provenance and edit routes, per the approved account
hero). The avatar/sub-meta facts are snapshot fields; vitals stay editable
through the snapshot-field correction path. Trust renders as opacity.

## Source

- **Mockups:** `~/Downloads/DailyOS Design System (2)/mockups/surfaces/{people,project}-detail/variations/D-composite.html`, briefing/account d-spine
- **Reference styles:** `.docs/design/reference/_shared/styles/EntityHeroBase.module.css` (core + 2026-06-10 unified-block additions)
- **Shipped consumer:** `src/components/composition/blocks/AccountHeroBlock.*` (approved account hero composes the base)
