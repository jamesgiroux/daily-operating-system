# RSM Final Update - Voiceover Script

Performable voiceover for the showcase video. Talking-head plus slides cutting back and
forth; this VO carries the story, the slides punctuate it. Maps 1:1 to the deck at
`2026-05-25-dailyos-work-record.html` (18 slides).

**Delivery.** This is written to be *said*, not read. Short, lumpy, a bit unfinished in
places, the way you actually talk. Don't smooth it out when you perform it. If a line wants a
"like" or an "um" or a pause, let it. Roughly 6 to 7 minutes unhurried.

**Authenticity pass.** Second draft, run for AI tells. Stripped the balanced em-dash asides,
broke up the tidy three-item lists, varied the sentence length hard, and cut the recurring
framing crutches. Read it out loud once. Anywhere it still feels like a script instead of you,
mark it and we'll rough it up more.

**Yours to make real.** Lines marked with `†` reach for a personal reaction. Keep them only if
they're true. Say them in your own words or cut them. Don't act a feeling you don't have.

---

## S1 - Cover - "Democratizing Personal Intelligence"

Okay. So this is the wrap on Radical Speed Month. I know the title's a bit of a stretch. Stick
with me though, I think it'll land by the end. Mostly I just want to show you what we built,
and honestly why I think it matters more than the app does.

## S2 - The pain - "Is this a fact, or did the AI just make it up?"

Let me start with something I think we've all felt. You ask an AI something, and it gives you
this answer that sounds great. Totally confident. And you have no idea if it's actually true.
So what do you do. You go check it yourself. Which is the exact thing the AI was supposed to
save you from. And the more I poked at that, the more I started thinking it's not really a model
problem. It's an architecture problem. The model says something, and nothing around it has any
idea what it just said.

## S3 - The trust question - "Can you define trust in code?"

So the thing I kept coming back to all month was trust. Not accuracy. Trust. Can you actually
build for that. And trust is a weird one, because it's not really a fact. It's more of a
feeling. You kind of just land on it without thinking it all the way through. And I wasn't sure
you could put that into code at all. So that's what the month turned into. Me trying to figure
out if you could.

## S4 - How humans decide trust - "It's really micro-signals compounding."

So before I touched any code, I went the other way and asked how people do it. Think about
meeting someone for the first time. You don't decide to trust them. You just kind of know. Your
eyes and ears are picking up all these little things, their face, their hands, the way they're
talking, and it adds up to a read before you've even finished saying hello. And I think that's
the whole trick right there. It's never one thing. It's a bunch of little signals stacking up
at once.

## S5 - The same logic, in code - "Code has no senses. It has memory."

Problem is, code can't do any of that. It doesn't have eyes. It doesn't have ears. All it's got
is memory, the stuff it can pull up later. So if you want it to do what your gut does, those
signals can't live in a glance or a tone of voice, because it can't see those. They've got to
be in the memory itself. Every fact has to carry its own evidence around with it. We started
calling that, a fact that drags its own signals along with it, a claim. And that little thing is
what the whole rest of this is built on.

## S6 - The blind spot - "Smart isn't the same as trustworthy."

Now, we're definitely not the only ones who think memory's the key here. Karpathy's got a wiki
his AI keeps up. Garry Tan's got a brain his reads before it answers you. The compound
engineering folks are doing it too. Everybody's racing to give these things a better memory. And
I think they're right, honestly, memory matters. But here's the part I can't stop chewing on.
It's all memory. Almost none of it is judgment. Like, knowing what to actually trust, or what to
flag, or what to just leave out. So the memory gets smarter and smarter, and it's no more
trustworthy than it was.

## S7 - Flat vs weighted memory - "Every fact, the same weight."

Let me just show you what I mean. So on the left, that's memory the way it works right now. You
ask about an account and it dumps everything on you at the same volume. The sponsor, the
renewal, some plan from back in February. It's all flat. It all weighs the same. On the right is
the exact same question, except now there's judgment in it. Same facts. But now you can see
what's current, what's gone old, what got replaced and nobody told you. And that difference,
right there, that's the whole thing.

## S8 - The five pillars - "The micro-signals every claim carries."

