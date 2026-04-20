# DailyOS: what if your AI chief of staff just knew?

**Date:** 2026-04-20. **Author:** James Giroux. **For:** Automattic Radical Speed Month.

Looking for one or two partners for the month.

## The problem

Everyone's trying to get AI into their workday in a way that actually helps. Schedule agents. Meeting prep agents. "Who's waiting on me" agents. The longer you're in a role, the more institutional knowledge you hold, and the more context has to be re-assembled before every meeting, every project, every decision.

The tools we use to manage all that were built for a different era. One where the human did the work. You marked the task complete. You moved it through the status columns. You wrote the meeting summary. You published the report. You were the integration layer between the ten apps that held your life.

AI is making the busy work faster. Not always better. Single-session. Missing context. Half right. Good at telling you what you've already done. What you want is help figuring out what comes next.

And all of it is prompt-driven. Every morning, you type context back in. Who you are. What project. What account. Which meeting. The AI starts from zero. You're the one stitching together the world it needs.

**What if it just knew?**

## The category exists (mostly)

You've probably heard of second brains. Notion, Obsidian, Garry Tan's GBrain. Tools to help you master the mess. Your AI chief of staff that tells you where to be, what's important, and what could be coming.

Most of them are a terminal and a folder of markdown files. Not bad. We're all adults, we can read hashes. But it could be better. It could be proactive. It could be doing things before you ask. OpenClaw and Hermes gesture at that version.

We're Automattic. Pioneers of open source and privacy. Builders. Could we not have something like this, shaped for our teams?

## Where I think it gets interesting

DailyOS is a native macOS app I've been building. It takes the raw stream of your work day (calendar, email, transcripts, CRM, Glean search) and turns it into a briefing you trust. No prompt to start. No context to paste in. It was already paying attention.

- **Prepared, not empty.** Open the app and your day is already there. No dashboards to configure. No statuses to move. Skip a day, nothing breaks.
- **Every claim shows its source.** Click any line of the briefing, see the transcript or email it came from and when. No opaque model output.
- **Every claim carries a confidence level.** Trust this. Use with caution. Verify first. You see what to rely on, not just what was generated.
- **Corrections stick.** When you tell the AI it's wrong about something, it stays wrong in the ledger. The next enrichment cycle cannot quietly put the bad data back. This is the single bug that kills every AI assistant I've tried.
- **Individual context, not organisational.** Glean knows what the company knows. DailyOS knows what *you* know. Different layer, complementary.
- **Open by default, safe by design.** Content stays on your laptop. You bring your own Claude key. Nothing about your workday leaks to our servers, because there aren't any. Markdown output, so any AI tool you use next can read it.

*It's running today. I use it every morning. The architecture for the next version is settled, and I'm ready to build.*

## The month

Get DailyOS stable enough that folks are willing to try it. Stable means the briefing doesn't lie and your corrections stick. Safe means your work stays yours, full stop. Harden the install path, ship the parts that make meeting prep and account context trustworthy, and invite curious Automatticians in. See what they do with it.

## Metrics

- **Do people try it, and do they keep using it?** The honest first signal.
- **Trust calibration.** When the app says "trust this," does the human actually find it correct? No silent errors in the high-confidence band is the bar.
- **Zero silent overwrites.** The AI should never quietly undo a user correction. This is the bug I'm most determined to kill.
- **No data surprises.** Your content stays on your laptop. Glean stays your own relationship. Nothing leaks anywhere you didn't put it. This is table stakes at Automattic, and it's the design from day one.

## Who I'm looking for

Comfort with Rust and Tauri (there's backend work that splits cleanly across two people). A designer who wants to work on the hard UX problem of showing trust and sources without turning the screen into a wall of footnotes. Or someone willing to be one of the first curious users and help shape what good looks like. Any of those makes the month count.
