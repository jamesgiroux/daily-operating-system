# RSM Final Update — Voiceover Script

Performable voiceover for the showcase video. Talking-head + slides cutting back and forth;
this VO carries the story, the slides punctuate it. Maps 1:1 to the deck at
`2026-05-25-dailyos-work-record.html` (18 slides).

**Delivery.** Conversational, first person, thinking out loud — not a lecture. Slight hedges
("I think," "what I keep noticing") are intentional; they're the voice. Reads as one
continuous take if you want it to. Roughly 6–7 minutes at an unhurried pace.

**Yours to make real.** A few lines reach for a personal/emotional beat (flagged inline with
`†`). Keep them only if they ring true — swap in your own reaction, or cut. Don't perform a
feeling you don't have.

---

## S1 — Cover · "Democratizing Personal Intelligence"

This is the wrap on Radical Speed Month. The title's a bit of a leap, I'll admit it — so bear
with me, and I'll try to earn it by the end. Mostly I want to show you what we built this
month, and why I think it ends up mattering more than the thing itself.

## S2 — The pain · "Is this a fact, or did the AI just make it up?"

Let me start with something I think everyone here has felt. You ask an AI a question, it hands
back this confident, polished answer — and you have no real idea whether you can use it. So you
go check it yourself. And that's the part that gets me, because checking it is exactly the work
the AI was supposed to take off your plate. The longer I sat with that, the more it stopped
looking like a model problem and started looking like an architecture problem. The model
produces, and the system around it has no idea what it just made.

## S3 — The trust question · "Can you define trust in code?"

So the question I kept circling all month was whether you could build for trust, not just for
accuracy. And trust is a slippery thing to pin down. It's more of a gut feeling than a fact —
something you arrive at without quite reasoning your way there. I honestly wasn't sure it was a
thing you could put into code at all. That's really what the month turned into. Me trying to
find out.

## S4 — How humans decide trust · "It's really micro-signals compounding."

Before I went anywhere near the code, I went the other direction and asked how we do it. Think
about meeting someone new. You don't sit there and decide to trust them. Your eyes and ears
just pick up all these tiny signals, and they pile up into an opinion before you've even
finished a sentence. And I think that pile-up is the whole thing. It's never one signal. It's a
bunch of them compounding at once.

## S5 — The same logic, in code · "Code has no senses. It has memory."

The trouble is, code can't do that. It doesn't have senses. All it has is memory — the stuff it
can recall later. So if you want it to do what your gut does, those signals can't live in a
glance or a tone of voice. They have to live inside the memory itself. Every single fact has to
carry its own evidence around with it. We started calling a fact that travels with its signals
a claim. And that little unit is what everything else gets built on.

## S6 — The blind spot · "Smart isn't the same as trustworthy."

Now I'll be straight with you — we are not the only ones who think memory is the unlock here.
Karpathy's got a wiki his AI keeps. Garry Tan's got a brain his reads before every reply. The
compound-engineering crowd is all over it. Everybody's racing to give AI a better memory, and
honestly, I think that instinct is right. But what I keep noticing is that it's almost all
memory, and barely any of it is judgment. Knowing what to actually trust, what to flag, what to
leave out. So the memory keeps getting smarter, and no more trustworthy.

## S7 — Flat vs weighted memory · "Every fact, the same weight."

Let me just show you the difference. On the left is memory the way it works today. You ask
about an account, and it gives you everything it's got at the same volume — the sponsor, the
renewal, some plan from back in February, all flat, all equal. On the right is the same
question, but with judgment in it. Same facts. Except now you can see what's current, what's
gone stale, what got quietly replaced. That gap, right there, is the whole game.

## S8 — The five pillars · "The micro-signals every claim carries."

So how does the right-hand side actually happen. Every claim carries five things. Who it's
about. When it was true. Who's allowed to see it. Whether it's still believed. And whether
it's even worth surfacing right now. All five of those are shipped. That's the raw material —
those are the signals.

## S9 — The trust compiler · "Five signals aren't a decision."

But five signals on their own still don't tell you what to do. They describe the claim, they
don't decide it. So something has to weigh them — and weigh them against things that aren't
even on the claim. How reliable the source is. Whether anything backs it up. Whether anything
contradicts it. Every time you've corrected it before. We call that the trust compiler, and its
whole job is to take everything the system knows about a fact and turn it into one call you can
actually act on.

## S10 — How trust gets decided · "The five signals compound into a band."

And the call it makes isn't a percentage. Nobody's gut hands them a seventy-three percent. It
hands you a band. This one you trust. This one you'd double-check. This one you'd ask about
before you act on it. And the nice part is that band travels as a kind of contract. A briefing
reads it, a WordPress page reads it, a tool over MCP reads it, and they all treat that fact the
same way.

## S11 — The part that learns · "Every correction makes the next answer better."

And none of it is frozen. The moment you fix something, or wave it off, or confirm it, that goes
right back in and re-scores the claim — and re-scores wherever it came from, too. So over time
it quietly stops being generic and starts being yours. †And that, to me, is the part I find
most interesting. It slowly learns the shape of how you think.

## S12 — The Intelligence Loop · "Signals in. Surfaces out. Feedback closes the loop."

Put all of that together, and what you've got is a loop. Sources come in. They become claims.
The compiler turns those into trust. The runtime does something useful with them. It shows up
on a surface. And your feedback closes the whole thing back to the top. One front door for all
of it, so it stays consistent no matter where it lands. That's the Intelligence Loop. That's
the thing we actually built this month.

## S13 — What we shipped · "One month. A working Intelligence Loop."

Quick sense of the scale. One month. Somewhere around nine hundred commits. The codebase more
than doubled. And the app was running the entire time we were building it. I won't sit on the
numbers, but I do want you to feel that this isn't a prototype. It's real, and it's running.

## S14 — What the old way would have cost · "40 · 42mo · $19M"

†And here's the one that still gets me a little. If you run this through the standard industry
estimators — team size, schedule, cost — a system this size comes back at around forty people,
three and a half years, and nineteen million dollars. This was one month. Me, and a fleet of
agents. †I'm not sure I've fully wrapped my head around that yet. But I'm pretty sure it's the
most important number on the slide.

## S15 — The unlock · "Great context is the thing everything else needs."

So why does any of this matter. Here's where I landed. Every good doc, every decent deck, every
analysis worth reading starts with good context. And trustworthy intelligence is that context —
it's the part you don't have to second-guess. Get that right, and everything downstream just
gets better. The trust at the bottom is what makes all the AI work you stack on top of it
actually worth doing.

## S16 — The value travels · "What you know is worth more when it moves."

And the thing about good context is that it doesn't want to stay on your laptop. The second it's
trustworthy, you want to hand it to people. A briefing turns into your team's shared picture. A
month of digging turns into a post your network actually reads. It moves onto a P2, onto your
own site, into the tools you're already in. The better the source, the more it's worth passing
on.

## S17 — The good problem · "The more you trust it, the more you make."

Which, of course, creates a good problem. The more you trust it, the more you lean on it — and
the more you lean on it, the more you make. And all of that has to live somewhere. It has to
stay organized. It has to stay findable. The better personal intelligence gets, the more that —
managing everything it produces — becomes the real problem to solve.

## S18 — Close · "Democratizing Personal Intelligence."

So that's the bet. We spent a month proving you can build personal intelligence you actually
trust. But the bigger idea is that it shouldn't only belong to the handful of people who can
build their own. Making it real, for everyone, on tools they already have — I think that's the
next thing WordPress is for. And that, really, is what this whole month was about.