So how does that right side actually happen. Every claim carries five things with it. Who it's
about. When it was actually true. Who's allowed to see it. Whether we still believe it. And
whether it's even worth bringing up right now. All five of those are built. They're shipped.
That's the raw material. Those are the signals.

## S9 - The trust compiler - "Five signals aren't a decision."

But five signals on their own still don't tell you what to do with it. They describe the claim.
They don't decide anything. So something's got to weigh them. And not just them, also a bunch of
stuff that isn't even on the claim. How good the source is. Whether anything else backs it up.
Whether anything contradicts it. Every time you've gone in and corrected it before. We call that
thing the trust compiler. And all it really does is take everything we know about a fact and
boil it down to one call you can actually act on.

## S10 - How trust gets decided - "The five signals compound into a band."

And the call it makes isn't a percentage. Your gut never hands you a seventy-three percent,
right. It hands you a band. This one, you're good, just use it. This one, eh, double-check it
first. This one, go ask somebody before you do anything. And the part I like is that band is
basically a contract. The briefing reads it the same way a WordPress page reads it the same way
some tool over MCP reads it. They all treat that fact exactly the same.

## S11 - The part that learns - "Every correction makes the next answer better."

And none of this is locked in. The second you fix something, or you toss it, or you go yeah,
that one's right, that goes back in and re-scores the claim. And it re-scores wherever it came
from too. So over time it slowly stops being this generic thing and starts being yours.
†And honestly, that's the part I care about most. It kind of learns how you think.

## S12 - The Intelligence Loop - "Signals in. Surfaces out. Feedback closes the loop."

So you put all of that together and what you've got is a loop. Sources come in. They turn into
claims. The compiler turns the claims into trust. The runtime does something useful with them.
It shows up somewhere you can see it. And then your feedback closes the whole thing right back to
the top. One front door for all of it, so it behaves the same no matter where it pops up. That's
the Intelligence Loop. That's the thing we actually built.

## S13 - What we shipped - "One month. A working Intelligence Loop."

Quick sense of scale here. One month. Something like nine hundred commits. The codebase more than
doubled. And the whole time we were building it, the app was actually running. I'm not gonna sit
here and read you numbers. I just want you to feel that this isn't some prototype. It's real,
and it's running right now.

## S14 - What the old way would have cost - "40 - 42mo - $19M"

†Okay, this is the one that kind of messes with me. If you take what we built and run it through
the normal industry estimators, you know, team size, how long it'd take, what it'd cost, a
system this size comes back at something like forty people, three and a half years, nineteen
million dollars. This was one month. It was me and a bunch of agents. †I don't think I've really
processed that one yet, to be honest. But I'm pretty sure it's the most important number up
there.

## S15 - The unlock - "Great context is the thing everything else needs."

So why does any of this actually matter. It pretty much all comes down to context. Every good
doc, every halfway decent deck, anything worth reading, it starts with good context. And trustworthy intelligence,
that's the context. It's the part you don't have to keep second-guessing. You get that part
right, and everything you build on top of it gets better. The trust underneath is what makes all
the AI stuff you pile on top of it worth doing in the first place.

## S16 - The value travels - "What you know is worth more when it moves."

And good context, it doesn't want to just sit on your laptop. The second you trust it, you want
to give it to people. That briefing becomes the thing your whole team is looking at. A month of
research turns into a post people actually read. It ends up on a P2, on your own site, in the
tools you're already in all day. The better the source is, the more you want to pass it around.

## S17 - The good problem - "The more you trust it, the more you make."

Which kind of creates a good problem for us. The more you trust it, the more you use it. And the
more you use it, the more you make with it. And all of that has to go somewhere. It's got to stay
organized. You've got to be able to find it again later. So the better this stuff gets, the more
the real problem becomes just, managing all of it. All the stuff it's putting out.

## S18 - Close - "Democratizing Personal Intelligence."

So that's the bet. We spent a month showing you can actually build personal intelligence you
trust. But the bigger thing is, it shouldn't only belong to the few people who can go build their
own. Making it real for everyone, on stuff they already have, I think that might be the next
thing WordPress is for. And that's really what this whole month was about.
