# Glean Prompt: Health & Outlook Enrichment Signals

**Purpose:** Supplemental intelligence pull for the Health & Outlook tab. Extracts the 5 high-leverage gaps identified during the UX consultation — signals we're NOT currently getting from the base Glean enrichment.

**Target:** Drop into Glean chat as a one-shot test. Verify JSON parseability and signal quality before integrating into the PTY enrichment pipeline.

**Test account:** Globex Holdings (or any account name — swap `{ACCOUNT_NAME}`).

---

## The prompt (paste into Glean)

```
You are a customer success intelligence system. For the customer account "Globex Holdings", search ALL available data sources (Salesforce, Zendesk, Gong, Slack, internal documents, LinkedIn data if indexed, org directory, Google Workspace, Notion/Confluence if configured) and extract HIGH-LEVERAGE leading signals that are often missed by standard enrichment.

Focus on EARLY WARNING signals, TRENDS, and DIVERGENCES — not static state. We already have the base account intelligence (ARR, renewal date, stakeholder list, support tickets, recent wins). Do NOT duplicate that. This pull is specifically for the signals below.

## Required Output Format

Respond with a SINGLE JSON object. No prose, no markdown fences, no commentary before or after. Your entire response must be parseable by JSON.parse(). Begin with { and end with }. Nothing else.

The JSON object must have these fields. Omit any field you have no data for — do not fabricate. Return `null` for scalar fields with no data and `[]` for list fields with no data.

```json
{
  "champion_risk": {
    "champion_name": "full name of current champion, or null",
    "at_risk": true,
    "risk_level": "low|moderate|high",
    "risk_evidence": [
      "specific behavioral signal: dated, sourced. e.g. 'Email response time slowed from avg 4h to 18h over last 30d (source: Gmail)'",
      "title change, LinkedIn activity, tenure anomaly, sentiment shift, etc."
    ],
    "tenure_signal": "e.g. '3.2 years at company, recently promoted 6mo ago — promotion-to-departure risk window'",
    "recent_role_change": "description or null",
    "email_sentiment_trend_30d": "warming|stable|cooling",
    "email_response_time_trend": "faster|stable|slower|unknown",
    "backup_champion_candidates": [
      { "name": "full name", "role": "title", "why": "signal that makes them a backup candidate", "engagement_level": "high|medium|low" }
    ]
  },

  "product_usage_trend": {
    "overall_trend_30d": "growing|stable|declining|unknown",
    "overall_trend_90d": "growing|stable|declining|unknown",
    "features": [
      {
        "name": "feature/product name",
        "adoption_status": "active|growing|stable|declining|dormant",
        "active_users_estimate": "number or range if known, else null",
        "usage_trend_30d": "growing|stable|declining|unknown",
        "evidence": "source + signal. e.g. 'Parse.ly dashboard: 3 of 5 licensed seats inactive 30d (source: Salesforce opportunity notes)'"
      }
    ],
    "underutilized_features": [
      { "name": "feature", "licensed_but_unused_days": 60, "coaching_opportunity": "short note" }
    ],
    "highly_sticky_features": [
      { "name": "feature", "why_sticky": "embedded in workflow / compliance-critical / power-user pattern" }
    ],
    "summary": "1-2 sentence rollup on usage health"
  },

  "channel_sentiment": {
    "email": { "sentiment": "positive|neutral|mixed|negative", "trend_30d": "warming|stable|cooling", "evidence": "dated signal" },
    "meetings": { "sentiment": "positive|neutral|mixed|negative", "trend_30d": "warming|stable|cooling", "evidence": "dated signal, ideally with Gong sentiment" },
    "support_tickets": { "sentiment": "positive|neutral|mixed|negative|frustrated", "trend_30d": "warming|stable|cooling", "evidence": "Zendesk signal, recent ticket tone" },
    "slack": { "sentiment": "positive|neutral|mixed|negative", "trend_30d": "warming|stable|cooling", "evidence": "shared channel tone if applicable" },
    "divergence_detected": true,
    "divergence_summary": "e.g. 'Meetings positive / tickets frustrated — customer performing happiness in person while escalating via support'"
  },

  "transcript_extraction": {
    "churn_adjacent_questions": [
      {
        "question": "verbatim or near-verbatim question the customer asked",
        "speaker": "who asked",
        "date": "YYYY-MM-DD",
        "source": "Gong call / email / Slack",
        "risk_signal": "what this suggests"
      }
    ],
    "expansion_adjacent_questions": [
      {
        "question": "verbatim question",
        "speaker": "who asked",
        "date": "YYYY-MM-DD",
        "source": "source",
        "opportunity_signal": "what this suggests",
        "estimated_arr_upside": "number or range if inferable, else null"
      }
    ],
    "competitor_benchmarks": [
      {
        "competitor": "competitor name",
        "context": "exact context of mention — evaluation, casual reference, decision-relevant comparison",
        "threat_level": "mentioned|evaluating|actively_comparing|decision_relevant",
        "date": "YYYY-MM-DD",
        "source": "source"
      }
    ],
    "decision_maker_shifts": [
      {
        "shift": "new person referenced, old person disengaged, or chain-of-command change",
        "who": "name or role",
        "date": "YYYY-MM-DD",
        "source": "source",
        "implication": "how this changes our approach"
      }
    ],
    "budget_cycle_signals": [
      {
        "signal": "e.g. 'Annual planning now', 'Q3 budget locked', 'New fiscal year March'",
        "date": "YYYY-MM-DD",
        "source": "source",
        "implication": "timing implication for our renewal/expansion"
      }
    ]
  },

  "commercial_signals": {
    "arr_trend_12mo": [
      { "period": "YYYY-MM", "arr": 185400, "note": "optional context — new contract, expansion, reduction" }
    ],
    "arr_direction": "growing|flat|shrinking",
    "payment_behavior": "on-time|occasional-late|chronically-late|disputes|unknown",
    "payment_evidence": "most recent invoice detail if known",
    "discount_history": [
      { "date": "YYYY-MM-DD", "percent_or_amount": "e.g. 15% or $25K", "reason": "why granted" }
    ],
    "discount_appetite_remaining": "full|partial|exhausted|unknown",
    "budget_cycle_alignment": "customer fiscal year start vs our renewal — tension or alignment narrative",
    "procurement_complexity": {
      "last_cycle_length_days": 45,
      "signers_required": 3,
      "legal_review_required": true,
      "known_gotchas": "MSA renegotiation coming, new procurement policy, etc."
    },
    "previous_renewal_outcome": "renewed flat, renewed with expansion, renegotiated down, churned and won back, etc."
  },

  "advocacy_track": {
    "is_reference_customer": true,
    "logo_permission": "yes|no|requested|unknown",
    "case_study": { "published": false, "in_progress": false, "topic": "optional", "publish_date": "YYYY-MM-DD or null" },
    "speaking_slots": [
      { "event": "event name", "date": "YYYY-MM-DD", "speaker": "who", "topic": "topic" }
    ],
    "beta_programs_in": [
      { "program": "program name", "enrolled_date": "YYYY-MM-DD", "engagement_level": "active|passive|inactive" }
    ],
    "referrals_made": [
      { "referred_company": "name", "outcome": "became-customer|meeting-taken|no-outcome|unknown", "date": "YYYY-MM-DD" }
    ],
    "nps_history": [
      { "survey_date": "YYYY-MM-DD", "score": 67, "verbatim": "customer quote if any", "respondent": "name if known" }
    ],
    "advocacy_trend": "strengthening|stable|cooling"
  },

  "quote_wall": [
    {
      "quote": "verbatim or near-verbatim quote — must be something someone actually said, not paraphrased",
      "speaker": "full name",
      "role": "their role",
      "date": "YYYY-MM-DD",
      "source": "Gong call title / email subject / Slack channel",
      "sentiment": "positive|neutral|negative|mixed",
      "why_it_matters": "one line on what this quote reveals"
    }
  ]
}
```

## Quality Guidance

- **Evidence required.** Every signal needs a date and a source. "Sentiment is cooling" is worthless without "source: 3 shorter-than-usual email replies from Chris Anderson Apr 10-15".
- **Leading over lagging.** If you can only produce one signal per section, make it the earliest indicator. "Champion changed LinkedIn headline last week" beats "champion has been our contact for 2 years".
- **Verbatim quotes only.** For quote_wall and transcript_extraction, if you're paraphrasing, don't include it. Direct speech only.
- **Divergence is signal.** If the customer is happy in meetings but frustrated in tickets, that IS the answer — flag it as divergence_detected: true and describe it.
- **Estimated ARR upside** in expansion questions: only include if you can reasonably ground it (deal size, seat count, tier). Otherwise null.
- **Budget cycle signals** are goldust — if a customer mentioned "we're locked on Q3 budgets" on a call 2 months ago, surface it.
- **Backup champion candidates:** who else at the customer has power + vested interest? Not just "engaged contacts" — people with meaningful influence over decisions.
- **Omit rather than fabricate.** Empty arrays and nulls are expected — an honest "we don't know" beats a hallucinated answer.
- **No markdown, no prose, no commentary.** Just the JSON object.

Your response begins with `{` and ends with `}`. Nothing else.
```

