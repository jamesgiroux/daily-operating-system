-- DOS-258 rule rename: P4a → P4b, P4b → P4c, P4c → P4d.
--
-- Shifts all existing P4 rule identifiers one letter forward so the new
-- P4a (stakeholder-inference) slot is free. Three rule-identifier columns
-- carry these strings:
--   linked_entities_raw.rule_id        → stored bare ("P4a", "P4b", "P4c")
--   linked_entities_raw.source         → prefixed ("rule:P4a", ...)
--   entity_linking_evaluations.rule_id → stored bare
--
-- Executed as a two-pass rename inside a single transaction so a row that
-- was originally "P4a" never gets caught by the next pass (P4b → P4c).
-- Pass 1 prefixes existing identifiers with a sentinel "_";
-- Pass 2 replaces the sentinel with the correctly-shifted letter.
--
-- Idempotent: if no rows match the old identifiers, all statements are no-ops.

BEGIN;

-- ---------------------------------------------------------------------------
-- Pass 1 — mark old identifiers with a sentinel prefix.
-- ---------------------------------------------------------------------------

UPDATE linked_entities_raw SET rule_id = '_P4a' WHERE rule_id = 'P4a';
UPDATE linked_entities_raw SET rule_id = '_P4b' WHERE rule_id = 'P4b';
UPDATE linked_entities_raw SET rule_id = '_P4c' WHERE rule_id = 'P4c';

UPDATE linked_entities_raw SET source  = '_rule:P4a' WHERE source = 'rule:P4a';
UPDATE linked_entities_raw SET source  = '_rule:P4b' WHERE source = 'rule:P4b';
UPDATE linked_entities_raw SET source  = '_rule:P4c' WHERE source = 'rule:P4c';

UPDATE entity_linking_evaluations SET rule_id = '_P4a' WHERE rule_id = 'P4a';
UPDATE entity_linking_evaluations SET rule_id = '_P4b' WHERE rule_id = 'P4b';
UPDATE entity_linking_evaluations SET rule_id = '_P4c' WHERE rule_id = 'P4c';

-- ---------------------------------------------------------------------------
-- Pass 2 — shift each sentinel one letter forward.
-- ---------------------------------------------------------------------------

UPDATE linked_entities_raw SET rule_id = 'P4b' WHERE rule_id = '_P4a';
UPDATE linked_entities_raw SET rule_id = 'P4c' WHERE rule_id = '_P4b';
UPDATE linked_entities_raw SET rule_id = 'P4d' WHERE rule_id = '_P4c';

UPDATE linked_entities_raw SET source  = 'rule:P4b' WHERE source = '_rule:P4a';
UPDATE linked_entities_raw SET source  = 'rule:P4c' WHERE source = '_rule:P4b';
UPDATE linked_entities_raw SET source  = 'rule:P4d' WHERE source = '_rule:P4c';

UPDATE entity_linking_evaluations SET rule_id = 'P4b' WHERE rule_id = '_P4a';
UPDATE entity_linking_evaluations SET rule_id = 'P4c' WHERE rule_id = '_P4b';
UPDATE entity_linking_evaluations SET rule_id = 'P4d' WHERE rule_id = '_P4c';

COMMIT;
