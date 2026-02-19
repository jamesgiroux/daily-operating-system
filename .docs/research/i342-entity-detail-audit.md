# I342 Entity Detail Surfaces -- JTBD Definition + Element Inventory

**Auditor:** entity-detail-auditor
**Date:** 2026-02-18
**Scope:** Account Detail, Project Detail, Person Detail (editorial pages), plus entity list pages (Accounts, Projects, People)

---

## Part 1: Are These Genuinely Different Surfaces?

**Verdict: Yes, but they share a structural skeleton.**

The three entity detail pages (Account, Project, Person) share a common magazine-layout editorial structure and reuse six shared components from `src/components/entity/`. However, each has unique chapters that reflect genuinely different jobs. The pattern:

| Structural Layer | Account | Project | Person |
|---|---|---|---|
| Hero chapter | AccountHero | ProjectHero | PersonHero |
| Vitals strip | Shared (VitalsStrip) | Shared (VitalsStrip) | Shared (VitalsStrip) |
| Resolution keywords | Yes | Yes | No |
| Assessment chapter | StateOfPlay (shared) | TrajectoryChapter (unique) | PersonInsightChapter (unique, adaptive) |
| Forward-looking chapter | N/A | HorizonChapter (unique) | N/A |
| People chapter | StakeholderGallery (shared) | StakeholderGallery (shared) | PersonNetwork (unique, inverted) |
| Watch list chapter | WatchList (shared) + WatchListPrograms | WatchList (shared) + WatchListMilestones | WatchList (shared) |
| Timeline chapter | UnifiedTimeline (shared) | UnifiedTimeline (shared) | UnifiedTimeline (shared) |
| Work chapter | TheWork (shared) | N/A (actions in appendix) | N/A |
| Appendix | AccountAppendix (unique) | ProjectAppendix (unique) | PersonAppendix (unique) |
| Finis marker | Shared (FinisMarker) | Shared (FinisMarker) | Shared (FinisMarker) |

The chapter nav islands are different per entity type (CHAPTERS arrays), and the accent colors differ (turmeric for accounts, olive for projects, larkspur for people).

**Key observation:** The surfaces are genuinely different in their *middle chapters* (the "analysis" layer) but identical in their *bookend* chapters (hero at top, timeline + appendix at bottom). This suggests the shell is right to be shared, but each entity type needs its own editorial narrative between the bookends.

---

## Part 2: Surface-Level JTBD

### Entity Detail Pages (collectively)

**Situation:** The user is about to meet someone from Acme, needs to prep for a project review, or wants to understand a relationship trajectory after a notable interaction. They arrive via a link from the daily briefing, meeting detail, or by searching the entity list.

**Motivation:** "Give me the full picture of this relationship/initiative so I can show up informed, make good decisions, and not miss anything important."

**Desired outcome:** The user closes the page feeling they understand the current state, the key people, the risks, and what needs their attention -- enough to act confidently in their next interaction.

**Boundary:** This is the *dossier*, not the *action queue*. It builds understanding, not to-do lists (though it surfaces commitments as context). It is not the place to manage workflow or do real-time collaboration.

---

### Account Detail

**Situation:** Before a customer meeting, during quarterly reviews, when a health indicator changes, or when a colleague asks "what's going on with Acme?"

**Motivation:** "I need to understand this relationship's health, trajectory, risks, and the people involved so I can protect and grow revenue."

**Desired outcome:** The user can articulate the state of the account in one paragraph, knows the risks, knows who matters, and knows what commitments are outstanding.

**Boundary:** Not a CRM data entry screen. Not a financial reporting tool. Not where you manage day-to-day tasks (though it shows them for context).

### Project Detail

**Situation:** Before a project status meeting, when assessing whether a project is on track, or when deciding resource allocation.

**Motivation:** "I need to understand this project's momentum, timeline risk, milestone progress, and team dynamics to make informed decisions about where to focus."

**Desired outcome:** The user knows whether the project is on track, what's blocking it, when the next milestone is, and what decisions are pending.

**Boundary:** Not a project management tool (no Gantt charts, no sprint boards). Not where you do the work -- it's where you understand the work.

### Person Detail

**Situation:** Before a meeting with this person, when onboarding to a new relationship, when trying to understand someone's role in an account, or when doing relationship hygiene (merges, data cleanup).

