CREATE TABLE IF NOT EXISTS projection_ledger (
    projection_id TEXT PRIMARY KEY,
    surface TEXT NOT NULL CHECK (surface IN ('wordpress_db', 'markdown_file')),
    surface_locator TEXT NOT NULL,
    surface_locator_hash TEXT NOT NULL,
    locator_status TEXT NOT NULL DEFAULT 'live' CHECK (
        locator_status IN ('live', 'tombstoned')
    ),
    dailyos_canonical_id TEXT NOT NULL,
    dailyos_source_runtime TEXT NOT NULL,
    dailyos_projection_version INTEGER NOT NULL CHECK (dailyos_projection_version >= 1),
    composition_id TEXT NOT NULL,
    composition_version INTEGER NOT NULL CHECK (composition_version >= 1),
    current_signature_id TEXT REFERENCES projection_signatures(signature_id),
    canonical_signed_payload_sha256 TEXT NOT NULL,
    claim_watermark_sha256 TEXT NOT NULL,
    last_verified_at TEXT,
    verification_status TEXT NOT NULL DEFAULT 'pending' CHECK (
        verification_status IN (
            'pending',
            'verified',
            'stale',
            'tampered',
            'unknown_key',
            'revoked_key',
            'quarantined',
            'rollback',
            'payload_too_large',
            'missing_signature',
            'unsupported_algorithm',
            'unsupported_canonicalization',
            'wrong_runtime_anchor',
            'signature_invalid'
        )
    )
);

CREATE INDEX IF NOT EXISTS idx_projection_ledger_surface_locator
    ON projection_ledger(surface, surface_locator_hash, locator_status);

CREATE INDEX IF NOT EXISTS idx_projection_ledger_composition
    ON projection_ledger(composition_id, composition_version);

CREATE TABLE IF NOT EXISTS projection_signatures (
    signature_id TEXT PRIMARY KEY,
    projection_id TEXT NOT NULL REFERENCES projection_ledger(projection_id),
    key_id TEXT NOT NULL REFERENCES projection_signing_keys(key_id),
    signature_status TEXT NOT NULL CHECK (
        signature_status IN ('active', 'superseded', 'revoked', 'retired')
    ),
    alg TEXT NOT NULL DEFAULT 'Ed25519' CHECK (alg = 'Ed25519'),
    canonicalization TEXT NOT NULL DEFAULT 'RFC8785-JSON' CHECK (
        canonicalization = 'RFC8785-JSON'
    ),
    canonical_signed_payload_bytes BLOB NOT NULL,
    canonical_signed_payload_sha256 TEXT NOT NULL,
    signature_bytes BLOB NOT NULL,
    signature_envelope_b64url TEXT NOT NULL,
    issued_at TEXT NOT NULL,
    superseded_by_signature_id TEXT REFERENCES projection_signatures(signature_id),
    revoked_at TEXT,
    retired_at TEXT
);

CREATE UNIQUE INDEX IF NOT EXISTS idx_projection_signatures_active_projection
    ON projection_signatures(projection_id)
    WHERE signature_status = 'active';

CREATE INDEX IF NOT EXISTS idx_projection_signatures_key
    ON projection_signatures(key_id, signature_status);

CREATE TABLE IF NOT EXISTS projection_ledger_blocks (
    projection_id TEXT NOT NULL REFERENCES projection_ledger(projection_id),
    block_id TEXT NOT NULL,
    block_order INTEGER NOT NULL CHECK (block_order >= 0),
    block_type TEXT NOT NULL,
    block_payload_sha256 TEXT NOT NULL,
    PRIMARY KEY (projection_id, block_id)
);

CREATE INDEX IF NOT EXISTS idx_projection_ledger_blocks_order
    ON projection_ledger_blocks(projection_id, block_order);

CREATE TABLE IF NOT EXISTS projection_ledger_block_refs (
    projection_id TEXT NOT NULL,
    block_id TEXT NOT NULL,
    claim_ref_index INTEGER NOT NULL CHECK (claim_ref_index >= 0),
    claim_id TEXT NOT NULL,
    claim_version INTEGER NOT NULL CHECK (claim_version >= 1),
    field_path TEXT,
    provenance_invocation_id TEXT,
    provenance_field_path TEXT,
    scope_grant_hash TEXT,
    PRIMARY KEY (projection_id, block_id, claim_ref_index),
    FOREIGN KEY (projection_id, block_id)
        REFERENCES projection_ledger_blocks(projection_id, block_id)
        ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_projection_ledger_block_refs_claim
    ON projection_ledger_block_refs(claim_id, claim_version);

CREATE INDEX IF NOT EXISTS idx_projection_ledger_block_refs_projection_claim
    ON projection_ledger_block_refs(projection_id, claim_id);
