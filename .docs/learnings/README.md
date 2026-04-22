# Learnings

This directory captures things we've figured out while building DailyOS that might be useful to someone else building anything similar.

## What goes here

One topic per entry. Something we hit, wrestled with, and have at least a provisional answer for. Written as story, not as thesis. First person. Honest about what we still don't know.

Good candidates:
- An architectural choice that took a while to see clearly
- A bug or failure mode that forced a design change
- A contract between subsystems that started accidental and became deliberate
- A question we're still sitting with that's worth thinking about in public

Bad candidates:
- A feature announcement
- A positioning argument
- A summary of an ADR without the story around it
- Anything we're not confident enough to defend

## What it's for

Two reasons, roughly equal weight.

**Build in public.** Much of what we're figuring out in DailyOS is also being figured out by Karpathy's LLM Wiki gist, Garry Tan's GBrain, OpenClaw, Hermes, and a quiet army of people in Slacks and group chats. We've been doing it in our own repo for three months. At some point it starts to look like hoarding. This is a way to share back.

**Teach our future selves.** Three months from now I will not remember why we decided the thing we decided. An ADR records the decision. A learning records the story. Both matter. The ADR is the contract; the learning is the context.

## Shape of an entry

Date-prefixed filename. One h1 title. Lead with the lived moment. Follow the thinking as it actually unfolded, including wrong turns. Cite the ADRs the entry grounds out in, lightly. Close on what's still open.

Target length: 800 to 1500 words. Karpathy-gist-ish. Readable in one sitting.

## Shape of the voice

- First person
- Present tense where the thinking is still active, past tense for the story
- Plain English. No jargon the reader hasn't already earned
- Hedge when you're actually uncertain. Stop hedging when you actually aren't
- No positioning. No selling. No "the category is..." anything
- The goal is "a smart engineer nods and remembers something they half-thought once," not "a leadership team approves a budget"

## Where these might end up

Some will stay internal. Some will become P2 posts. Some will become external gists or blog posts once they've been pressure-tested a bit. The commit to this directory is the first draft; where it lands later is a separate decision.

## Current entries

- [Where code ends and AI begins](2026-04-21-where-code-ends-and-ai-begins.md). The deterministic/probabilistic boundary, how we hit it, how the contract between the two sides emerged.
