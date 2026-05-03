//! Google Drive integration for DailyOS.
//!
//! Provides Drive document import and continuous sync via Changes API.
//! Files imported via Google Picker land in entity Documents/ folders
//! and are automatically processed by the watcher for intel enrichment.

pub mod client;
pub mod poller;
pub mod sync;

pub use poller::run_drive_poller;
pub use sync::{get_all_watched_sources, remove_watched_source, upsert_watched_source};
