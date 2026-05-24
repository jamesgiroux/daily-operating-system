# DailyOS Release Notes

User-facing release notes. Written for the people who use DailyOS every day — not for developers.
This file is the source of truth for the What's New modal. Each version gets one entry.

Format: lead with the story of the release, then features as benefit-led bullets.
Each feature should tell you what it is, where to find it, and what to do first.
No internal jargon. No issue numbers. No "entity intelligence" or "enrichment."
Write like you're telling a customer what got better and how to get started.

---

## v1.2.2 — Your role, your dossier

DailyOS started as a chief of staff for Customer Success, but most of what it does applies to any role that lives in account relationships. v1.2.2 makes that real. Pick your role in onboarding (or change it later in Preferences) and the AI's thinking shifts to match — the language, the dimensions it scores, the things it watches for, the vitals it surfaces. Affiliates and Partnerships joins Customer Success, Product Marketing, and Core as a ready-to-use role today.

Plus a heavy polish round on the design pass that landed in v1.2.1: clearer Health, Context, and Work tabs, and a long list of capture fields you can finally edit by hand for the details AI does not know yet.

**Your role, your dossier.** The role you pick changes what the AI thinks about, not just the labels you see. Affiliates? Partner Revenue, Partner Stage, and Performance Score lead your vitals strip — and AI write-ups are framed around campaign cycles, not renewals. Switch roles any time from Preferences. Onboarding asks every new user up front.

**Health & Outlook leads with The Call.** Confidence, peer benchmark, and recommended start window — the verdict on the relationship in one line. The benchmark cell ("Above peers / At peers / Below peers") compares you to similar customers using cross-account context, with a note on which sources informed the read. Below that, dimension bars with role-aware names and the supporting tension between computed score and observed direction.

**Context tab tells the relationship story.** Thesis, who's in the room, what matters to them (with compliance and competitive context), what we have built together, and their voice — verbatim quotes pulled from your conversations. Commercial shape, Technical shape, and Relationship fabric round it out. Most fields were "— not captured" before; now every one is editable inline. Selectors for Yes/No-style fields, free text for the rest.

**Work tab focuses on next.** Commitments, suggestions, programs, recently landed. Empty chapters disappear from navigation instead of leaving dead links.

**One way to correct anything.** "Is this accurate?" replaces the old mix of thumbs up/down, inline edits, and field-conflict prompts. Yes / No on individual claims, Yes / Partially / No on AI write-ups (Partially opens an annotation; No opens an inline editor). Every correction teaches the system what to trust.

**Daily AI budget you actually control.** Diagnostics now shows a single user-defined daily token cap that's enforced, replacing the old 50k display that didn't do anything. Set it as low or high as you like.

**Less wasted background work.** Archived accounts are skipped by the AI queue. Suggestions and commitments deduped at the source — no more dozens of identical rows piling up across runs.

**The Work tab actually renders now.** On larger accounts, the Suggestions and Commitments sections were rendering as blank space due to a CSS animation gate that should never have been load-bearing on operational content. Removed entirely from Work surfaces; the magazine-editorial polish stays where it belongs.

**Smaller things.** Compliance section appears once (in "What matters to them"), not twice. Products list restructured from long verbose paragraph dumps into compact grouped product / feature lists. The contamination detector no longer flags your own subdomains as if they belonged to a foreign account. Health scoring no longer silently feeds on fabricated zeros when a database query fails — failures log honestly.

---

## v1.2.1 — The right account, every time

DailyOS has always known a lot about your accounts. The problem was linking. A meeting with someone from Acme might file under a parent company, a subsidiary, or not link at all — depending on subtle quirks in how the meeting was structured. This release replaces that heuristic guessing with a deterministic system: the same meeting will always reach the same answer, and you can see why.

**Meetings link to the right account, reliably.** The system now applies a defined set of rules — sender domain, title, attendee history, meeting series — in a specific order. No more coin-flips between parent and subsidiary. No more internal team syncs accidentally linking to a customer. Open any meeting to see which account it's connected to and dismiss the link if the system got it wrong.

