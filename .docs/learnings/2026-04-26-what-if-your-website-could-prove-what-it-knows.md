# Can Your Website Show Its Work?

*AI Claims Need Receipts*

**2026-04-26. James Giroux. Thought experiment.**

I was at the Forrester B2B Summit this week. The headline message was the GTM Singularity: agents are reshaping how we think about a website visitor, and therefore how we think about marketing.

The framing holds up. Agents are the new visitor. They show up with intent, context from somewhere else, and a job to finish. They click, read, and decide on different terms than the humans we've spent decades designing for. The marketing motion is going to bend around that, and it should.

Are you seeing this too? At WordPress VIP, the customers I work with are seeing a huge increase in agent and bot traffic, and it's challenging every part of their strategy. We're all asking the same question: if the website visitor is changing this profoundly, what about the thing the visitor is reading?

Websites are kind of my thing, and I still think the website is an organization's superpower, more so in the age of AI. It's where a company says what it sells, what it stands for, and what it knows. It's the canonical surface, and everything downstream gets shaped by what's on it: search results, AI overviews, agent answers, sales conversations.

## The thirty-second gut read

When you and I meet for the first time, something happens in the first thirty seconds we don't really have a word for. You read my posture. You catch a micro-expression when I talk about my last job. You notice whether my eyes meet yours when I answer a hard question.

You compress all of it into a gut read, and you start operating from it before either of us has said anything definitive. We sometimes call it a vibe. Vibe undersells what's happening. It's a thousand small signals collapsing into a single posture: how much of yourself to extend, and on what terms.

We do this with brands too. We do it with websites. Pull up a company's policy page and almost instantly we know whether it feels current, hand-written by a lawyer, copy-pasted from a template, or quietly out of date. We don't articulate the signals. They land.

Until recently, websites only had to deal with one kind of reader: humans, who arrive with that gut-read apparatus already running. The signals didn't have to be on the page. The reader brought them.

Agents don't have any of that. When an agent reads my return policy, it sees text. It doesn't see the page's history. It doesn't see the governance around it. It has no idea I keep the policy under strict review, or that the support macro down the hall paraphrases it for tier-one tickets. None of the connective tissue we use to form a gut read is visible to it.

Which leaves the agent with two bad options. Over-trust the whole page and treat every string as canonical, or under-trust it and refuse to act.

## The same gap, from a different angle

