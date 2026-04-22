# Trust as a visible band, not a hidden number

**Status:** stub. Drafting TBD.

## Synopsis

Under the hood, every claim compiles a trust score from six factors. Surfaced to the user, the score becomes one of three labels: trust this, be careful, verify first. This entry is about why showing the raw number was worse than showing the band, and about the surprising amount of design work that went into three labels.

## Outline

- The lived moment: shipping a confidence percentage next to every claim and watching every user either ignore it or squint at it.
- The wrong assumption: "people want more information." People want less information, better chosen.
- The six factors and what each one costs computationally. Why six, not four, not ten.
- Compiling score to band: the thresholds, why they're not evenly spaced, why they moved twice.
- UX discipline: never surface "be careful" as alarm, never surface "trust this" as reassurance when it's just absence of contradiction.
- What's still open: how bands compose when a summary draws from claims across all three bands simultaneously.

## Related ADRs

- ADR-0114 (scoring unification)
- ADR-0110 (evaluation harness)
- DOS-5 (trust compiler implementation)
