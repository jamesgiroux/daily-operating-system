-- I504: Preserve AI rationale on inferred person-to-person relationships.
ALTER TABLE person_relationships ADD COLUMN rationale TEXT;