**Motivation:** "I need to understand this person's role in my professional world -- who they are, how we interact, what organizations they connect to, and how the relationship is trending."

**Desired outcome:** The user can recall context about this person instantly, knows the relationship temperature, and understands their organizational connections.

**Boundary:** Not a CRM contact card (though it shows contact fields). Not a social profile. The job is *relationship intelligence*, not *contact management*.

---

## Part 3: Section-Level JTBDs

### Account Detail Sections

#### Chapter 1: The Headline (AccountHero + VitalsStrip + Keywords)
**File:** `src/components/account/AccountHero.tsx` (lines 30-162), `src/components/entity/VitalsStrip.tsx`, inline keyword section in `AccountDetailEditorial.tsx` (lines 265-329)
**Job:** "In 5 seconds, tell me what this account is and how it's doing."
- Hero: Account name (76px serif), executive assessment lede (intelligence-generated), health/lifecycle/internal badges, company context description
- Vitals: ARR, health status, lifecycle stage, renewal countdown, NPS, meeting frequency, contract start
- Keywords: Resolution keywords (auto-extracted, removable) -- help the system match meetings to this account
- Meta actions: Edit Fields, Manage Team, Build Intelligence, Archive/Unarchive, Reports link, + Business Unit

**Observations:**
- "Resolution Keywords" is system vocabulary (violates ADR-0083). Users don't know what "resolution" means in this context.
- "Build Intelligence" is system vocabulary. Should be something like "Check for updates" or "Refresh".
- The hero "Account Intelligence" date label is system vocabulary.
- Company context appears both in the hero (if intelligence has it) and in the appendix. Duplication.
- The meta action row has 5-7 buttons in a wrap layout. Dense for a hero section.

#### Chapter 2: State of Play (StateOfPlay)
**File:** `src/components/entity/StateOfPlay.tsx`
**Job:** "What's working and what's struggling in this relationship right now?"
- Two StateBlocks: "What's Working" (sage) and "Where It's Struggling" (terracotta)
- Pull quote from second paragraph of executive assessment
- Truncated to 5 items per section, expandable
- Editable inline via EditableText

**Observations:**
- Account-only chapter (projects use TrajectoryChapter, people use PersonInsightChapter). Could be reframed if the same intelligence data structure exists on all entities.
- The pull quote placement (after the state blocks) feels like an afterthought rather than a deliberate editorial choice.

#### Chapter 3: The Room (StakeholderGallery)
**File:** `src/components/entity/StakeholderGallery.tsx`
**Job:** "Who are the key people in this account, what are their engagement levels, and what do I need to know about each?"
- 2-column grid of stakeholder cards: avatar, name (editable), engagement badge (EngagementSelector), role (editable), assessment (editable, truncated)
- Linked people fallback when no intelligence stakeholders
- "Your Team" strip at bottom (account team members with roles)
- Add/remove stakeholders, create person entity from stakeholder
- Filtered: internal people excluded from stakeholder list (shown in "Your Team" instead)

**Observations:**
- "Your Team" strip only appears for accounts (via accountTeam prop). Projects use the same component but title it "The Team" and pass no accountTeam.
- The engagement levels (Champion, Active Partner, Participates, Passive, Not Yet Seen, At Risk) are well-designed product vocabulary.
- "Create contact" hover action is useful but discoverable only on hover -- easy to miss.

#### Chapter 4: Watch List (WatchList + WatchListPrograms)
**File:** `src/components/entity/WatchList.tsx`, `src/components/account/WatchListPrograms.tsx`
**Job:** "What should I be worried about, what's going well, and what don't we know yet?"
- Full-bleed linen background band
- Three sections: Risks (terracotta dots), Wins (sage dots), Unknowns (larkspur dots)
- First risk and first win get callout treatment (left border, background)
- Editable inline
- Account-specific bottom section: Active Initiatives (WatchListPrograms) with editable name/status, add/delete

**Observations:**
- This is one of the most useful chapters on any entity detail page. The risks/wins/unknowns framework is clear and actionable.
- "Active Initiatives" (account-specific) feel like they belong more in "The Work" or a dedicated programs chapter than nested inside the watch list.