**Dismissals stick across the whole app.** When you dismiss an account link on a meeting, it's gone — from that meeting, from the briefing, from the email thread. It doesn't bounce back after the next sync. The system remembers your correction and applies it everywhere.

**New people surface before they matter.** When the system spots someone regularly in your meetings who isn't in your stakeholder list yet, they appear in a Pending queue on the account page. One click to confirm, one click to dismiss. You stay in control — nothing gets added without your say.

**Triage cards do something now.** Health triage items (risks, gaps, aging concerns) now have Snooze and Resolve buttons. Snooze a card if the issue is known and being handled. Resolve it when it's fixed. The triage list clears as you work through it, instead of just accumulating.

**Internal meetings stay internal.** A standing bug caused some internal team meetings to get classified as customer meetings, pulling them into account briefings where they didn't belong. Fixed. Team syncs, all-hands, and 1:1s with colleagues now stay out of your customer-facing context.

**Your backups are real now.** A quiet bug in the backup system was producing near-empty files instead of actual copies of your database. The backup now captures everything — including any unwritten WAL data — and we verify the result before calling it done. If you had backups that were suspiciously small (under 64KB), they weren't real. New backups will be.

---

## v1.2.0 — Actions that close the loop

DailyOS used to be one-directional: AI produces, you consume. Now actions are a two-way street. Capture your own tasks, push them to Linear, and let the system learn what matters to you.

**Add a task from anywhere.** Press Cmd+K, type "add action," and capture what's on your mind. If you're looking at an account page, the task auto-links to that account. It shows up in your briefing, meeting prep, and entity pages alongside AI-extracted items. No separate task manager needed.

**Push actions to Linear with one click.** Hover over any action and click the Linear icon to create an issue. The app picks the right project automatically based on your linked accounts. Pushed actions show a persistent badge with a clickable link back to Linear.

**AI now recommends what to do next.** Each account page shows 2-3 specific, concrete recommended actions based on health signals, recent meetings, and open commitments. Track them to add to your list, or dismiss to teach the system what you don't need.

**Actions that need decisions stand out.** Tasks blocked on approvals, budget sign-offs, or legal review get a "Decision needed" badge so you can see at a glance what's waiting on someone else.

**The system learns from your rejections.** When you dismiss a suggested action, DailyOS remembers. It won't suggest the same thing again for that account. Sources that consistently miss the mark get quieter over time.

**Objectives connect to real conversations.** When your customer mentions a goal in a call and you already have that objective tracked, the system links the evidence automatically. You'll see "3 mentions in calls" on objectives that keep coming up.

**Zero-guilt aging.** Stale actions fade and auto-archive after 30 days. Urgent items and anything linked to an objective are exempt. The briefing lets you know when items are aging out, without pressure.

**One-click Node.js setup.** New users without Node.js installed no longer need to download it manually. The setup wizard handles the entire installation automatically.

---

## v1.1.3 — Smarter meetings, easier setup

Setting up DailyOS just got simpler, and your meeting briefings just got smarter.

**Install Claude Code with one click.** No more copying terminal commands. If you have Node.js installed, a single button in the setup wizard handles everything. If you don't have Node.js yet, the app now points you to the right place instead of a confusing download page.

**Meeting briefings now speak the right language.** Customer meetings still get the full treatment: wins, risks, champion health, and commitment tracking. But internal team meetings now focus on what actually matters for those conversations: decisions made, who owns what, and blockers surfaced. Your 1:1s extract coaching moments and personal commitments instead of trying to find a "champion." Training sessions and personal calls get a quick summary without burning time on analysis that doesn't apply.

**People get proper workspaces.** When you manually add a person, they now get a full workspace directory with the same structure as accounts and projects: a profile, organized subfolders for transcripts and notes, and files that Claude Desktop and other tools can read.

**Status checks actually work now.** The Settings page was linking to the wrong product (Claude Desktop instead of Claude Code) and the "Check again" button was using a stale cache. Both are fixed. All status indicators across the app now update together when you refresh.

---

## v1.1.2 — Your meetings, in the right place

Since mid-February, meeting transcripts and notes were quietly piling up in a catch-all archive instead of filing themselves under the right account. v1.1.2 fixes the pipeline, recovers what it can, and makes the whole system smarter about where things belong.

