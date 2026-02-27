# DailyOS Release Notes

User-facing release notes. Written for the people who use DailyOS every day — not for developers.
This file is the source of truth for the What's New modal. Each version gets one entry.

Format: lead with the story of the release, then features as benefit-led bullets.
Each feature should tell you what it is, where to find it, and what to do first.
No internal jargon. No issue numbers. No "entity intelligence" or "enrichment."
Write like you're telling a customer what got better and how to get started.

---

## v0.15.2 — Observability & Enterprise Context

Now you can see what DailyOS does — and connect it to your company's knowledge.

Every action DailyOS takes is now recorded in a tamper-evident activity log: calendar syncs, briefing updates, security events, and anything unusual. You can browse it, export it, and verify it hasn't been tampered with. It's the kind of transparency a chief of staff should have — you trust it because you can audit it.

**Activity Log** — Go to **Settings → Data → Activity Log** to see what the app has been doing. Events are grouped by day, color-coded by category, and written in plain English ("Calendar synced (47 events)", "Intelligence updated", "Database opened"). Click any entry to see the raw details. Use the category filters to focus on what matters — Security, Data, AI, or Anomalies.

**Export & Verify** — Hit *Export Log* to save a copy of the full audit trail as a JSON file. Hit *Verify Integrity* to confirm the hash chain is intact — if anyone (or anything) has modified a record, you'll know.

**Enterprise Context (Glean)** — If your company uses Glean, DailyOS can now pull from your organization's knowledge graph to enrich briefings and reports. Go to **Settings → Context Source**, switch to Glean mode, and enter your endpoint. Choose Additive (Glean supplements local data) or Governed (Glean replaces local connectors). When Glean is unavailable, everything falls back to local data seamlessly.

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

DailyOS now generates reports — slide decks you can actually share. Account Health Reviews for your internal team, EBR/QBRs for your customers, SWOT analyses, and personal impact reports for yourself. All generated from the intelligence DailyOS has already been building. Every report is a live document: click any field to edit it, and your changes save automatically.

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
- **1:1 meeting intelligence** — When you have a one-on-one, the briefing now focuses on that person — their history, notes, and open actions — not just the account.
- **Background task throttling** — The app is significantly quieter when you're actively working. No action needed; this just happens.

---

## v0.14.1 — /me Page & Professional Context

DailyOS now knows who you are.

The new /me page is where DailyOS learns your professional context — your role, what you own, your annual and quarterly priorities, and the knowledge base that shapes how it thinks about your accounts and meetings. The more you put in, the more useful your briefings become.

- **Your professional context** — Go to the **Me** page and fill in your role, what you're measured on, and your value proposition. This context flows into every briefing and report DailyOS generates.
- **Two-layer priorities** — Add your annual bets and quarterly focus under **My Priorities**. No expiration, no nagging — they stay until you remove them.
- **Context entries** — Under **Context**, add things DailyOS should know: product details, common objections, pricing context, internal processes. DailyOS retrieves these when relevant.
- **File attachments** — Drag a PDF or document onto the **Attachments** section to give DailyOS deeper product and domain knowledge.
