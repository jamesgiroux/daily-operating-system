-- DOS-258 follow-up: complete entity_graph_version trigger coverage.
--
-- Migration 113 added triggers for account_domains, account_stakeholders,
-- and keyword column updates on accounts/projects. It was missing INSERT,
-- DELETE, and name/archived changes on accounts and projects — all inputs
-- that P5 (title evidence) and P4b/P4c (domain evidence) read.
--
-- Without these triggers, the following changes would NOT bump the graph
-- version and evaluations running concurrently would use a stale snapshot:
--   • A new account is created → P4/P5 candidates change immediately
--   • An account is archived → P5 should stop matching it
--   • An account is renamed → P5 name-match results change
--   • A project is created/renamed/archived → same as above
--
-- All triggers follow the same pattern as migration 113:
--   UPDATE entity_graph_version SET version = version + 1 WHERE id = 1

-- accounts: new account or deleted account changes the P4/P5 candidate set
CREATE TRIGGER IF NOT EXISTS bump_egv_accounts_i
    AFTER INSERT ON accounts
    BEGIN UPDATE entity_graph_version SET version = version + 1 WHERE id = 1; END;

CREATE TRIGGER IF NOT EXISTS bump_egv_accounts_d
    AFTER DELETE ON accounts
    BEGIN UPDATE entity_graph_version SET version = version + 1 WHERE id = 1; END;

-- accounts: name change affects P5 whole-word matching
CREATE TRIGGER IF NOT EXISTS bump_egv_accounts_name
    AFTER UPDATE OF name ON accounts
    BEGIN UPDATE entity_graph_version SET version = version + 1 WHERE id = 1; END;

-- accounts: archive/unarchive changes which accounts P5 considers
CREATE TRIGGER IF NOT EXISTS bump_egv_accounts_archived
    AFTER UPDATE OF archived ON accounts
    BEGIN UPDATE entity_graph_version SET version = version + 1 WHERE id = 1; END;

-- projects: new project or deleted project
CREATE TRIGGER IF NOT EXISTS bump_egv_projects_i
    AFTER INSERT ON projects
    BEGIN UPDATE entity_graph_version SET version = version + 1 WHERE id = 1; END;

CREATE TRIGGER IF NOT EXISTS bump_egv_projects_d
    AFTER DELETE ON projects
    BEGIN UPDATE entity_graph_version SET version = version + 1 WHERE id = 1; END;

-- projects: name change affects P5 matching
CREATE TRIGGER IF NOT EXISTS bump_egv_projects_name
    AFTER UPDATE OF name ON projects
    BEGIN UPDATE entity_graph_version SET version = version + 1 WHERE id = 1; END;

-- projects: archive/unarchive
CREATE TRIGGER IF NOT EXISTS bump_egv_projects_archived
    AFTER UPDATE OF archived ON projects
    BEGIN UPDATE entity_graph_version SET version = version + 1 WHERE id = 1; END;
