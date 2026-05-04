# Hero pattern consolidation audit

DOS-373. Per-hero audit of DailyOS's hero patterns with consolidation recommendation.

## Summary

- Audited: 7 hero-style patterns
- Recommend keep separate: 4 (3 entity-identity heroes + DailyBriefing's Lead pattern)
- Recommend collapse: 2
- Recommend refactor: 1

The main consolidation opportunity is not the entity heroes. `AccountHero`, `PersonHero`, and `ProjectHero` already share `EntityHeroBase.module.css` for the 76px editorial title, hero date, badges, archived banner, and metadata controls (`src/components/entity/EntityHeroBase.module.css:22`, `src/components/entity/EntityHeroBase.module.css:31`, `src/components/entity/EntityHeroBase.module.css:41`, `src/components/entity/EntityHeroBase.module.css:61`, `src/components/entity/EntityHeroBase.module.css:78`, `src/components/entity/EntityHeroBase.module.css:95`). Their remaining divergence is entity-specific data and editing behavior.

The biggest collapse target is `EditorialPageHeader`: its API and CSS nearly duplicate `SurfaceMasthead`, but with different names (`subtitle` vs `lede`, `meta` vs `accessory`, `children` vs `glance`) and a few size/rule variants. There is also a residual inline MeetingDetail hero in production code (`src/pages/MeetingDetailPage.tsx:1202`) even though the design docs say MeetingHero is already a `SurfaceMasthead` composition (`.docs/design/patterns/SurfaceMasthead.md:16`, `.docs/design/surfaces/MeetingDetail.md:26`).

## Catalog

### AccountHero

- **File:** `src/components/account/AccountHero.tsx` + `src/components/account/AccountHero.module.css`
- **Used in surfaces:** `/accounts/$accountId` via `AccountDetailPage` (`src/router.tsx:523`, `src/pages/AccountDetailPage.tsx:975`). Also still imported by the legacy/editorial page (`src/pages/AccountDetailEditorial.tsx:139`).
- **Structure:**
  - eyebrow: yes, implemented as the hero date row with `IntelligenceQualityBadge`, "Last updated", and optional account type picker (`src/components/account/AccountHero.tsx:71`, `src/components/account/AccountHero.tsx:72`, `src/components/account/AccountHero.tsx:82`).
  - title: account name in `h1`; editable through `editName`, `setEditName`, and `onSaveField("name", v)` (`src/components/account/AccountHero.tsx:90`, `src/components/account/AccountHero.tsx:91`, `src/components/account/AccountHero.tsx:93`, `src/components/account/AccountHero.tsx:96`).
  - lede: no current runtime lede. The comment says the narrative moved to `AccountExecutiveSummary` (`src/components/account/AccountHero.tsx:46`), though `.lede` CSS remains composed from the shared base (`src/components/account/AccountHero.module.css:15`).
  - accessory: no right-side accessory slot. Account type is inline in the eyebrow row; `vitalsSlot` and `provenanceSlot` render below the title (`src/components/account/AccountHero.tsx:26`, `src/components/account/AccountHero.tsx:27`, `src/components/account/AccountHero.tsx:28`, `src/components/account/AccountHero.tsx:106`, `src/components/account/AccountHero.tsx:108`).
  - watermark: no runtime `BrandMark` or pseudo-element observed. The file header still claims a watermark asterisk (`src/components/account/AccountHero.tsx:4`), but the TSX imports no `BrandMark` and the CSS only composes shared base classes (`src/components/account/AccountHero.tsx:6`, `src/components/account/AccountHero.tsx:13`, `src/components/account/AccountHero.module.css:3`).
  - entity-tint: account/turmeric via `--color-account` and account badge variants (`src/styles/design-tokens.css:86`, `src/components/account/AccountHero.module.css:47`, `src/components/account/AccountHero.module.css:193`).
  - other: parent breadcrumb (`src/components/account/AccountHero.tsx:51`), archived banner (`src/components/account/AccountHero.tsx:62`), editable account type dropdown (`src/components/account/AccountHero.tsx:121`).
- **Variants observed:** parent vs child account (`detail.parentId && detail.parentName`, `src/components/account/AccountHero.tsx:52`); archived vs active (`detail.archived`, `src/components/account/AccountHero.tsx:63`); editable vs read-only title (`src/components/account/AccountHero.tsx:92`); customer/internal/partner type badges (`src/components/account/AccountHero.tsx:115`); vitals omitted for internal accounts at call site (`src/pages/AccountDetailPage.tsx:981`); preset `EditableVitalsStrip` vs fallback `VitalsStrip` (`src/pages/AccountDetailPage.tsx:982`, `src/pages/AccountDetailPage.tsx:988`).
- **Recommendation:** keep separate. It is entity-identity chrome with account-specific edit paths, parent hierarchy, type semantics, and dossier vitals. Clean up stale watermark/lede comments later, but do not collapse into `SurfaceMasthead`.

### PersonHero

- **File:** `src/components/person/PersonHero.tsx` + `src/components/person/PersonHero.module.css`
- **Used in surfaces:** `/people/$personId` via `PersonDetailEditorial` (`src/router.tsx:675`, `src/pages/PersonDetailEditorial.tsx:227`).
- **Structure:**
  - eyebrow: yes, hero date row with `IntelligenceQualityBadge`, DailyOS update time, and optional Clay update time (`src/components/person/PersonHero.tsx:100`, `src/components/person/PersonHero.tsx:101`, `src/components/person/PersonHero.tsx:103`, `src/components/person/PersonHero.tsx:104`).
  - title: person name in `h1`, paired with `Avatar`; editable through `editName`, `setEditName`, and `onSaveField("name", v)` (`src/components/person/PersonHero.tsx:107`, `src/components/person/PersonHero.tsx:108`, `src/components/person/PersonHero.tsx:110`, `src/components/person/PersonHero.tsx:115`).
  - lede: first paragraph of `intelligence.executiveAssessment`, 300-character truncation, "Read more" expansion (`src/components/person/PersonHero.tsx:67`, `src/components/person/PersonHero.tsx:68`, `src/components/person/PersonHero.tsx:69`, `src/components/person/PersonHero.tsx:70`, `src/components/person/PersonHero.tsx:127`).
  - accessory: no top-right accessory slot. Entity-specific content follows inline: subtitle, bio, social links, badges, and meta actions (`src/components/person/PersonHero.tsx:141`, `src/components/person/PersonHero.tsx:163`, `src/components/person/PersonHero.tsx:170`, `src/components/person/PersonHero.tsx:207`, `src/components/person/PersonHero.tsx:219`).
  - watermark: no runtime `BrandMark` or pseudo-element observed. The file header still claims a larkspur-tinted watermark (`src/components/person/PersonHero.tsx:3`), but TSX imports `Avatar`, not `BrandMark` (`src/components/person/PersonHero.tsx:12`).
  - entity-tint: person/larkspur via avatar background/text, enriching state, and social links (`src/styles/design-tokens.css:88`, `src/components/person/PersonHero.module.css:18`, `src/components/person/PersonHero.module.css:108`, `src/components/person/PersonHero.module.css:155`).
  - other: role/email/org subtitle assembly (`src/components/person/PersonHero.tsx:74`), relationship and temperature badges (`src/components/person/PersonHero.tsx:35`, `src/components/person/PersonHero.tsx:41`, `src/components/person/PersonHero.tsx:207`), destructive delete action (`src/components/person/PersonHero.tsx:252`).
- **Variants observed:** archived vs active (`src/components/person/PersonHero.tsx:91`, `src/components/person/PersonHero.tsx:242`, `src/components/person/PersonHero.tsx:247`); external/internal/unknown relationship classes (`src/components/person/PersonHero.tsx:35`); hot/warm/cool/cold temperature classes (`src/components/person/PersonHero.tsx:41`); lede collapsed/expanded (`src/components/person/PersonHero.tsx:70`, `src/components/person/PersonHero.tsx:130`); Clay enrichment available/loading (`src/components/person/PersonHero.tsx:228`, `src/components/person/PersonHero.tsx:230`); optional bio/social/phone blocks (`src/components/person/PersonHero.tsx:163`, `src/components/person/PersonHero.tsx:170`); vitals are rendered outside the hero section by the page (`src/pages/PersonDetailEditorial.tsx:245`).
- **Recommendation:** keep separate. The person hero is not just eyebrow/title/lede; it is profile identity, avatar, relationship temperature, contact metadata, Clay enrichment, and merge/archive/delete workflow.

### ProjectHero

- **File:** `src/components/project/ProjectHero.tsx` + `src/components/project/ProjectHero.module.css`
- **Used in surfaces:** `/projects/$projectId` via `ProjectDetailEditorial` (`src/router.tsx:616`, `src/pages/ProjectDetailEditorial.tsx:272`).
- **Structure:**
  - eyebrow: yes, hero date row with `IntelligenceQualityBadge` and DailyOS update time (`src/components/project/ProjectHero.tsx:71`, `src/components/project/ProjectHero.tsx:72`, `src/components/project/ProjectHero.tsx:73`, `src/components/project/ProjectHero.tsx:74`).
  - title: project name in `h1`; editable through `editName`, `setEditName`, and `onSaveField("name", v)` (`src/components/project/ProjectHero.tsx:77`, `src/components/project/ProjectHero.tsx:78`, `src/components/project/ProjectHero.tsx:80`, `src/components/project/ProjectHero.tsx:83`).
  - lede: first paragraph of `intelligence.executiveAssessment`, 300-character truncation, "Read more" expansion (`src/components/project/ProjectHero.tsx:54`, `src/components/project/ProjectHero.tsx:55`, `src/components/project/ProjectHero.tsx:56`, `src/components/project/ProjectHero.tsx:57`, `src/components/project/ProjectHero.tsx:93`).
  - accessory: no top-right accessory slot. Status and owner render as badge row below lede/title (`src/components/project/ProjectHero.tsx:108`, `src/components/project/ProjectHero.tsx:109`, `src/components/project/ProjectHero.tsx:117`).
  - watermark: no runtime `BrandMark` or pseudo-element observed. The file header still claims an olive-tinted watermark (`src/components/project/ProjectHero.tsx:3`).
  - entity-tint: project/olive exists as a design token (`src/styles/design-tokens.css:87`), but the current hero mostly uses status colors. The only direct project tint in the module is `.metaButtonEnriching`, and that class is not selected by the current TSX (`src/components/project/ProjectHero.module.css:69`, `src/components/project/ProjectHero.tsx:126`).
  - other: archive/unarchive meta actions (`src/components/project/ProjectHero.tsx:124`, `src/components/project/ProjectHero.tsx:133`, `src/components/project/ProjectHero.tsx:138`).
- **Variants observed:** archived vs active (`src/components/project/ProjectHero.tsx:62`, `src/components/project/ProjectHero.tsx:133`, `src/components/project/ProjectHero.tsx:138`); editable vs read-only title (`src/components/project/ProjectHero.tsx:79`); status active/on_hold/completed/default (`src/components/project/ProjectHero.tsx:29`, `src/components/project/ProjectHero.module.css:28`, `src/components/project/ProjectHero.module.css:33`, `src/components/project/ProjectHero.module.css:38`, `src/components/project/ProjectHero.module.css:43`); owner badge optional (`src/components/project/ProjectHero.tsx:117`); preset `EditableVitalsStrip` vs fallback `VitalsStrip` outside the hero (`src/pages/ProjectDetailEditorial.tsx:288`, `src/pages/ProjectDetailEditorial.tsx:290`, `src/pages/ProjectDetailEditorial.tsx:309`).
- **Recommendation:** keep separate. It is still an entity dossier header with project status, owner, archive workflow, and page-level editable vitals. Do not collapse into `SurfaceMasthead`; separately consider restoring/clarifying the project tint if the dossier identity needs to be stronger.

### MeetingDetail inline hero (residual MeetingHero)

- **File:** `src/pages/MeetingDetailPage.tsx` + `src/pages/meeting-intel.module.css`
- **Used in surfaces:** `/meeting/$meetingId` via `MeetingDetailPage` (`src/router.tsx:598`, `src/pages/MeetingDetailPage.tsx:1202`).
- **Structure:**
  - eyebrow: yes, static "Meeting Briefing" kicker (`src/pages/MeetingDetailPage.tsx:1211`, `src/pages/MeetingDetailPage.tsx:1212`), plus optional urgency banner above it (`src/pages/MeetingDetailPage.tsx:1203`, `src/pages/MeetingDetailPage.tsx:1204`).
  - title: meeting title in `h1` (`src/pages/MeetingDetailPage.tsx:1216`, `src/pages/MeetingDetailPage.tsx:1217`).
  - lede: no generic lede in the hero block. Meeting synthesis appears elsewhere in the surface; the hero focuses on metadata and readiness.
  - accessory: no `SurfaceMasthead` accessory. Lifecycle badge, metadata line, quality badge, entity chips, health strip, and signal banners are inline children (`src/pages/MeetingDetailPage.tsx:1220`, `src/pages/MeetingDetailPage.tsx:1227`, `src/pages/MeetingDetailPage.tsx:1236`, `src/pages/MeetingDetailPage.tsx:1241`, `src/pages/MeetingDetailPage.tsx:1258`, `src/pages/MeetingDetailPage.tsx:1277`, `src/pages/MeetingDetailPage.tsx:1288`).
  - watermark: no.
  - entity-tint: meeting uses turmeric urgency/lifecycle accents and larkspur for new signals, not a single entity identity tint (`src/pages/meeting-intel.module.css:454`, `src/pages/meeting-intel.module.css:458`, `src/pages/meeting-intel.module.css:502`).
  - other: `MeetingEntityChips`, account health strip, and consistency status banners are all hosted in the hero section (`src/pages/MeetingDetailPage.tsx:1244`, `src/pages/MeetingDetailPage.tsx:1259`, `src/pages/MeetingDetailPage.tsx:1289`).
- **Variants observed:** starts-soon/urgent banner (`src/pages/MeetingDetailPage.tsx:1204`, `src/pages/meeting-intel.module.css:439`, `src/pages/meeting-intel.module.css:450`); past meeting opacity wrapper (`src/pages/MeetingDetailPage.tsx:1197`, `src/pages/meeting-intel.module.css:435`); lifecycle badge optional (`src/pages/MeetingDetailPage.tsx:1220`); account health strip only for linked accounts with health (`src/pages/MeetingDetailPage.tsx:1259`); new signals banner (`src/pages/MeetingDetailPage.tsx:1278`); consistency status banner for non-ok state (`src/pages/MeetingDetailPage.tsx:1289`).
- **Recommendation:** collapse into `SurfaceMasthead`. The design system already declares MeetingHero to be a `SurfaceMasthead` composition (`.docs/design/patterns/SurfaceMasthead.md:16`, `.docs/design/surfaces/MeetingDetail.md:26`), but production code has not completed that migration. Treat this as implementation drift.

### EditorialPageHeader

- **File:** `src/components/editorial/EditorialPageHeader.tsx` + `src/components/editorial/EditorialPageHeader.module.css`
- **Used in surfaces:** entity list shell (`src/components/entity/EntityListShell.tsx:60`), Actions (`src/pages/ActionsPage.tsx:249`), Projects empty state (`src/pages/ProjectsPage.tsx:266`), People empty state (`src/pages/PeoplePage.tsx:293`), Inbox (`src/pages/InboxPage.tsx:635`, `src/pages/InboxPage.tsx:746`), History (`src/pages/HistoryPage.tsx:59`), Me profile (`src/pages/MePage.tsx:289`).
- **Structure:**
  - eyebrow: no explicit prop.
  - title: required `title` prop rendered in `h1` (`src/components/editorial/EditorialPageHeader.tsx:8`, `src/components/editorial/EditorialPageHeader.tsx:50`).
  - lede: `subtitle` prop rendered as paragraph (`src/components/editorial/EditorialPageHeader.tsx:10`, `src/components/editorial/EditorialPageHeader.tsx:51`).
  - accessory: `meta` prop rendered in a right-side column (`src/components/editorial/EditorialPageHeader.tsx:11`, `src/components/editorial/EditorialPageHeader.tsx:53`).
  - watermark: no.
  - entity-tint: none by default; `ruleColor` allows caller-provided accent, used by Me (`src/components/editorial/EditorialPageHeader.tsx:16`, `src/pages/MePage.tsx:298`).
  - other: `children` render in `.after` slot below the rule (`src/components/editorial/EditorialPageHeader.tsx:12`, `src/components/editorial/EditorialPageHeader.tsx:56`).
- **Variants observed:** `scale="standard" | "page" | "profile"` (`src/components/editorial/EditorialPageHeader.tsx:4`, `src/components/editorial/EditorialPageHeader.module.css:20`, `src/components/editorial/EditorialPageHeader.module.css:25`, `src/components/editorial/EditorialPageHeader.module.css:30`); `width="standard" | "reading"` (`src/components/editorial/EditorialPageHeader.tsx:5`, `src/components/editorial/EditorialPageHeader.module.css:12`, `src/components/editorial/EditorialPageHeader.module.css:16`); `rule="standard" | "subtle"` (`src/components/editorial/EditorialPageHeader.tsx:6`, `src/components/editorial/EditorialPageHeader.module.css:35`, `src/components/editorial/EditorialPageHeader.module.css:39`); title-only, title+meta, title+subtitle, and children-as-controls usages (`src/pages/ActionsPage.tsx:249`, `src/pages/ActionsPage.tsx:255`, `src/pages/InboxPage.tsx:746`, `src/pages/MePage.tsx:291`).
- **Recommendation:** collapse into `SurfaceMasthead` after `SurfaceMasthead` gets parity for `scale`/rule variants. This is the highest-confidence consolidation: both components are surface headers with title, lede/subtitle, top-right metadata/accessory, width, rule color, and after/glance content.

### DailyBriefing's editorial-briefing hero

- **File:** `src/components/dashboard/DailyBriefing.tsx` + `src/styles/editorial-briefing.module.css`
- **Used in surfaces:** `/` DashboardPage success state (`src/router.tsx:496`, `src/router.tsx:486`). The same CSS hero classes are also reused by Emails (`src/pages/EmailsPage.tsx:415`) and DashboardSkeleton (`src/components/dashboard/DashboardSkeleton.tsx:44`).
- **Structure:**
  - eyebrow: no.
  - title: one-sentence `heroHeadline` from `data.overview.summary`, falling back to "A clear day. Nothing needs you." or "Your day is ready." (`src/components/dashboard/DailyBriefing.tsx:355`, `src/components/dashboard/DailyBriefing.tsx:356`, `src/components/dashboard/DailyBriefing.tsx:363`, `src/components/dashboard/DailyBriefing.tsx:364`).
  - lede: not as a conventional masthead lede. `data.overview.focus` renders as a focus directive block; capacity renders as mono metadata (`src/components/dashboard/DailyBriefing.tsx:366`, `src/components/dashboard/DailyBriefing.tsx:370`, `src/components/dashboard/DailyBriefing.tsx:380`).
  - accessory: no.
  - watermark: no.
  - entity-tint: no entity tint; focus block uses turmeric left rule (`src/styles/editorial-briefing.module.css:106`).
  - other: the hero is the start of the "Day Frame" chapter, not a generic page header (`src/components/dashboard/DailyBriefing.tsx:362`).
- **Variants observed:** no-meeting vs active-day fallback headline (`src/components/dashboard/DailyBriefing.tsx:356`); optional capacity line when `data.focus` exists (`src/components/dashboard/DailyBriefing.tsx:367`); optional focus block when `data.overview.focus` exists (`src/components/dashboard/DailyBriefing.tsx:380`); staleness intentionally removed (`src/components/dashboard/DailyBriefing.tsx:386`); CSS still contains unused `.heroNarrative` and `.staleness` classes (`src/styles/editorial-briefing.module.css:80`, `src/styles/editorial-briefing.module.css:91`).
- **Recommendation:** keep separate as the `Lead` pattern, not a masthead. The design spec explicitly defines Lead as DailyBriefing's one-sentence opener (`.docs/design/patterns/Lead.md:14`, `.docs/design/patterns/Lead.md:18`) and says `SurfaceMasthead` should not be used for DailyBriefing's opening (`.docs/design/patterns/SurfaceMasthead.md:26`).

### SurfaceMasthead (the post-Wave-3 canonical)

- **File:** `src/components/layout/SurfaceMasthead.tsx` + `src/components/layout/SurfaceMasthead.module.css`
- **Used in surfaces:** Settings only in production (`src/pages/SettingsPage.tsx:264`). Design docs name Settings and MeetingDetail as intended consumers (`.docs/design/patterns/SurfaceMasthead.md:86`, `.docs/design/surfaces/MeetingDetail.md:26`).
- **Structure:**
  - eyebrow: optional `eyebrow` prop rendered above title (`src/components/layout/SurfaceMasthead.tsx:10`, `src/components/layout/SurfaceMasthead.tsx:54`).
  - title: required `title` prop rendered in `h1` (`src/components/layout/SurfaceMasthead.tsx:11`, `src/components/layout/SurfaceMasthead.tsx:55`).
  - lede: optional `lede` prop (`src/components/layout/SurfaceMasthead.tsx:12`, `src/components/layout/SurfaceMasthead.tsx:56`).
  - accessory: optional `accessory` prop rendered top-right (`src/components/layout/SurfaceMasthead.tsx:13`, `src/components/layout/SurfaceMasthead.tsx:58`).
  - watermark: no.
  - entity-tint: no entity tint; `ruleColor` sets rule accent (`src/components/layout/SurfaceMasthead.tsx:18`, `src/components/layout/SurfaceMasthead.tsx:35`).
  - other: optional `glance` slot below the rule (`src/components/layout/SurfaceMasthead.tsx:14`, `src/components/layout/SurfaceMasthead.tsx:61`); component emits `data-ds-name` and `data-ds-spec` (`src/components/layout/SurfaceMasthead.tsx:48`, `src/components/layout/SurfaceMasthead.tsx:49`).
- **Variants observed:** API supports `density="compact" | "default" | "rich"` and `width="standard" | "reading"` (`src/components/layout/SurfaceMasthead.tsx:5`, `src/components/layout/SurfaceMasthead.tsx:6`); CSS maps compact/default/rich to different vertical rhythm and title sizes (`src/components/layout/SurfaceMasthead.module.css:19`, `src/components/layout/SurfaceMasthead.module.css:24`, `src/components/layout/SurfaceMasthead.module.css:29`, `src/components/layout/SurfaceMasthead.module.css:65`, `src/components/layout/SurfaceMasthead.module.css:69`, `src/components/layout/SurfaceMasthead.module.css:73`); production currently uses only title/default/standard (`src/pages/SettingsPage.tsx:264`).
- **Recommendation:** refactor. Keep it as canonical, but close parity gaps before migration: its code says default title 42px and rich title 76px (`src/components/layout/SurfaceMasthead.module.css:69`, `src/components/layout/SurfaceMasthead.module.css:73`), while its spec says default 36px and rich 52px (`.docs/design/patterns/SurfaceMasthead.md:44`, `.docs/design/patterns/SurfaceMasthead.md:46`). It also needs an `EditorialPageHeader`-equivalent subtle rule variant before safe collapse.

## Field-by-field comparison

| Field | AccountHero | PersonHero | ProjectHero | MeetingInline | EditorialPageHeader | briefingLead | SurfaceMasthead |
|---|---|---|---|---|---|---|---|
| eyebrow | yes: quality/date/type | yes: quality/date/Clay | yes: quality/date | yes: static kicker + urgency | no | no | yes |
| title (h1) | yes | yes | yes | yes | yes | yes | yes |
| lede | no current runtime lede | yes | yes | no generic lede | subtitle | focus block, not masthead lede | yes |
| accessory slot | no | no | no | no | meta | no | yes |
| watermark | no runtime | no runtime | no runtime | no | no | no | no |
| entity-tint | account/turmeric | person/larkspur | weak project/olive | turmeric state accent | caller ruleColor only | no | ruleColor only |
| vitals | slot inside hero | outside hero | outside hero | health strip inside hero | children slot can host | no | glance slot |
| metadata strip | parent/type/date/provenance | subtitle/social/meta actions | status/owner/meta actions | time/type/entity/quality | meta + after | capacity line | eyebrow/accessory/glance |
| archive state | yes | yes | yes | past-meeting opacity | no | no | no |
| editable fields | name/type via `onSaveField` | name/role | name/status display | no title edit | no | no | caller-provided children only |
| shared base | EntityHeroBase | EntityHeroBase | EntityHeroBase | none | own CSS | editorial-briefing CSS | own CSS |

## Recommendations

### Keep separate

- `AccountHero`, `PersonHero`, `ProjectHero` - keep as entity dossier heroes. They share `EntityHeroBase`, but each hosts entity-specific identity, editing, metadata, and workflow that would make `SurfaceMasthead` overly polymorphic.
- `DailyBriefing` hero - keep separate as the `Lead` pattern. It is a one-sentence editorial opener with day-frame focus, not a page masthead.

### Collapse

- `EditorialPageHeader` - collapse into `SurfaceMasthead` after parity work. Maintain a temporary compatibility wrapper if needed, but the durable component should be `SurfaceMasthead`.
- MeetingDetail inline hero - collapse into `SurfaceMasthead`. The spec already says MeetingHero is not a separate pattern; the production page still needs to catch up.

### Refactor

- `SurfaceMasthead` - refactor before broad migration. Required parity: standard/page/profile sizing, subtle vs standard rule, and compatibility for `meta`/`children` call sites. Also resolve code/spec size drift for default and rich densities.

## Migration plan

- Add `SurfaceMasthead` parity for `EditorialPageHeader`: either `scale="standard" | "page" | "profile"` aliases or equivalent density tokens, plus `ruleVariant="standard" | "subtle"` or an equivalent rule-height/color API.
- Convert `EditorialPageHeader` into a thin wrapper over `SurfaceMasthead` first. Map `subtitle -> lede`, `meta -> accessory`, and `children -> glance`/after content without changing call sites.
- Migrate low-risk title-only call sites first: Inbox empty state, Projects empty state, People empty state, History.
- Migrate content-heavy `EditorialPageHeader` consumers next: `EntityListShell` and Actions, because they use the `children` slot for search/tabs.
- Migrate Me profile after visual parity for `scale="profile"` and `ruleColor`.
- Replace MeetingDetail's inline `styles.heroSection`/`styles.heroTitle` block with `SurfaceMasthead density="rich"` once the MeetingDetail composition is ready. Map kicker/date metadata into `eyebrow`, title into `title`, lifecycle/status/quality into `accessory`, and entity chips/health strip into `glance`. Leave new-signals and consistency banners below the masthead.
- Leave `AccountHero`, `PersonHero`, `ProjectHero`, and DailyBriefing's lead out of this migration. Optional cleanup only: remove stale watermark comments and unused hero CSS/classes after confirming no design reference depends on them.

## Out of scope

- Implementation. This audit only documents divergence and recommends migration order.
- Visual parity testing. Any `EditorialPageHeader` or MeetingDetail migration should get before/after screenshots.
- Broader "hero" names that are not top-of-surface mastheads in this audit, such as `HealthBadge`'s hero size, `SentimentHero`, and onboarding welcome copy.
- Collapsing DailyBriefing's hero into `SurfaceMasthead`; it is intentionally the `Lead` pattern, not a generic hero.