**Transcripts route to the right account again.** The system now matches meeting attendees to known accounts by email domain. A call with someone from a customer's company files under that account automatically. When that doesn't work, it falls back to matching account names in the meeting title, then checks if the transcript itself already knows which account it belongs to. If nothing matches, it goes to the archive. Every routing decision is logged so you can see why a file ended up where it did.

**We recovered the backlog.** A one-time recovery command scans the archive for customer meeting transcripts that should have been filed under accounts. It matched about a third of the stranded files automatically. Internal meetings (team syncs, all-hands) are correctly left in the archive.

**Drop a folder, get an account.** Create a new folder under Accounts in Finder and it automatically appears in DailyOS within seconds. Rename it and the app updates to match. Rename it in the app and the folder updates to match. No more manual setup required.

**Adjustable text size.** New control in Settings to increase or decrease the base font size across the entire app. Your preference is saved between sessions.

---

## v1.1.1 — Making the magic reliable

v1.1.0 introduced lifecycle intelligence and stakeholder management. v1.1.1 makes it all work consistently. Meetings that should have generated briefings now do. Intelligence that referenced the wrong company is filtered out. The daily briefing focuses on what matters today, not yesterday's noise.

**Meetings actually show up now.** Some meetings with customer names in the title or known contacts on the invite weren't generating briefings at all. We traced this to six independent gaps in how meetings get linked to accounts and fixed all of them. If a meeting should have a briefing, it gets one.

**Smarter about who's who.** When someone from a partner company attends meetings across multiple accounts, the system no longer links every meeting to every account they're associated with. The linking is now proportional. Three people from Acme on a call? That's clearly an Acme meeting. One person who works across twelve accounts? Their vote is weighted accordingly.

**Your team list makes sense.** The "Your Team" section was showing 20+ internal colleagues for some accounts. Now it only includes people you've intentionally added, not everyone who ever sat in on a call.

**Stakeholder roles work properly.** Changing someone's role from "End User" to "Decision Maker" no longer adds a second badge. You can remove any role, not just ones you assigned yourself. AI-suggested stakeholders that already appear in your confirmed list are automatically hidden.

**Cleaner daily briefing.** Suggested actions moved to the Actions page where they belong. Lifecycle updates only show if they happened today. The "Last updated" orphan label is gone. The DB size warning toast is gone. Less noise, more signal.

**Meeting agendas know what kind of meeting it is.** A QBR now gets strategic risks and programs. A 1:1 gets action items. A training meeting skips account intelligence entirely. And all meetings, not just the first batch of the day, get AI-refined agendas with "why this matters" context.

**Notification controls.** New section in Settings lets you toggle briefing alerts, meeting note alerts, and connection alerts independently. Set quiet hours so the app doesn't interrupt you outside work hours. Transcript notifications are batched instead of one per meeting.

**Under the hood.** All GitHub Actions pinned to commit SHA. Gravatar API key moved from config file to macOS Keychain. Google OAuth trimmed from 6 scopes to 3. The main account detail page was decomposed from one 1,245-line file into 12 focused components. Intelligence scoring now applies time decay so recent assessments matter more than six-month-old ones.

---

## v1.1.0 — It should just know

This release makes DailyOS act on what it learns. The system now detects lifecycle transitions — renewals confirmed, accounts going quiet, contracts approaching — and reports what it did in your daily briefing. You confirm with one click or correct what's wrong. No pipeline to manage. No stages to update. It just knows.

**Lifecycle intelligence.** When a renewal is confirmed (order form signed, Salesforce opportunity closed), DailyOS automatically updates the account, rescores health, and tells you in the morning briefing. Approaching renewals, engagement drops, and at-risk signals surface the same way. You click "Looks good" or "Fix something" — corrections teach the system to do better next time.

**Stakeholder roles that stick.** Designate a champion, set engagement levels, assign multiple roles per person — your designations survive every refresh. The system suggests new stakeholders it discovers from meetings and Glean, but never overwrites what you've set. Manage stakeholder roles from the account page or the person detail page.

**Products appear automatically.** DailyOS discovers your customer's products from Salesforce via Glean and shows them on the account page. No inventory to manage. Wrong product? Correct it inline and the system learns.

