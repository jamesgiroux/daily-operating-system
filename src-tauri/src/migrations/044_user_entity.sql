-- User entity: single-row table for the user's professional context
CREATE TABLE IF NOT EXISTS user_entity (
    id INTEGER PRIMARY KEY CHECK (id = 1),
    name TEXT,
    company TEXT,
    title TEXT,
    focus TEXT,
    value_proposition TEXT,
    success_definition TEXT,
    current_priorities TEXT,
    product_context TEXT,
    playbooks TEXT,
    company_bio TEXT,
    role_description TEXT,
    how_im_measured TEXT,
    pricing_model TEXT,
    differentiators TEXT,
    objections TEXT,
    competitive_context TEXT,
    annual_priorities TEXT,
    quarterly_priorities TEXT,
    user_relevance_weight REAL DEFAULT 1.0,
    created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
);

-- User context entries: professional knowledge snippets for semantic retrieval
CREATE TABLE IF NOT EXISTS user_context_entries (
    id TEXT PRIMARY KEY,
    title TEXT NOT NULL,
    content TEXT NOT NULL,
    embedding_id TEXT,
    created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
);
