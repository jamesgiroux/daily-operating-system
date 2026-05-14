-- v170 is the next free registered migration slot after v169 surface pairing/session authority.
-- W3-B V5 removes bearer tokens from signed surface auth; the column lived on surface_client_sessions.
ALTER TABLE surface_client_sessions DROP COLUMN bearer_token_hash;