**Source attribution everywhere.** Key account fields now show where their data came from — "via Salesforce," "via Zendesk," "you noted." When the system finds a different value than what you entered, it shows the conflict and lets you accept or dismiss. Your edits always win.

**Smarter with your time.** Background AI work now runs on a budget with pause guards. Stale intelligence automatically fades from active views. Dismissed items don't bounce back. The daily briefing generates faster with email processing running in the background instead of blocking.

**The Outlook chapter breathes.** Renewal confidence, growth opportunities, and commercial reality each get their own section with editorial spacing. Risk factors read as prose, not bullet lists. Expansion signals show stage and ARR impact.

---

## v1.0.4 — Your edits, your data

This release is about trust. When you edit something in DailyOS, it should stay edited. When you open the app, your data should be there. When you click a button, it should do what it says.

**Your edits survive account refreshes.** Previously, editing a stakeholder role, a risk, or a recent win could get silently overwritten the next time an account refreshed. The system now recognizes fields you've touched and protects them — your changes stick, period.

**Your briefing loads instantly.** Emails on the daily briefing used to show up blank for the first 10-15 seconds while waiting for a background refresh. Now your cached emails appear immediately when you open the app, and fresh data fills in behind the scenes.

**Tools and Reports work where they should.** The toolbar dropdowns on account pages were appearing on the wrong pages and failing when clicked. We rebuilt them from scratch — each dropdown now manages its own state, and stale actions from a previous page can never bleed through to a new one.

**Stakeholder counts tell the truth.** The "X of Y stakeholders" stat was inflated by counting every linked contact, not just the people who actually appear in your account's stakeholder view. Fixed to count only stakeholders with defined engagement.

**Business unit emails route correctly.** Emails for accounts with business units now route to the right parent or child account during processing.

---

## v1.0.3 — The Meeting Record

After a meeting, the page used to show your prep with outcomes stacked on top. Now it becomes something you'd actually send to your VP.

**Your meetings write their own report.** Process a transcript and the page transforms into a Meeting Record — a structured executive document with a headline, summary, engagement dynamics, champion health, categorized findings with direct quotes, and action items. Pre-meeting prep slides into a collapsed appendix at the bottom. Every section earns its space.

**See what changed since last time.** "The Thread" shows what happened between this meeting and the previous one with the same account: actions you completed, actions still open, how the health score moved, and who's new in the room. First meetings get a clean introduction instead.

**Know which predictions landed.** "What We Predicted vs What Happened" compares what your briefing warned you about against what actually came up. Confirmed predictions build your confidence in the system. Surprises highlight blind spots. The system learns from both — correct predictions make their source more trusted next time.

**Act on what you heard.** Suggested actions extracted from the transcript appear with Accept and Dismiss buttons right in the record. Accept moves them to your pending queue. Dismiss removes them. Pending actions show a Done button. Everything stays in place when you click — no page jumps.

**The meeting page knows what time it is.** Before the meeting: full editable briefing. During: read-only prep with a live "in progress" indicator. After: processing progress with phase-by-phase updates. Once processed: the Meeting Record. The folio bar tells you exactly which stage you're in.

**A record for Claude, too.** After processing, a structured markdown file lands in your account's Meeting Records folder. Ask Claude Code "what happened in the Acme QBR?" and it finds the answer from a single document.

---

## v1.0.2 — Scores you can trust

Health scores were telling the wrong story. An account that just renewed with ARR growth was showing "at risk." A CSM who tagged a champion saw the role disappear after the next refresh. Email activity wasn't factoring into health at all. This release fixes all of it.

**Health scores actually make sense now.** Five formula bugs were causing healthy accounts to score low. A recently-renewed account now scores high instead of rock-bottom. Stakeholder coverage rewards what you have instead of penalizing what's missing. Champion engagement is recognized even when you haven't formally tagged someone. Every account's score will recalculate automatically — expect scores to shift upward for healthy accounts.

**Your edits stick.** When you set someone as a champion, technical lead, or executive sponsor, that designation now survives account refreshes. Previously, the AI could reorder stakeholders during a refresh and your role assignments would silently move to the wrong person. Fixed.

