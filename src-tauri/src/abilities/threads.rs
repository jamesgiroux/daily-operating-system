use chrono::{DateTime, Utc};

use crate::abilities::provenance::ThreadId;
use crate::services::context::{SeededRng, ServiceContext};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ThreadMetadata {
    pub id: ThreadId,
    pub created_at: DateTime<Utc>,
    pub created_by: String,
    pub display_label: Option<String>,
}

/// Dormant v1.4.0 construction helper. Stabilize as public API only
/// when the v1.4.2 pilot locks thread creation semantics.
pub(crate) fn create_thread(
    ctx: &ServiceContext<'_>,
    display_label: Option<&str>,
) -> ThreadMetadata {
    ThreadMetadata {
        id: thread_id_from_rng(ctx.rng),
        created_at: ctx.clock.now(),
        created_by: ctx.actor.to_string(),
        display_label: display_label.map(str::to_string),
    }
}

fn thread_id_from_rng(rng: &dyn SeededRng) -> ThreadId {
    let mut bytes = [0u8; 16];
    bytes[..8].copy_from_slice(&rng.random_u64().to_be_bytes());
    bytes[8..].copy_from_slice(&rng.random_u64().to_be_bytes());
    bytes[6] = (bytes[6] & 0x0f) | 0x40;
    bytes[8] = (bytes[8] & 0x3f) | 0x80;
    ThreadId::new(uuid::Uuid::from_bytes(bytes))
}
