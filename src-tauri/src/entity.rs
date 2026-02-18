//! Profile-agnostic tracked entity abstraction (ADR-0045).
//!
//! CS = Account, PM = Project, Manager = Person. The `entities` table
//! provides a universal layer so core behaviors (last-contact tracking,
//! capture association, action linking) work for any profile. CS-specific
//! fields (lifecycle, ARR, health) remain in the `accounts` table.

use serde::{Deserialize, Serialize};

/// The kind of entity being tracked, driven by active profile.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EntityType {
    Account, // CS profile
    Project, // PM profile (future)
    Person,  // Manager profile (future)
    Other,
}

impl EntityType {
    /// Return the default entity type for a given profile name.
    pub fn default_for_profile(profile: &str) -> Self {
        match profile {
            "customer-success" => EntityType::Account,
            "product-manager" => EntityType::Project,
            "manager" => EntityType::Person,
            _ => EntityType::Other,
        }
    }

    /// String label for SQL storage.
    pub fn as_str(&self) -> &'static str {
        match self {
            EntityType::Account => "account",
            EntityType::Project => "project",
            EntityType::Person => "person",
            EntityType::Other => "other",
        }
    }

    /// Parse from SQL string.
    pub fn from_str_lossy(s: &str) -> Self {
        match s {
            "account" => EntityType::Account,
            "project" => EntityType::Project,
            "person" => EntityType::Person,
            _ => EntityType::Other,
        }
    }
}

/// A row from the `entities` table.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DbEntity {
    pub id: String,
    pub name: String,
    pub entity_type: EntityType,
    pub tracker_path: Option<String>,
    pub updated_at: String,
}
