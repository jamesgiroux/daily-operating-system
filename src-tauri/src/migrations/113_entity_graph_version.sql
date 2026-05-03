-- entity_graph_version singleton counter + triggers.
--
-- Replaces per-evaluation hash walks with a trigger-maintained O(1) counter.
-- The evaluate() function reads this version at the start of each evaluation
-- and stores it in entity_linking_evaluations. If the version has changed by
-- the time the evaluation tries to commit (because a concurrent write bumped
-- it), the evaluation retries once against the new snapshot.
--
-- Triggers fire on every mutation to the four graph inputs:
--   1. account_domains   — domain → account mapping changes
--   2. account_stakeholders — stakeholder roster changes
--   3. accounts.keywords — entity keyword changes
--   4. projects.keywords — project keyword changes
--   5. projects.account_id — project → account link changes
--
-- Using AFTER UPDATE OF col_name ON table restricts keyword triggers to
-- actual keyword column writes, avoiding spurious bumps from unrelated
-- account/project updates.

CREATE TABLE IF NOT EXISTS entity_graph_version (
    id      INTEGER PRIMARY KEY CHECK (id = 1),
    version INTEGER NOT NULL
);

INSERT OR IGNORE INTO entity_graph_version (id, version) VALUES (1, 0);

-- account_domains: any row change alters domain→account mapping
CREATE TRIGGER IF NOT EXISTS bump_egv_account_domains_i
    AFTER INSERT ON account_domains
    BEGIN UPDATE entity_graph_version SET version = version + 1 WHERE id = 1; END;

CREATE TRIGGER IF NOT EXISTS bump_egv_account_domains_u
    AFTER UPDATE ON account_domains
    BEGIN UPDATE entity_graph_version SET version = version + 1 WHERE id = 1; END;

CREATE TRIGGER IF NOT EXISTS bump_egv_account_domains_d
    AFTER DELETE ON account_domains
    BEGIN UPDATE entity_graph_version SET version = version + 1 WHERE id = 1; END;

-- account_stakeholders: stakeholder roster changes affect P4b/P4c group evidence
CREATE TRIGGER IF NOT EXISTS bump_egv_stakeholders_i
    AFTER INSERT ON account_stakeholders
    BEGIN UPDATE entity_graph_version SET version = version + 1 WHERE id = 1; END;

CREATE TRIGGER IF NOT EXISTS bump_egv_stakeholders_u
    AFTER UPDATE ON account_stakeholders
    BEGIN UPDATE entity_graph_version SET version = version + 1 WHERE id = 1; END;

CREATE TRIGGER IF NOT EXISTS bump_egv_stakeholders_d
    AFTER DELETE ON account_stakeholders
    BEGIN UPDATE entity_graph_version SET version = version + 1 WHERE id = 1; END;

-- accounts.keywords: P5 title evidence depends on entity keyword set
CREATE TRIGGER IF NOT EXISTS bump_egv_account_keywords
    AFTER UPDATE OF keywords ON accounts
    BEGIN UPDATE entity_graph_version SET version = version + 1 WHERE id = 1; END;

-- projects.keywords: project keyword changes affect title/subject evidence
CREATE TRIGGER IF NOT EXISTS bump_egv_project_keywords
    AFTER UPDATE OF keywords ON projects
    BEGIN UPDATE entity_graph_version SET version = version + 1 WHERE id = 1; END;

-- projects.account_id: changes which account a project is linked to
CREATE TRIGGER IF NOT EXISTS bump_egv_project_account_id
    AFTER UPDATE OF account_id ON projects
    BEGIN UPDATE entity_graph_version SET version = version + 1 WHERE id = 1; END;
