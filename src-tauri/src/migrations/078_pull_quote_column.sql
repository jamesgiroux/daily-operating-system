-- 078: Persist pull_quote on entity_assessment so it survives app restart.
ALTER TABLE entity_assessment ADD COLUMN pull_quote TEXT;
