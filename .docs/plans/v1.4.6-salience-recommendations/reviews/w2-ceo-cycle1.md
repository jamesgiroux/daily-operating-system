# W2 L0 CEO/product review — cycle 1

Verdict: BLOCK

Findings:

- Thresholds, budgets, and cooldowns were named as constants without product rationale or expected eval coverage.
- Critical bypass could create unbounded primary-surface noise without a storm guard.
- Notable budget keyed only by `surface_key` could multiply the same user's primary volume across surfaces.
- Background items need an Activity Log cap/grouping rule.
- Suppressed feedback should only be overridden by critical/urgent candidates when material new evidence or subject state changed.
- Trigger classes and their defaults need to be explicit product policy, not implementation placeholders.
- Tests need to prove noisy fixtures stay quiet, low-trust urgent items go to review/defer, suppressed items require new evidence, and primary daily volume stays in budget.

Cycle 2 packet changes: add policy rationale table, shared primary budget, Critical storm guard, Activity Log cap, material-evidence override rule, trigger-class defaults, and calibration tests.
