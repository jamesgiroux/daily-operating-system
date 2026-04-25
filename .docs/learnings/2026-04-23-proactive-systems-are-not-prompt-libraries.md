# Proactive systems are not prompt libraries

**Status:** stub. Drafting TBD.

## Synopsis

A library of agents, skills, prompts, or slash commands can be powerful. It can also leave the core burden untouched: the user still has to know what to ask, when to ask it, and what context to include. DailyOS is pursuing a different shape. This entry is about the difference between promptable capability and proactive knowing.

## Outline

- The lived moment: noticing that even very capable AI setups still depended on me to frame the question before they could help.
- The distinction: prompt libraries wait; proactive systems notice, prepare, and surface.
- Why it matters: the first half-hour of the day is usually not a reasoning problem but a reconstruction problem.
- Architectural implications: schedules, event triggers, invalidation, refresh policies, dossier maintenance, and briefing generation.
- Why this connects back to zero-guilt design: good software should remove recurring reconstruction work, not create a more impressive way to do it manually.
- What's still open: where proactive behavior feels magical, and where it starts to feel presumptuous or noisy.

## Related ADRs

- ADR-0007 (daily briefing philosophy)
- ADR-0080 (signal-based freshness and health scoring)
- ADR-0081 (feedback, decay, and learning loop)
- ADR-0097 (event-driven intelligence service)
