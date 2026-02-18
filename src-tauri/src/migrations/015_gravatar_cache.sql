-- I229: Gravatar MCP integration — avatar and profile cache
CREATE TABLE IF NOT EXISTS gravatar_cache (
    email TEXT PRIMARY KEY,
    avatar_url TEXT,
    display_name TEXT,
    bio TEXT,
    location TEXT,
    company TEXT,
    job_title TEXT,
    interests_json TEXT,
    has_gravatar INTEGER NOT NULL DEFAULT 0,
    fetched_at TEXT NOT NULL,
    person_id TEXT REFERENCES people(id) ON DELETE SET NULL
);

CREATE INDEX IF NOT EXISTS idx_gravatar_cache_person_id ON gravatar_cache(person_id);
