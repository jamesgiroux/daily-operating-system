//! Ingestion-run tracking — W1-C fills this with the
//! `document_ingestion_runs` table mutations + idempotency keying.
//!
//! W1-A pre-creates this placeholder so no later lane needs to create a new
//! file or edit `mod.rs`.