**Email activity counts toward health.** Your inbox conversations now factor into account health scores. If you have active email threads with an account's stakeholders, the system sees that engagement. Previously, email signals were collected but never made it to the health scoring engine.

**Actions flow from transcript to triage.** When you process a meeting transcript, extracted actions now appear on the Actions page immediately — no navigation required. The page auto-switches to show new suggestions, and you'll see a notification when they arrive.

**Meeting briefings show your prep.** Expanding any meeting on the daily briefing now shows your prep grid (what to discuss, what to watch, recent wins) and a pre-meeting action checklist. These components existed but weren't connected after a recent redesign.

**Transcripts file to the right place.** Process a 1:1 meeting and the transcript files to that person's folder. Process a project meeting and it goes to the project folder. Previously, only account-linked meetings were routed correctly.

---

## v1.0.1 — Your inbox, under control

The Correspondent got smarter. DailyOS now understands what's happening in your inbox — not just what arrived, but what needs your attention, what you committed to, and which accounts have gone quiet.

**Archive from anywhere, and it sticks.** Archive an email on The Correspondent or the daily briefing and it disappears from every page instantly — and from Gmail too. Undo if you change your mind. Pin important emails to keep them at the top of their priority band. Open any email in Gmail with one click.

**Commitments get tracked.** When DailyOS spots a commitment in an email ("Will confirm budget by Friday"), you can track it as an action right there — with a title, due date, owner, and the account it belongs to. Tracked commitments show up in your action queue and stay visible on the email even after you reload.

**Know when accounts go quiet.** If an account that normally emails you weekly suddenly goes silent, a "Gone Quiet" notice appears on The Correspondent. Dismiss it if the silence is expected — your dismissal teaches the system what matters. Gone-quiet alerts also surface in your daily briefing.

**Emails connect to meetings.** When someone who emailed you is in an upcoming meeting, you'll see a meeting badge on their email. Click it to jump straight to the briefing. Meeting detail pages now show recent correspondence from attendees.

**Behind the scenes.** Your database now monitors its own growth and cleans up old data automatically — resolved emails after 60 days, deactivated signals after 30, old signal events after 180. User corrections are never deleted. A storage card in Settings → Diagnostics shows your current DB size.

---

## v1.0.0 — Your chief of staff is ready

DailyOS is a personal chief of staff that runs entirely on your Mac. Connect your Google Calendar and Gmail, and it prepares your workday before you open the app — daily briefings, meeting prep, account insights, and action tracking. No servers, no subscriptions beyond Claude. Your data never leaves your machine.

This is the 1.0 release. The architecture is solid, the intelligence is real, and the experience is designed for Customer Success professionals who manage a book of business.

**Every account gets a health score.** Six dimensions — champion health, stakeholder coverage, engagement patterns, meeting cadence, signal momentum, and financial proximity — combine into one number with a trend arrow. Sparse accounts get an honest confidence qualifier instead of a misleading score. Open any account to see the full breakdown, with the evidence behind every dimension.

**Your meetings know what happened last time.** When you process a transcript, DailyOS extracts categorized wins (not just "things went well" — specific adoption milestones, expansion signals, advocacy moments), risks with urgency levels, champion health changes, and commitments made. All of this flows into the next meeting's briefing automatically. Walk into every meeting knowing what matters.

**Success Plans keep your accounts on track.** Set objectives and milestones for any account. Choose from four templates (onboarding, growth, renewal, at-risk) or build your own. When a lifecycle event happens — like a renewal closing — related milestones auto-complete. AI suggests new objectives based on what it hears in your meetings.

**Search everything.** Press Cmd+K to find any account, person, project, meeting, or action. Results appear in under 300 milliseconds.

**Works offline.** Disconnect from the internet — your briefings and account context are all cached locally. A status indicator shows what's current and what's stale. No blank screens, ever.

**Your data, explained and exportable.** Settings tells you exactly what's stored and for how long. Export everything as a ZIP. Clear your data or delete everything with one click. This is your brain — you own it completely.

