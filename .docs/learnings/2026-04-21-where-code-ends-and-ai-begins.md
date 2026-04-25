# Where code ends and AI begins

**2026-04-21. James Giroux. DailyOS.**

Early on in the project I had a phase in the pipeline that asked the LLM to produce JSON. Just a small structured output: extract a few fields from an email, return them as a valid JSON object, pass them downstream for storage.

Most of the time it worked. The rest of the time it didn't. A missing comma. A string truncated mid-quote because the model decided it was done. A field I'd defined as a number that came back wrapped in quotes. Once, a perfectly valid JSON document that described something entirely different from what I'd asked for.

I spent about a week wrapping the output in retry loops and repair heuristics. If the JSON failed to parse, try once more with a sterner prompt. If a field was missing, ask again. If the value was the wrong type, coerce it and log a warning. My test suite, which I'd thought was solid, started passing on the retries and hiding the underlying instability. I had a pipeline that mostly worked, which is the most dangerous kind of broken.

At some point I sat back and realised I was asking a probabilistic system to produce deterministic output, and then treating it as broken when it occasionally refused. The system wasn't broken. The design was.

## The rule I eventually wrote down

The decision I landed on felt obvious in retrospect, which is usually how it goes with the good ones. The LLM is good at producing natural language. It is bad, in a fundamental and un-improvable-by-retry way, at producing output that has to be structurally perfect. Ask it to do one and not the other.

So the rule became: **the LLM outputs what it is good at. Deterministic code produces what the rest of the system needs.**

In the original pipeline, that meant the LLM wrote markdown and a small Python function converted that markdown into validated, structurally-guaranteed JSON. The Python function was boring. It was also testable without invoking the model. It could fail, but it would fail in predictable ways, not stochastic ones.

[ADR-0006](../decisions/0006-determinism-boundary.md) is the short document I wrote at the time. It says about four things, and each one turns out to matter.

## The generalization nobody warned me about

The thing I didn't see coming is that once you name this boundary, it shows up everywhere. What started as a small rule about JSON output ended up being the architectural principle underneath most of the substrate.

When the LLM generates a claim about an entity (Alice is the champion at Acme), the claim itself is probabilistic. The LLM might be right, might be wrong, might be hallucinating. I cannot put that directly into a database and trust it. What I can do is treat the LLM's output as a *candidate* and let deterministic code enforce policy on the candidate. Check whether the user tombstoned this assertion before. Check whether it contradicts a higher-trust claim. Check the provenance envelope is well-formed. If all the gates pass, the candidate becomes a committed claim. If any gate fails, the candidate is rejected or queued for review.

The same pattern repeats at every layer:

- The LLM drafts a meeting briefing. A deterministic evaluator scores it against a rubric before a human sees it.
- The LLM suggests a relationship type between two entities. A deterministic policy registry decides whether that signal propagates, how far, and with what coalescing.
- The LLM produces a summary. A deterministic envelope records which source claims contributed which fragments and at what trust level.
- The LLM extracts a date from a prose message. A deterministic parser validates that it's a real date that makes sense in context.

In each case the shape is the same. Probabilistic work produces candidates. Deterministic work applies policy. The candidates never commit directly to state. The policy never guesses.

## The contract, named

I think this is the thing worth saying out loud, because it's not unique to us and I don't see it stated clearly anywhere in the current AI-harness conversation.

**Probabilistic subsystems are allowed to fail without consequence, because deterministic subsystems catch the failures before they become facts.**

That's the contract. Without it, every probabilistic failure is a bug that reaches the user. With it, probabilistic failure is expected, noticed, logged, often recovered from automatically, and never silently accepted as truth.

The harness around the model is mostly this contract, written in code, at every boundary where probabilistic meets deterministic. The number of boundaries is much larger than you'd think before you build one.

## What this isn't, to be clear

It's not a rejection of the model. The probabilistic layer is doing the work that nothing else can do: reading prose, generating natural language, pattern-matching across messy input, producing candidate hypotheses about the world. Nothing deterministic replaces that.

It's also not a retreat to classical software. The system is still an AI-native system. The deterministic layer is the scaffolding, not the point. Without the probabilistic layer, there's nothing to scaffold.

What it is, I think, is the answer to the question "why is this AI assistant trustworthy and that one isn't." The untrusted ones skipped the contract. They let probabilistic output become truth directly, and they paper over the failures when users notice. The trusted ones (or the few of them, so far) have the contract, whether they've named it or not.

## What others are doing with it

Andrej Karpathy's LLM Wiki gist dances around this without stating it. His three-layer architecture (raw / wiki / schema) is a version of the same idea: the wiki layer is human-curated schema-bound, the raw layer is LLM-generated and freer. The tension between the two layers *is* the contract. The gist doesn't quite name it as a rule, and the comments are full of people asking about the places where the two layers rub against each other (contradictions, drift, provenance). Those are all "where does the contract actually go" questions.

Garry Tan's GBrain is interesting because it makes the pragmatic choice to let more slip through the deterministic gate when the user is a single power user (himself) with context to catch errors. That's a legitimate choice for a userbase of one. It stops being legitimate the moment you have a second user.

OpenClaw and Hermes both operate with agent teams where the contract lives between agents. One agent proposes, another (or a rule-based checker) gates. Same pattern, different distribution.

## What's still open for me

Two things I don't have clean answers for.

The first: there's a middle zone where the deterministic code needs LLM output to decide what to do, but has to stay deterministic in its own decisions. The runtime evaluator pass is the clearest case. It uses an LLM to score an LLM's output against a rubric, then uses the score to decide whether to commit. The score is probabilistic; the decision based on the score is deterministic. Is this really the contract, or am I just pushing the probabilistic problem one step down? I think it's fine in practice; I haven't fully convinced myself theoretically.

The second: how much of the deterministic code can be safely generated by the LLM itself? Today my policy registry, parsers, and validators are hand-written. The LLM is plenty capable of writing them. If I let it, I've added a probabilistic step in generating the deterministic layer, which feels like it breaks the whole contract. But if I never let it, I spend a lot of human time on code that isn't the creative part of the system. There's probably a good answer involving tests and review gates; I haven't written it yet.

## The short version

If you're building anything that puts an LLM near user-facing state, draw the line between what the LLM produces and what the system commits. Make the LLM output into candidates. Let deterministic code apply policy. The harness is the contract between the two. Most of what makes an AI assistant trustworthy or untrustworthy lives in whether that contract is strong or weak.

That rule took me about a week of debugging JSON to learn and a few months to generalize. Offered here in case it saves someone a week.