Neither option is what we want. The place I started seeing this from a different angle was inside DailyOS, the personal operating system I [wrote about recently](https://jamesgiroux.ca/zero-guilt-design-releasing-my-daily-operating-system/). The thing has morphed since that post. What started as a daily readout of your day is becoming something more like a persistent personal intelligence platform.

Most of the work since has been about strengthening the intelligence layer underneath. More signals, longer memory, sharper synthesis when the surrounding context actually lands. All in service of a /today brief that walks me into the day with a real read on what's moving in my accounts.

The further that work has gone, the more I've run into the same wall everyone working with AI eventually hits. The further you get from a source, the less *true* the output becomes. Layers of summarization, inference, and synthesis erode the connection back to evidence. Even when the language stays confident.

A second wall sits next to the first. Temporal flatness. What was true last quarter shows up in today's brief with the same weight and the same authority as something true this morning. The system has the data. It just doesn't have a sense of the *age* of what it's saying.

Both problems land somewhere specific for me. The whole point of the /today brief is to walk me into a meeting actually prepared. When the prep is partly stale, partly inferred, and partly grounded, all rendered in one block of confident prose, it's not as dependable as I want it to be. It's the thing AI does that I most need it not to do.

So I've been wrestling. Contradictory sources. Older sources outranking newer ones. Source verification. Ways to express confidence at the level of an individual statement, instead of the brief as a whole.

## What AI actually needs from content

The wrestling has been pushing me toward a more basic question: what does AI actually need from the content we give it?

What an intelligence system needs, underneath everything, is something it can act on. A statement. A fact. A directive. An attribute. Something definite enough to do something with next.

That actionable unit doesn't have a fixed size. Sometimes it's a whole page: a policy document, a product spec, a customer profile. Sometimes it's a single line tucked inside a paragraph: a return window, an eligibility criterion, a deal stage. The unit is functional, not structural. Whatever shape carries the meaning.

I've been calling it a claim.

A claim, the way I'm using the word, is content that asks someone, or asks an agent, to believe it or act on it.

*"This meeting needs prep."*
*"This project is at risk."*
*"You owe this person a reply."*
*"Our return policy is 30 days."*

Different jobs, but the same pressure: the moment a claim lands in front of someone, it can change what they do next.

An output usually contains many claims. A /today brief might tell me a project is blocked, a customer is at risk, and a follow-up is overdue, all rendered in one block of confident prose. Some are rock solid. Others are inferred from soft signals. Others are stale from last quarter. The output looks uniform. The claims inside it have wildly different relationships to evidence.

Once I started thinking at the claim level, two questions started organizing everything. *Where did this come from?* And *is it safe to act on for what I'm about to do with it?*

Those are the same two questions an agent reading my return policy needs to answer.

## What I've ended up tracking

Building toward those two questions in real code surfaced five things to track. They emerged in roughly the order I needed them.

The first one was the obvious one. Provenance. If the system tells me a project is at risk, I want to walk back to the missed milestone and the meeting note that fed it. If I can't trace the path, the claim isn't trustworthy yet, no matter how well the language reads. Polished prose with nothing underneath is the failure mode I'm trying hardest to avoid.

Provenance got me partway. Then I started running into stale claims. Different statements decay at different rates. *James prefers concise briefings* can sit for weeks without losing value. *This deal is moving this week* can be wrong by Wednesday. The freshness logic has to know what *kind* of claim it's looking at, not just whether there's a timestamp on it. A timestamp on the record is easy. Knowing how much the underlying truth has decayed is harder.

Next was the single-source problem. A claim built on one signal can be fine for a low-stakes summary. A recommendation usually needs more than one leg under it. That's corroboration. And the threshold isn't fixed. What counts as enough depends on what the claim is being used for.

The trickier flip side is contradiction. When something newer or stronger disagrees with what the system already believes, does the system know to lower confidence, or does it dig in? In DailyOS, this is where I've had to be most careful. Confident-sounding stale claims, dressed up in fresh language, are the most expensive kind of failure I've found.

The last one I'm still actively wrestling with. Consequence. The same claim can be safe for one action and unsafe for another. *This account is at risk* might be the right thing to surface in a /today brief and the wrong thing to mention in a customer-facing email. The claim hasn't changed. The consequence has.

None of these are exotic. They're what a careful person does intuitively when reading something and deciding what to do next. The work has been making each of them legible to the system, and to me.

## A conversation from the DAM side

Earlier in the week, I was talking with a Forrester analyst. They didn't know any of the thinking I'd been doing on claims and trust, but they were circling the same problem from the angle of digital asset management. Marketing operations teams have been wrestling with it for years, long before agents.

A modern digital asset management system encodes a lot of this. A DAM tracks who uploaded an asset and when. It knows when it expires, whether legal cleared it, which markets and channels it can run in, what the review cadence is.

Marketers don't think of those as "trust signals." They're just how grown-up content operations work. The asset and its governance travel together. A creative drops into a campaign tool already knowing where it's allowed to go, until when, and on whose authority.

What the analyst and I had both arrived at, from different sides, is that the discipline hasn't fully translated to claims. The text on a policy page gets treated as a static string. But in the agent era, that string is the source of dozens of claims downstream systems will act on, with almost no metadata to tell an agent (or the next person updating the page) where the organization stands behind what it says.

## What's been tried for claims

I want to be careful here, because parts of the discipline have translated. Schema.org's ClaimReview markup, introduced in 2015 by Google and Duke Reporters' Lab, makes fact-checking articles machine-readable: the claim being reviewed, who reviewed it, when, the verdict, and a URL back to the source. Major platforms consume it (Google, Bing, Facebook, YouTube). It's the closest existing analog for what I'm describing, and it predates most of the current AI conversation.

I think AI's progress makes ClaimReview more valuable, not less. The reasons might differ from what its designers had in mind in 2015, but the underlying work, making claims structurally legible to machines, is exactly the substrate the agent era needs.

Two things about ClaimReview are worth sitting with. Adoption has been spotty. Research from 2021 found that fewer than half of fact-checkers worldwide had implemented the markup, and the picture in under-represented languages is worse. The spec exists; the practice hasn't fully caught up. It's also narrowly scoped: it's for fact-check verdicts, where one party reviews another party's claim. It doesn't handle the more general case where a site makes a claim about itself.

There's a distinction worth pulling apart. ClaimReview is about a third party rendering a verdict on someone else's statement. What I keep arriving at is a step before that: a way for a site to publish its most authoritative version of what *it* says about itself, with the provenance and limits attached, so an agent or person can weigh it in context.

Take Pepsi and Coca-Cola. Both will make claims about themselves that an agent might end up reading. They're competitors and they position against each other. A receipt isn't trying to determine which one is universally right. It's trying to make sure that when an agent pulls from a Pepsi page, what it pulls is what Pepsi most authoritatively says about Pepsi. Not a stale CRM note, not a paraphrased support macro, not a marketing draft from three quarters ago. The consumer (the agent, or the human behind it) can then weigh that statement against the task they're working on.

So this isn't truth-finding. It's authority-locating. ClaimReview sits next to that work. The shape I keep circling sits in front of it: helping a site say, in a form an agent can read, here is our most current version, here is who stands behind it, here is what it's safe to do with it.

There's a broader landscape too. Content authenticity for media. Structured-data conventions for products and events. Work on credentials and provenance. People are circling this from a lot of angles, which is encouraging. What I haven't found is a primitive that does the general case: any site, any claim, with a receipt that knows the limits on its use.

## Claims with receipts

What I keep arriving at, when I try to imagine this for the content sites publish, is a receipt attached to each claim. An authority packet, if you want a more technical name. I'm not married to the label. The shape matters more.

My first instinct was a score. A number (0.82, 0.91) that summarizes how confident the system is in a claim. It's the obvious move. It gives you an immediate visual reference. Green, yellow, red. Safe, suspect, no.

But the more I sit with it, the less a score does for me. It doesn't tell me what's underneath, or what would change it, or whether the claim is safe for what I'm about to do with it. The score compresses the trail away, exactly when I need it.

I'm still open to a score as a glance, a small visual companion to a richer object. The load-bearing thing, though, has to be the trail.

So instead of a site saying:

> Our return policy is 30 days.

It says something more like:

> Our return policy is 30 days.
> Source: canonical policy page, /legal/returns.
> Last reviewed: 2026-04-12, legal.
> Corroboration: matches commerce setting `return_window_days = 30`.
> Contradictions: none known.
> Safe uses: quote to customer, summarize in support contexts.
> Restricted uses: exceptions require human review.

The first version is what websites look like today. A human can work with it. They bring the gut read.

The second is the agent equivalent of those micro-expressions. We can't make models develop a gut read for our content. We can hand them the structured form of what our gut read is already running on. Call it inspectable authority if you want a name for it.

## Why WordPress, of all places

I want to be careful with this next leap. I work at WordPress VIP, and I'm aware of how easy it is to take a thought experiment and pretend it's a roadmap. I'm still working through how this would actually fit: WordPress core, plugins, standards, hosting. Questions for another post.

The conceptual fit is hard to ignore, though. Authority on a WordPress site is already distributed. Part of it lives on the page. Part lives in plugins. Part lives in the connected systems: commerce, CRM, analytics, customer support. That distribution is one of the reasons WordPress works at scale. It's also exactly the surface where agents need more context than they currently have.

When an agent arrives at a site today, it sees the surface and not much else. A plugin exposes an action without signaling whether it's safe to take. A model reads a page that never marks itself canonical. A commerce setting encodes a number with no record of how it was reviewed. A connected analytics source explains a traffic dip without showing how confident it is.

Humans reconcile that mess every day, because we bring organizational context to it. We know the policy page outranks the support macro. We know the legal review happened in the workflow plugin. We know which dashboard the CFO actually trusts and which one is decorative. Agents don't have any of that, unless the site can express it.

The question I keep arriving at is whether WordPress could give sites a shared way to describe their content and the receipts that should travel with it. Not new metadata about the page. Metadata about the *statement* the page is making, and how the site stands behind it.

If a commerce plugin emits a claim about returns, a legal page could mark itself canonical for that claim. A workflow plugin could attach the review state. A support setting could mark which downstream uses are allowed and which need a human. Each part of the WordPress stack knows part of the trust story. What's missing is a shared way to express it.

## What WordPress already has in place

A few specific things about how WordPress is built make this feel less like a leap and more like a natural extension.

Start with the plugin ecosystem. WordPress has spent twenty years cultivating an ecosystem of third-party plugins that each own a piece of a site's authority. Yoast owns SEO metadata. Advanced Custom Fields owns structured fields. WooCommerce owns commerce data. Each already speaks a shared language to the rest of WordPress: they register hooks, expose REST endpoints, declare capabilities, attach metadata to posts. The infrastructure for plugins to publish information about themselves is mature.

Adding claim receipts to that vocabulary is a short walk. A plugin already declares what it does. The next move is for it to declare what it knows about the claims it emits: the source, the review state, the uses it supports.

Then there's Gutenberg. The block-based content model gave WordPress a finer-grained way to think about a page. A block isn't just rendered HTML. It knows its type, its attributes, where it lives in the post hierarchy, how it gets rendered. Attaching a receipt to a block is a much smaller architectural lift than attaching one to free-floating markup. The container is already there.

There's also the editorial DNA. WordPress was built around publishing, with content as a first-class object: author, revisions, status, publication moment. A claim in WordPress isn't a data row. It's an editorial unit, written by someone, approved at some point, owned by a workflow. That orientation matters when we're talking about receipts. The claim already has the right shape inside the system; the question is how to expose it legibly to other systems.

And then there's scale. WordPress runs roughly forty percent of the web. That number matters because this only works if enough sites speak the same language. A standard born in WordPress doesn't stay in WordPress. Block markup and the REST API both spread the same way. If WordPress decides claim receipts are a thing it expresses, a meaningful fraction of the web speaks that language by default.

None of this means WordPress is the only place this could live. Headless CMSes will solve a version. Webflow will solve a version. Drupal probably has half the parts already. But the combination of distributed authority, a plugin ecosystem, structured content, editorial DNA, and reach makes WordPress the place where this could plausibly become a primitive instead of a feature.

## The harder version of the problem

Even with the WordPress shape I've sketched, there's a harder version of this problem. The same Forrester conversation turned there too. Inside one site, with one team owning both the claim and the receipt, you can imagine this working. What gets harder is keeping the receipt intact when the claim moves through an enterprise stack.

Picture a normal martech path. A website publishes a claim: a policy, or a signal about visitor intent. A CRM enriches an account record from it. A CDP turns that record into a segment. A marketing automation platform uses the segment to personalize a journey. A support assistant later reads the downstream fields and writes an answer back to the customer.

By the time the claim has traveled through all that, the answer might still sound right. It might even *be* right. But what is it actually grounded in?

Two failure modes show up. The first is enrichment degradation. Each system reshapes the claim for its own job, and with every hop, the context that made it trustworthy gets thinner. The reshaping is useful inside one tool. It becomes inscrutable everywhere else.

The second is authority drift. The derivative becomes more reachable than the source, and the organization slowly starts treating the derivative as canonical. The CRM note outranks the policy page. The AI summary becomes more authoritative than the evidence behind it. The downstream tool is right there, and the original is two clicks and a permission boundary away.

Nobody has to make a wrong call for this to happen. It's the natural shape of connected systems: every tool optimizes for its own workflow, and every integration moves a slightly smaller version of the truth.

For receipts to mean anything across an enterprise, they have to inherit at each hop, not reset. The CRM record has to know it's holding an enrichment, not a claim. The CDP segment has to know it's a derivative two layers down. The support assistant has to be able to reach back through the chain when the stakes warrant it. It's the lineage problem DAMs already solved for assets, applied at a different unit and scale.

## A sketch, not a spec

I don't have a spec for this yet. What I have is a sketch.

The smallest useful version probably isn't ambitious. A handful of high-stakes surfaces: policy pages, pricing, editorial canon, commerce settings, plugin actions that touch the customer. A standard shape for a claim receipt. A way for plugins to participate. A surface for the site owner to see and edit what their site is saying about its own authority.

The right interface is probably invisible most of the time. Receipts ride along with the content. They surface to a human only when a system is about to cross a trust boundary: about to send something externally, act on it, quote it to a customer.

The traps are real. A receipt can pretend to be more precise than the underlying reality. It can go stale and start lying. A bad actor can dress up a garbage claim in a clean receipt and make it look canonical. None of those are reasons to skip the primitive. They're reasons to design it carefully, and to make the receipt itself inspectable.

I keep coming back to the same sentence, every time I open DailyOS or look at a customer's WordPress estate.

*What does this site know, and what is safe to do with that belief?*

I don't have a clean answer. I'm not even sure claim receipts is the right name. Authority packets, inspectable authority, something else entirely. I think the thing has to exist, in some form.

If you've been thinking about this from another angle, I'd love to hear it. This is nascent for me, still fleshing it out. I'd rather post the half-formed version and have the conversation than wait for tidy.

---

## Publishing notes

**TL;DR:** AI agents are becoming a new class of website visitor, but they read content without the human gut checks that tell us whether something is current, canonical, approved, or safe to act on. The post argues for claim-level "receipts" so websites can expose not just what they say, but how confidently they stand behind it.

**Meta description:** AI agents are becoming website visitors, but they can't tell which claims are current, authoritative, or safe to act on. This essay explores why websites need claim-level receipts: provenance, freshness, corroboration, contradiction, and consequence.

**Social blurb:** AI agents are reading your website, but they have no gut read. They can't tell what's canonical, stale, approved, contradicted, or safe to act on.

I wrote about why the agent era needs claim-level receipts: a way for websites to show their work.