#### Chapter 5: The Record (UnifiedTimeline)
**File:** `src/components/entity/UnifiedTimeline.tsx`
**Job:** "What's the history of interactions with this account?"
- Chronological timeline: meetings (linked to meeting detail), emails, captures
- 10 items visible by default, expandable
- Vertical line timeline visual treatment

**Observations:**
- The timeline is read-only and backward-looking. Its job is clear: provide context, not drive action.
- The same component is used identically across all three entity types. Good reuse.

#### Chapter 6: The Work (TheWork)
**File:** `src/components/entity/TheWork.tsx`
**Job:** "What commitments do I have related to this account, and what's coming up?"
- "Next Meeting" readiness callout (from intelligence)
- Upcoming meetings list (date, title, type badge)
- Commitments: open actions grouped by urgency (Overdue, This Week, Upcoming)
- Inline action creation

**Observations:**
- **Account-only.** Projects put actions in the appendix instead. People have no work chapter at all. This inconsistency is the most significant structural difference between entity types.
- The "Next Meeting" readiness callout duplicates information available on the meeting detail page. Cross-surface duplication.
- "Commitments" label is good product vocabulary for actions in this context.

#### Appendix (AccountAppendix)
**File:** `src/components/account/AccountAppendix.tsx`
**Job:** "Reference material I don't need every visit but want available when I dig deep."
- Double-rule separator with "Reference" label
- Sections: Lifecycle events (with + Record), Notes (editable textarea), Files (FileListSection with re-index), Company Context (from intelligence or detail), Value Delivered (from intelligence), Portfolio Summary (parent accounts), Business Units (child accounts with health dots, ARR, action counts)

**Observations:**
- The appendix is account-heavy. Lifecycle events, portfolio summary, BUs, value delivered -- these are all account-specific concepts with no counterpart on projects or people.
- Notes are editable inline. This is a good pattern shared across all three entity types.
- "Company Context" appears both here and in the hero section. Redundant.
- "Value Delivered" is a useful intelligence section but might be more impactful in the main body (e.g., in Watch List as "Wins delivered").

#### Drawers (Account-specific)
- **AccountFieldsDrawer:** Edit name, health, lifecycle, ARR, NPS, renewal date. Job: structured data entry without navigating away.
- **TeamManagementDrawer:** Add/remove account team members, search existing people, create inline. Job: manage internal team assignments.
- **LifecycleEventDrawer:** Record a lifecycle event (type, date, ARR impact, notes). Job: log milestone events in the account's history.

#### Dialogs (Account-specific)
- **Archive Confirmation:** Standard destructive action confirmation.
- **Child Account Creation:** Name + description for new business unit.

---

### Project Detail Sections

#### Chapter 1: The Mission (ProjectHero + VitalsStrip + Keywords)
**File:** `src/components/project/ProjectHero.tsx`, `ProjectDetailEditorial.tsx` lines 183-262
**Job:** "In 5 seconds, tell me what this project is and its status."
- Hero: name (76px serif), executive assessment lede, status badge (Active/On Hold/Completed), owner badge
- Vitals: status (highlighted), days to target (with trend arrow), milestone progress ("X of Y milestones"), meeting frequency, open action count
- Keywords: Same resolution keywords pattern as accounts
- Meta actions: Edit Fields, Build Intelligence, Archive/Unarchive

