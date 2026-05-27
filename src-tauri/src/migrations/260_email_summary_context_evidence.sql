-- v1.4.4a W5: persist the claim-backed context metadata used when
-- generating email summaries. Legacy contextual_summary rows predate this
-- prompt path, so readers must only display trust/source badges when these
-- fields were written by enrichment.

ALTER TABLE emails ADD COLUMN summary_context_prompt_version TEXT;
ALTER TABLE emails ADD COLUMN summary_context_trust_band TEXT;
ALTER TABLE emails ADD COLUMN summary_context_source_count INTEGER;
ALTER TABLE emails ADD COLUMN summary_context_source_keys_json TEXT;
ALTER TABLE emails ADD COLUMN summary_context_generated_at TEXT;
