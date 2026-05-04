# I438 — Onboarding: Prime DailyOS — First Content Ingestion Step

**Status:** Open
**Priority:** P0
**Version:** 1.0.0
**Area:** Frontend / Onboarding

## Summary

The final step of the first-run wizard teaches the user the most important habit in DailyOS: feeding it context. Transcripts, documents, and connector data are what build context — without them, the system has nothing to work with. This step gives the user an immediate win (something gets processed right now) and establishes the feeding habit before the automated connectors (Quill, Clay, Granola) take over. The pedagogical message is explicit: "DailyOS gets better when you give it context. Right now, that's you. Eventually, it's automatic."

**ADR-0083 note:** All user-facing copy in this step uses product vocabulary. "Context" not "intelligence." "Updates" not "signals." "Building context" not "enrichment." "Briefings and insights" not "intelligence." "Find new information" not "enrich."

## Acceptance Criteria

1. **Step 7 of the first-run wizard** is titled "Prime DailyOS" (or equivalent — the label should convey that the user is seeding the system, not just uploading a file). It appears after "Your first account" and before or alongside "About you." The step explains in 1–2 sentences why this matters: "Context builds from what you give it — meeting transcripts, documents, notes. The more you share, the more DailyOS understands. Until your connectors are running, you're the source."

2. The step offers two paths — the user chooses one (or both):

   **Path A — Drop something now (manual):**
   - A drag-and-drop zone (or file picker button) labelled "Drop a transcript or document"
   - Accepts: `.txt`, `.md`, `.pdf`, `.docx` files and plain text paste
   - On drop: the file is written to `_inbox/` immediately, the file watcher picks it up, and a "Processing..." indicator appears
   - If the user added an account in Step 5: the UI suggests linking the dropped file to that account ("This looks like it might be about [Account Name] — link it?")
   - The user does not have to wait for processing to complete before proceeding

   **Path B — Connect a feeder (automated):**
   - Three connector options shown as cards: **Quill**, **Granola**, **Google Drive**
   - Each shows: what it does in one sentence, whether it's detected as installed (Quill checks for the bridge, Granola checks for the cache file, Drive checks for OAuth)
   - Connecting one sets it up and shows "Will automatically feed DailyOS after your next meeting"
   - A "Connect Clay" option is shown if Clay OAuth is available

3. After choosing either path (or both), a confirmation message shows: "DailyOS is primed. Context will build from what you just gave it, and from your connectors as they run." The step completes and the wizard can proceed to finish.

4. "Skip for now" is available but labeled honestly: "Skip — I'll feed it manually later." Skipping shows a persistent reminder on the `/me` page and the Settings connectors section: "DailyOS hasn't received any context yet. Connect a feeder or drop a file to start building intelligence."

5. If the user dropped a file in Path A: after completing the wizard and landing on the daily briefing, a notification/callout appears when processing completes: "Your [filename] has been processed — check [account name]'s page to see what we found." This closes the loop and shows immediate value.

6. The step's copy adapts to what the user did in previous steps. If they connected Quill in Step 2 (Google) already and Quill is installed, Path B pre-selects Quill as "already connected" and the step says "Quill will automatically import your transcripts — you're ready." If no connectors are available and no file is dropped, the skip label is more urgent: "Skip — I understand intelligence will be limited until I feed it."

## Dependencies

The onboarding wizard must exist (v0.16.0, mostly done). A dormant `PrimeBriefing.tsx` chapter file exists in `src/components/onboarding/chapters/` but is not wired into the wizard's chapter list. This issue activates and rewrites it as the content ingestion step. The current wizard has 6 steps — this adds a 7th. Benefits from I424 (Granola) and I426 (Google Drive connector) being built, so all three connector options are functional. If connectors aren't ready, Path B shows them as "Coming soon" rather than active.

## Notes / Rationale

**The habit problem:** Every intelligence tool fails when users don't maintain it. DailyOS's answer is automation (Quill, Clay, Granola) — but automation only works if the user sets it up. Onboarding is the only moment where the user's attention is guaranteed. Use it to either (a) get at least one piece of context into the system right now, or (b) establish an automated feeder that will do it going forward. Either outcome means the user opens the app the next day to find something new is ready — which is the core retention hook.

**Why "Prime" not "Upload":** "Upload" sounds like file management. "Prime" sounds like preparation. The semantic difference matters — the user is not storing a file, they are activating the system. The verb should convey that this action has a visible downstream effect.

**The "until it feeds itself" framing:** Quill, Granola, Clay, and Google Drive eventually make this step invisible — transcripts flow in after meetings, documents sync automatically, contacts enrich overnight. But in the first session, the user is the signal source. Saying this explicitly sets the right expectation and makes the connector setup feel important rather than optional.