---

## What to watch for in the output

When you paste this into Glean and run it, evaluate:

1. **Parseability** — does it return valid JSON? If not, what fences/prose did it add?
2. **Evidence quality** — are dates and sources actually cited, or is it making up "high confidence" claims?
3. **Divergence detection** — does Glean correctly identify when meeting sentiment ≠ ticket sentiment for Globex Holdings? (They should — there's tension in the block editor issue.)
4. **Verbatim quotes** — does the quote_wall have actual speech from Gong, or paraphrased summaries?
5. **Signal depth** — for Globex Holdings specifically, we expect:
   - Champion: Chris Anderson (strong, no immediate risk flagged)
   - Expansion questions: Parse.ly pricing, headless CMS scope, domain consolidation
   - Competitor mentions: Webflow, Drupal, Google Analytics
   - Transcript signals from the Feb 17 and Mar 25 Globex Holdings/WPVIP check-ins

If Glean returns thin results on a section, that tells us the signal doesn't exist in the data (valid) OR our data sources aren't indexed deeply enough (fixable by adding sources).

---

## If the output is good → integration plan

1. **Add a new SQL migration**: `entity_assessment.health_outlook_signals_json TEXT` — stores the raw JSON from this prompt alongside the existing fields.

2. **Add to PTY enrichment flow**: `intel_queue.rs` calls this prompt after the main enrichment, merges result into the assessment row.

3. **Add a new field to `dimensions_json`** or a new top-level field on `AccountDetail` that the frontend reads.

4. **Frontend consumes via `useAccountDetail`** and renders the triage cards based on `champion_risk`, `commercial_signals.arr_direction`, etc.

5. **For trend-based cards (usage trend, sentiment trend)**: we can pull raw data (usage telemetry CSVs, per-message sentiment) into a PTY analysis step and have Claude output the trend synthesis back to the DB. Same pattern as this prompt, different input data.