**Glean integration (for enterprise teams).** If your company uses Glean, DailyOS pulls from Salesforce, Zendesk, Gong, Slack, and your org directory — producing richer context than local-only analysis. Everything still runs on your Mac; Glean just gives it more to work with. Connect during onboarding or in Settings.

**Built to stay out of your way.** Background work runs on dedicated threads — no beach balls during updates. If something fails in the background, it restarts automatically. The app launches to a branded welcome screen instantly, not a blank window. Errors tell you what happened instead of failing silently.

---

## v0.16.0 — A better start

Before today, opening DailyOS for the first time meant staring at an empty screen. No guidance, no demonstration of what's possible, no clear path to getting value. You had to know what to connect and what to fill in — without any indication of whether it was worth the effort.

That's fixed.

**See it working before you connect anything.** Click "Try with demo data" from the empty dashboard and DailyOS loads a real-looking workspace — sample customer accounts, upcoming meetings with pre-built briefings, active action items, and email context. It shows you what the app looks like when it's actually doing its job. When you're ready to bring in your real data, click "Connect real data" and the demo clears.

**A proper first-run walkthrough.** On first launch, a guided wizard takes you through everything that matters: verify Claude Code is running (required — without it the app can't build context for you), connect your Google account, set your role and work domain, add your first customer account, and give the system something to work with. Each step takes under a minute. Skip anything except Claude Code. If you close the app halfway through, your progress is saved.

**Empty pages are gone.** Every page in the app now explains what it does and what to do next — even when it has no data. Every surface has a direct action button and a reason to continue. No more blank white pages.

**Claude Code status in Settings.** Settings → System now shows whether Claude Code is installed and signed in, with a one-click path to fix it if not.

---

## v0.15.2 — Observability & Company Knowledge

Now you can see what DailyOS does — and connect it to your company's knowledge.

Every action DailyOS takes is now recorded in a tamper-evident activity log: calendar syncs, briefing updates, security events, and anything unusual. You can browse it, export it, and verify it hasn't been tampered with. It's the kind of transparency a chief of staff should have — you trust it because you can audit it.

**Activity Log** — Go to **Settings → Data → Activity Log** to see what the app has been doing. Events are grouped by day, color-coded by category, and written in plain English ("Calendar synced (47 events)", "Briefing updated", "Database opened"). Click any entry to see the raw details. Use the category filters to focus on what matters — Security, Data, AI, or Anomalies.

**Export & Verify** — Hit *Export Log* to save a copy of the full audit trail as a JSON file. Hit *Verify Integrity* to confirm the hash chain is intact — if anyone (or anything) has modified a record, you'll know.

**Company Knowledge (Glean)** — If your company uses Glean, DailyOS can now pull from your organization's knowledge to improve briefings and reports. Go to **Settings → Context Source**, switch to Glean mode, and enter your endpoint. Choose whether Glean supplements your local data or replaces Google Calendar and Gmail as the primary source. When Glean is unavailable, everything falls back to local data seamlessly.

**Touch ID (improved)** — App unlock is now native and instant. The old approach could occasionally stall — the new one won't.

---

## v0.15.1 — Security

Your data, locked down.

DailyOS now encrypts everything on disk. Your database, your briefings, your relationship context — all protected with AES-256 encryption backed by your Mac's Keychain. The app locks itself when you step away and unlocks with Touch ID when you come back. And the AI pipeline that builds your briefings is now hardened against adversarial input — so a cleverly-crafted calendar invite or email subject can't trick the system.

**Encryption** — happens automatically on first launch. Your existing data migrates in place. Nothing to configure, nothing to remember. If you ever need to verify: `file ~/.dailyos/dailyos.db` should show `data`, not `SQLite 3.x database`.

**App Lock** — after 15 minutes of inactivity, the app locks and shows a full-screen overlay. Touch the fingerprint sensor to get back in. Change the timeout (or turn it off) in **Settings → System → Security**.

**iCloud Warning** — if your workspace folder lives under iCloud Drive, Desktop, or Documents sync, you'll see a one-time heads-up. Local data and cloud sync don't mix well — the warning explains your options.

**Meeting Briefing Refresh** — switching an account or project on a meeting now updates the briefing content automatically. No more clicking refresh after reassigning a meeting.

---

## v0.15.0 — Reports

Your work, made presentable.

DailyOS now generates reports — slide decks you can actually share. Account Health Reviews for your internal team, EBR/QBRs for your customers, SWOT analyses, and personal impact reports for yourself. All generated from the context DailyOS has already been building. Every report is a live document: click any field to edit it, and your changes save automatically.

**Account Reports** — open any account and click the **Reports ▾** button in the top bar.

- **Account Health Review** — A 5-slide internal briefing: the state of the relationship, what's working, what's struggling, value delivered, and what's coming. Start here before your next QBR prep or team update. Hit *Generate* and you'll have a first draft in about a minute.
- **EBR / QBR** — A 7-slide customer-facing business review covering what happened, value delivered, metrics, challenges, the road ahead, and next steps. When it's ready, use *Export PDF* to save a shareable version.
- **SWOT Analysis** — A 5-slide strategic view: strengths, weaknesses, opportunities, and threats. Good for account planning before a big renewal conversation.

**Personal Reports** — open the **Me** page (your portrait icon in the nav) and use the **Weekly Impact** or **Monthly Wrapped** buttons in the top bar.

- **Weekly Impact** — A 5-slide editorial review of your prior week: priorities moved, wins, what you did, what to watch, and what carries forward. Generates automatically every Monday morning so it's waiting for you.
- **Monthly Wrapped** — Your prior month as a celebration, not a report. One insight per screen, bold colors, and a personality type matched to your role. Generates automatically on the 1st of each month. Worth scrolling all the way through.

---

## v0.14.3 — Google Drive

Your docs, inside your briefings.

Connect Google Drive and import documents, spreadsheets, and presentations directly into any account's workspace. Watch mode keeps files in sync automatically — when the Drive file changes, your account context updates too.

- **Import from Drive** — Open an account, go to its Documents section, and use *Import from Drive* to pick files via Google Picker. Docs, Sheets, and Slides are all supported. The file lands in the account's workspace and starts informing its briefings immediately.
- **Watch Mode** — Choose *Watch* instead of *Import Once* and DailyOS will check for changes hourly, updating account context when the source file changes.
- **Drive Settings** — Go to **Settings → Connectors → Google Drive** to manage watched documents, see last sync times, and trigger a manual sync.

---

## v0.14.2 — Role Presets & Performance

DailyOS speaks your language now.

Every role preset — Customer Success, Sales, Partnerships, Product, Leadership, and more — now fully drives the language, labels, and priorities throughout the app. Your briefings, stakeholder cards, and account sorting all adapt to how your role actually works.

- **Role-aware language** — Meeting prep, actions, and notifications now use your role's vocabulary. If you haven't set your role yet, go to **Settings → You** and pick the preset that fits.
- **Stakeholder and team roles** — Role badges on stakeholder cards now come from your preset's role definitions. Open any account and check the stakeholder section to see the updated options.
- **Preset-driven account sorting** — Accounts now sort by what matters to your role: renewal proximity for CS, deal stage for Sales, ARR for Leadership. You can still override the sort manually.
- **1:1 meeting briefings** — When you have a one-on-one, the briefing now focuses on that person — their history, notes, and open actions — not just the account.
- **Background task throttling** — The app is significantly quieter when you're actively working. No action needed; this just happens.

---

## v0.14.1 — /me Page & Professional Context

DailyOS now knows who you are.

The new /me page is where DailyOS learns your professional context — your role, what you own, your annual and quarterly priorities, and the knowledge base that shapes how it thinks about your accounts and meetings. The more you put in, the more useful your briefings become.

- **Your professional context** — Go to the **Me** page and fill in your role, what you're measured on, and your value proposition. This context flows into every briefing and report DailyOS generates.
- **Two-layer priorities** — Add your annual bets and quarterly focus under **My Priorities**. No expiration, no nagging — they stay until you remove them.
- **Context entries** — Under **Context**, add things DailyOS should know: product details, common objections, pricing context, internal processes. DailyOS retrieves these when relevant.
- **File attachments** — Drag a PDF or document onto the **Attachments** section to give DailyOS deeper product and domain knowledge.