**Observations:**
- "The Mission" chapter label is better than "The Headline" (account) -- it communicates purpose.
- No "Manage Team" action (projects use StakeholderGallery for team, not a separate drawer).
- No "Reports" action link (accounts have a risk briefing page; projects don't).

#### Chapter 2: Trajectory (TrajectoryChapter)
**File:** `src/components/project/TrajectoryChapter.tsx`
**Job:** "Is this project gaining or losing momentum?"
- Pull quote from executive assessment
- StateBlocks reframed: "Momentum" (working) and "Headwinds" (not working) -- project-appropriate vocabulary
- VelocityStrip: meeting frequency with trend arrows, open action count, days to target
- Remaining assessment prose

**Observations:**
- This is the project's equivalent of StateOfPlay but with better framing. "Momentum" and "Headwinds" communicate more than "What's Working" and "Where It's Struggling."
- The VelocityStrip is a good addition that accounts lack -- accounts have ARR but not velocity signals in the body.

#### Chapter 3: The Horizon (HorizonChapter)
**File:** `src/components/project/HorizonChapter.tsx`
**Job:** "What's coming next and are we going to make it?"
- Next milestone callout (name, target date, days away/overdue)
- Target date reality (large numeric display: "X days to target" or "X days overdue")
- Timeline risk (auto-extracted from risks matching timeline/deadline/schedule keywords)
- Decisions Pending (unknowns from intelligence, reframed with larkspur color)
- Meeting Readiness callout (for next meeting related to this project)

**Observations:**
- **Project-only. No equivalent on accounts or people.** This is the strongest argument that projects are a genuinely different surface.
- The "Decisions Pending" section is unknowns reframed -- good product vocabulary choice.
- Meeting readiness appears here AND in TheWork (for accounts). Different placement for the same concept.

#### Chapter 4: The Landscape (WatchList + WatchListMilestones)
**File:** Shared `WatchList.tsx` + `WatchListMilestones.tsx`
**Job:** "What risks, wins, and milestones should I track?"
- Same risks/wins/unknowns as accounts
- Bottom section: active milestones (name, status badge, target date) instead of programs

**Observations:**
- "The Landscape" is a better title than "Watch List" for this chapter -- the chapter heading is customizable via props.
- Milestones shown both here (active only) and in the appendix (full list). Intentional: landscape shows what to watch, appendix is reference.

#### Chapter 5: The Team (StakeholderGallery)
**Same shared component as accounts, titled "The Team" instead of "The Room".**

#### Chapter 6: The Record (UnifiedTimeline)
**Same shared component as accounts.**

#### Appendix (ProjectAppendix)
**File:** `src/components/project/ProjectAppendix.tsx`
**Job:** "Reference: actions, milestones, description, notes, files."
- Open Actions (with overdue highlighting, inline creation)
- Milestones (full list with status and target dates)
- Description (from project.description)
- Notes (editable textarea)
- Files (FileListSection)

**Observations:**
- **Actions are in the appendix, not a main chapter.** This is a significant difference from accounts, where TheWork is a first-class chapter. Implies actions are less central to the project's story than the account's story.
- No lifecycle events (account-specific concept).
- No company context, value delivered, portfolio summary, or BUs (all account-specific).

#### Drawers (Project-specific)
- **ProjectFieldsDrawer:** Edit name, status, milestone, owner, target date. Simpler than AccountFieldsDrawer.

---

### Person Detail Sections

#### Chapter 1: The Profile (PersonHero + VitalsStrip)
**File:** `src/components/person/PersonHero.tsx`, `PersonDetailEditorial.tsx` lines 155-174
**Job:** "In 5 seconds, tell me who this person is and how our relationship is trending."
- Hero: avatar + name (48px avatar, 76px serif name), executive assessment lede, email + org/role subtitle, social links (LinkedIn, Twitter/X), relationship badge (external/internal/unknown), temperature badge (hot/warm/cool/cold)
- Vitals: temperature, meeting frequency with trend, last meeting date, total meeting count
- Meta actions: Edit Details, Build Intelligence, Enrich from Clay, Merge Into..., Archive, Delete

**Observations:**
- **No keywords.** Unlike accounts and projects, people don't have resolution keywords. This makes sense -- people are matched by email, not keywords.
- **More meta actions** than any other entity type (6 buttons). "Merge Into..." and "Delete" are unique to people. "Enrich from Clay" is an external data source integration.
- Person Hero uses a `subtitle` field (email, org, role) that accounts and projects don't have.
- "Person Intelligence" date label -- same system vocabulary issue as other heroes.

#### Chapter 2: The Dynamic / The Rhythm (PersonInsightChapter)
**File:** `src/components/person/PersonInsightChapter.tsx`
**Job:** "What defines this relationship and how is it going?"
- **Adaptive framing** based on relationship type:
  - External: "The Dynamic" with "Relationship Strengths" / "Relationship Gaps"
  - Internal: "The Rhythm" with "Collaboration Strengths" / "Alignment Gaps"
- StateBlocks (working/not working) with relationship-appropriate labels
- Pull quote from executive assessment
- CadenceStrip: meeting frequency with trend, temperature, last met date
- Remaining assessment prose

**Observations:**
- **The adaptive framing is the most sophisticated JTBD-aware design in the entity system.** Internal colleagues have different relationship dynamics than external contacts; the UI reflects this.
- CadenceStrip (person) vs VelocityStrip (project) -- similar pattern, different data. Both could be a shared component.

#### Chapter 3: The Network (PersonNetwork)
**File:** `src/components/person/PersonNetwork.tsx`
**Job:** "Which accounts and projects is this person connected to?"
- **Inverted StakeholderGallery:** shows entities for a person (where StakeholderGallery shows people for an entity).
- Two-column grid: Accounts (turmeric dots) and Projects (olive dots) with entity type labels
- EntityPicker to link new accounts/projects
- Unlink button per entity

**Observations:**
- This is the *inverse* of the StakeholderGallery. On an account/project, you see "who's in the room." On a person, you see "which rooms are they in."
- The relationship is bidirectional -- linking here updates the entity's linked people. Good data model consistency.
- **Person-only chapter.** No equivalent on accounts or projects (they have StakeholderGallery instead).

#### Chapter 4: The Landscape (WatchList)
**Same shared component, no bottom section slot.** Simpler than account (no programs) or project (no milestones).

#### Chapter 5: The Record (UnifiedTimeline)
**Same shared component.** Person passes `recentMeetings`, `recentCaptures`, and `recentEmailSignals` from detail.

#### Appendix (PersonAppendix)
**File:** `src/components/person/PersonAppendix.tsx`
**Job:** "Reference: editable profile fields, notes, files, duplicate detection."
- Details grid: Name (editable), Email (read-only), Role (editable), Organization (read-only), Relationship (read-only), First Seen, Last Seen
- Notes (editable textarea)
- Files (FileListSection)
- Potential Duplicates: candidates with confidence scores, merge buttons

**Observations:**
- **No actions at all.** People don't have commitments on this surface. Actions are only visible on accounts (main chapter) and projects (appendix).
- **Duplicate detection is person-only.** Makes sense -- people are the only entity type auto-created from meeting attendees, so duplicates are common.
- The details grid has editable fields (name, role) mixed with read-only fields (email, org, relationship). The edit affordance is subtle (underline on focus).

#### Dialogs (Person-specific)
- **Merge Person Picker:** Search-and-select dialog for merge target. Person-specific data hygiene.
- **Merge Confirmation:** Destructive action confirmation with transfer details.
- **Delete Confirmation:** Destructive action with data loss warning.

---

## Part 4: Entity List Pages

### Accounts List (AccountsPage)
**File:** `src/pages/AccountsPage.tsx`
**Job:** "See all my accounts at a glance and navigate to the one I need."
- Headline: "Your Book"
- EntityListHeader with search, count, lifecycle filter tabs
- Folio actions: Archive toggle, + New button
- AccountRow: health dot, name, Internal badge, BU expand toggle (parent/child), team summary, action count, ARR, days since last meeting (terracotta if >14d)
- Create form (inline + bulk mode)
- Archive view with basic rows

**Observations:**
- "Your Book" is strong product vocabulary -- it frames accounts as a portfolio.
- The parent/child expand pattern is unique to accounts (BUs). Adds complexity to the list.
- ARR and health dot are account-specific -- dense but appropriate for the job.

### Projects List (ProjectsPage)
**File:** `src/pages/ProjectsPage.tsx`
**Job:** "See all my projects, filter by status, navigate to the one I need."
- Headline: "Projects"
- EntityListHeader with search, count, status filter tabs (All/Active/On Hold/Completed), archive toggle
- ProjectRow: status dot, name, status label (colored), owner + milestone subtitle, target date, open action count, days since last meeting
- Create form (inline + bulk)

**Observations:**
- Status filter tabs are project-specific and well-suited.
- Uses personality system for empty/no-matches copy (accounts don't).

### People List (PeoplePage)
**File:** `src/pages/PeoplePage.tsx`
**Job:** "See all contacts, filter by relationship type, handle data hygiene, navigate to a person."
- Headline: "The Room"
- EntityListHeader with search, count, relationship filter tabs (All/External/Internal/Unknown), archive toggle
- PersonRow: temperature dot, name, trend arrow, relationship badge, org/role subtitle, last seen date
- Sorted by temperature then last seen (hottest first)
- Hygiene features: unnamed person filter banner, duplicate detection banner with inline merge
- Create form (email + name)

**Observations:**
- "The Room" is strong product vocabulary -- matches the StakeholderGallery chapter title.
- Hygiene features (unnamed filter, duplicate detection) are unique to people and represent a significant amount of UI real estate. These serve data quality, not relationship intelligence.
- The duplicate banner with inline merge is powerful but complex -- it's a list-level feature that handles person-level operations.

---

## Part 5: Shared Entity Components (src/components/entity/)

| Component | File | Used By | Job |
|---|---|---|---|
| VitalsStrip | VitalsStrip.tsx | Account, Project, Person | Horizontal metric strip with dot separators. Callers build their own vitals array. |
| StateOfPlay | StateOfPlay.tsx | Account only | Working/Struggling state blocks + pull quote. |
| StakeholderGallery | StakeholderGallery.tsx | Account, Project | 2-column stakeholder cards with engagement badges, editable fields, "Your Team" strip. |
| WatchList | WatchList.tsx | Account, Project, Person | Risks/Wins/Unknowns in full-bleed linen band. Bottom section slot for entity-specific content. |
| UnifiedTimeline | UnifiedTimeline.tsx | Account, Project, Person | Chronological meeting/email/capture timeline. |
| TheWork | TheWork.tsx | Account only | Meeting readiness, upcoming meetings, commitments (actions). |
| EntityListShell | EntityListShell.tsx | Accounts, Projects, People lists | Skeleton, Error, Empty, Header, ArchiveToggle, FilterTabs, EndMark -- shared list page scaffolding. |
| EntityRow | EntityRow.tsx | Accounts, Projects, People lists | Generic list row: dot, name, nameSuffix, subtitle, right-aligned children. |
| FileListSection | FileListSection.tsx | Account, Project, Person appendices | File grid with reveal-in-Finder, expand, re-index. |
| EngagementSelector | EngagementSelector.tsx | StakeholderGallery | Dropdown badge for stakeholder engagement levels. |

---

## Part 6: Cross-Surface Duplication Analysis

| Element | Surfaces Where It Appears | Notes |
|---|---|---|
| Executive assessment lede | Entity detail hero + Meeting detail (if entity linked) | The same AI-generated text appears in the hero of every entity detail page and may appear in meeting briefings referencing the entity. |
| Meeting readiness / prep items | Account (TheWork), Project (HorizonChapter), Meeting Detail page | Next-meeting readiness appears on entity detail pages and the meeting detail page itself. Which surface owns this? |
| Upcoming meetings | Account (TheWork), Daily Briefing, Weekly Forecast | Upcoming meetings for an account appear on the account detail and the temporal surfaces. |
| Open actions | Account (TheWork), Project (Appendix), Actions page, Daily Briefing | Actions appear on entity pages, the dedicated actions page, and temporal briefings. |
| Stakeholder insights | Entity detail (StakeholderGallery), Meeting detail (attendee intelligence) | Stakeholder data appears on the entity page and on meeting pages when those stakeholders attend. |
| Risks | Entity detail (WatchList), Meeting detail (risks section), Risk Briefing page | Risk data from intelligence appears on the entity, individual meetings, and the risk briefing page. |
| Company context | Account hero (companyContext.description) + Account appendix (Company Context section) | Same data appears in two places on the same page. |
| Notes | Entity detail appendix (all three types) | No duplication -- notes are entity-specific. Good. |
| Files | Entity detail appendix (all three types) | No duplication -- files are entity-specific. Good. |

---

## Part 7: Vocabulary Violations (ADR-0083)

| Location | Current Text | Violation | Suggested Fix |
|---|---|---|---|
| AccountHero line 74 | "Account Intelligence" | System vocabulary | "Account Insights" or just the timestamp |
| ProjectHero line 60 | "Project Intelligence" | System vocabulary | "Project Insights" or just the timestamp |
| PersonHero line 89 | "Person Intelligence" | System vocabulary | "Profile" or just the timestamp |
| AccountHero line 146 | "Building intelligence..." | System vocabulary | "Updating context..." |
| All heroes | "Build Intelligence" button | System vocabulary | "Check for updates" or "Refresh" |
| AccountDetailEditorial line 279 | "Resolution Keywords" | System vocabulary | These serve entity resolution -- consider removing entirely or relabeling "Matching Keywords" |
| ProjectDetailEditorial line 210 | "Resolution Keywords" | Same | Same |
| PersonHero line 198 | "Enriching from Clay..." | System vocabulary | "Looking up profile..." |
| StakeholderGallery | "Add Stakeholder" | Acceptable but could be warmer | "Add person" |
| TheWork readiness | "Readiness Callout" (code only) | N/A (code-only) | -- |

---

## Part 8: Patterns & Structural Observations

### The Magazine Shell Pattern
All three entity detail pages register with `useRegisterMagazineShell` providing:
- `folioLabel`: "Account" / "Project" / "Person"
- `atmosphereColor`: turmeric / olive / larkspur
- `activePage`: accounts / projects / people
- `backLink`: to respective list page
- `chapters`: entity-specific chapter navigation
- `folioActions`: entity-specific header buttons (accounts only have these)

### The Intelligence Field Update Pattern
All three entity detail pages implement the same `handleUpdateIntelField` callback pattern (lines ~186-202 in account, ~122-137 in project, ~130-145 in person). This is copy-paste code. A shared hook could eliminate this.

### The Keywords Pattern
Accounts and projects both implement identical keyword parsing, rendering, and removal logic (AccountDetailEditorial lines 206-329, ProjectDetailEditorial lines 140-261). This is significant code duplication that could be extracted to a shared component.

### The Enrichment Pattern
All three entity types have "Build Intelligence" / enriching state / enrichSeconds. The hooks (`useAccountDetail`, `useProjectDetail`, `usePersonDetail`) each implement this pattern independently.

### The Appendix Pattern
All three appendices share the same visual treatment (double rule, "Reference"/"Appendix" label, section rules) but are entirely separate components with different content. The only shared sub-component is `FileListSection`.

### Chapter Naming Inconsistency
- Account: "The Headline" / "State of Play" / "The Room" / "Watch List" / "The Record" / "The Work"
- Project: "The Mission" / "Trajectory" / "The Horizon" / "The Landscape" / "The Team" / "The Record"
- Person: "The Profile" / "The Dynamic" or "The Rhythm" / "The Network" / "The Landscape" / "The Record"

The naming is intentionally different per entity type (good -- it reflects different jobs). But the shared components default to account-flavored names ("The Room", "Watch List") that get overridden by props. The person page's "The Landscape" matches the project's name but accounts use "Watch List."

### Missing Capabilities by Entity Type

| Capability | Account | Project | Person |
|---|---|---|---|
| Actions (main chapter) | TheWork | -- | -- |
| Actions (appendix) | -- | ProjectAppendix | -- |
| Actions (anywhere) | Yes | Yes | **No** |
| Milestones | -- | Yes (Horizon + Appendix) | -- |
| Lifecycle events | Yes (Appendix) | -- | -- |
| Programs/initiatives | Yes (WatchList) | -- | -- |
| Keywords | Yes | Yes | -- |
| Team management | Yes (Drawer) | -- | -- |
| Merge | -- | -- | Yes |
| Delete | -- | -- | Yes |
| Social links | -- | -- | Yes |
| Duplicate detection | -- | -- | Yes |
| Parent/child hierarchy | Yes (BUs) | -- | -- |
| Entity linking | -- | -- | Yes (PersonNetwork) |
| Risk briefing link | Yes (Reports) | -- | -- |

---

## Part 9: Key Questions for Phase 3

1. **Should actions appear on person detail pages?** People have commitments too. Currently actions only appear on account/project surfaces.

2. **Should TheWork be a shared component?** It's currently account-only as a main chapter, with projects getting a simpler version in the appendix. The inconsistency feels accidental rather than intentional.

3. **Is company context in the hero AND the appendix justified?** The hero shows the description; the appendix shows the full context block with industry/size/HQ. Consider having only one location.

4. **Should "Resolution Keywords" be user-facing at all?** This is a system concept (entity resolution). Users remove keywords but rarely understand why they exist.

5. **Is the meeting readiness callout in the right place?** It appears on account detail (TheWork), project detail (HorizonChapter), and meeting detail. The meeting detail page is the natural home; entity detail could just link to the meeting.

6. **Should the Person detail page have an action creation affordance?** It's the only entity type with no way to create actions from the detail page.

7. **Are the entity list pages doing too many jobs?** PeoplePage in particular handles relationship browsing AND data hygiene (duplicates, unnamed filter). These might be separate surfaces or at least separate modes.
