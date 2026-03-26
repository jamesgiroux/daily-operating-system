# DailyOS Release Notes

User-facing release notes. Written for the people who use DailyOS every day — not for developers.
This file is the source of truth for the What's New modal. Each version gets one entry.

Format: lead with the story of the release, then features as benefit-led bullets.
Each feature should tell you what it is, where to find it, and what to do first.
No internal jargon. No issue numbers. No "entity intelligence" or "enrichment."
Write like you're telling a customer what got better and how to get started.

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
