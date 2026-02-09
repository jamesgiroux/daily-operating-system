//! Entity Intelligence I/O and types (I130 / ADR-0057).
//!
//! Three-file entity pattern: dashboard.json (mechanical) + intelligence.json
//! (synthesized) + dashboard.md (artifact). This module owns the intelligence
//! layer — types, file I/O, and migration from the legacy CompanyOverview.
//!
//! Intelligence is entity-generic: the same `IntelligenceJson` schema applies
//! to accounts, projects, and people. The enrichment prompt is parameterized
//! by entity_type (handled in Phase 2).

use std::path::Path;

use chrono::Utc;
use serde::{Deserialize, Serialize};

use crate::accounts::CompanyOverview;
use crate::db::{ActionDb, DbAccount};
use crate::util::atomic_write_str;

// =============================================================================
// Intelligence JSON Schema
// =============================================================================

/// Top-level intelligence file (intelligence.json).
///
/// Entity-generic — same schema for accounts, projects, and people per ADR-0057.
/// Factual data (ARR, health, lifecycle) stays in dashboard.json. Intelligence
/// is synthesized assessment that the AI produces from all available signals.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct IntelligenceJson {
    #[serde(default = "default_version")]
    pub version: u32,
    #[serde(default)]
    pub entity_id: String,
    #[serde(default)]
    pub entity_type: String,
    #[serde(default)]
    pub enriched_at: String,
    #[serde(default)]
    pub source_file_count: usize,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub source_manifest: Vec<SourceManifestEntry>,

    /// Prose assessment: account situation / project status / relationship brief.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub executive_assessment: Option<String>,

    /// Account risks / project blockers / relationship risks.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub risks: Vec<IntelRisk>,

    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub recent_wins: Vec<IntelWin>,

    /// Working / not working / unknowns assessment.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub current_state: Option<CurrentState>,

    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub stakeholder_insights: Vec<StakeholderInsight>,

    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub value_delivered: Vec<ValueItem>,

    /// Prep items for the next meeting with this entity.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub next_meeting_readiness: Option<MeetingReadiness>,

    /// Company/project context from web search or overview.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub company_context: Option<CompanyContext>,
}

fn default_version() -> u32 {
    1
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SourceManifestEntry {
    pub filename: String,
    pub modified_at: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub format: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IntelRisk {
    pub text: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,
    #[serde(default = "default_urgency")]
    pub urgency: String,
}

fn default_urgency() -> String {
    "watch".to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IntelWin {
    pub text: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub impact: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct CurrentState {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub working: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub not_working: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub unknowns: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StakeholderInsight {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub role: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub assessment: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub engagement: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ValueItem {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub date: Option<String>,
    pub statement: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub impact: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MeetingReadiness {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub meeting_title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub meeting_date: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub prep_items: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CompanyContext {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub industry: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub size: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub headquarters: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub additional_context: Option<String>,
}

// =============================================================================
// File I/O
// =============================================================================

const INTEL_FILENAME: &str = "intelligence.json";

/// Read intelligence.json from an entity directory.
pub fn read_intelligence_json(dir: &Path) -> Result<IntelligenceJson, String> {
    let path = dir.join(INTEL_FILENAME);
    let content = std::fs::read_to_string(&path)
        .map_err(|e| format!("Failed to read {}: {}", path.display(), e))?;
    serde_json::from_str(&content)
        .map_err(|e| format!("Failed to parse {}: {}", path.display(), e))
}

/// Write intelligence.json atomically to an entity directory.
pub fn write_intelligence_json(dir: &Path, intel: &IntelligenceJson) -> Result<(), String> {
    std::fs::create_dir_all(dir)
        .map_err(|e| format!("Failed to create {}: {}", dir.display(), e))?;
    let path = dir.join(INTEL_FILENAME);
    let content = serde_json::to_string_pretty(intel)
        .map_err(|e| format!("Serialize error: {}", e))?;
    atomic_write_str(&path, &content)
        .map_err(|e| format!("Write error: {}", e))?;
    Ok(())
}

/// Check if intelligence.json exists in an entity directory.
pub fn intelligence_exists(dir: &Path) -> bool {
    dir.join(INTEL_FILENAME).exists()
}

// =============================================================================
// Migration: CompanyOverview → intelligence.json
// =============================================================================

/// Migrate legacy CompanyOverview from dashboard.json into intelligence.json.
///
/// Non-destructive: creates intelligence.json if it doesn't exist and
/// dashboard.json has a company_overview. Leaves dashboard.json untouched.
/// Returns the created IntelligenceJson, or None if no migration needed.
pub fn migrate_company_overview_to_intelligence(
    workspace: &Path,
    account: &DbAccount,
    overview: &CompanyOverview,
) -> Option<IntelligenceJson> {
    let dir = crate::accounts::resolve_account_dir(workspace, account);

    // Don't overwrite existing intelligence
    if intelligence_exists(&dir) {
        return None;
    }

    // Only migrate if there's actual content
    if overview.description.is_none()
        && overview.industry.is_none()
        && overview.size.is_none()
        && overview.headquarters.is_none()
    {
        return None;
    }

    let intel = IntelligenceJson {
        version: 1,
        entity_id: account.id.clone(),
        entity_type: "account".to_string(),
        enriched_at: overview
            .enriched_at
            .clone()
            .unwrap_or_else(|| Utc::now().to_rfc3339()),
        company_context: Some(CompanyContext {
            description: overview.description.clone(),
            industry: overview.industry.clone(),
            size: overview.size.clone(),
            headquarters: overview.headquarters.clone(),
            additional_context: None,
        }),
        ..Default::default()
    };

    match write_intelligence_json(&dir, &intel) {
        Ok(()) => {
            log::info!(
                "Migrated CompanyOverview → intelligence.json for '{}'",
                account.name
            );
            Some(intel)
        }
        Err(e) => {
            log::warn!(
                "Failed to migrate intelligence for '{}': {}",
                account.name, e
            );
            None
        }
    }
}

// =============================================================================
// DB Cache Operations
// =============================================================================

impl ActionDb {
    /// Insert or update the entity_intelligence cache row.
    pub fn upsert_entity_intelligence(
        &self,
        intel: &IntelligenceJson,
    ) -> Result<(), rusqlite::Error> {
        self.conn_ref().execute(
            "INSERT INTO entity_intelligence (
                entity_id, entity_type, enriched_at, source_file_count,
                executive_assessment, risks_json, recent_wins_json,
                current_state_json, stakeholder_insights_json,
                next_meeting_readiness_json, company_context_json
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)
            ON CONFLICT(entity_id) DO UPDATE SET
                entity_type = excluded.entity_type,
                enriched_at = excluded.enriched_at,
                source_file_count = excluded.source_file_count,
                executive_assessment = excluded.executive_assessment,
                risks_json = excluded.risks_json,
                recent_wins_json = excluded.recent_wins_json,
                current_state_json = excluded.current_state_json,
                stakeholder_insights_json = excluded.stakeholder_insights_json,
                next_meeting_readiness_json = excluded.next_meeting_readiness_json,
                company_context_json = excluded.company_context_json",
            rusqlite::params![
                intel.entity_id,
                intel.entity_type,
                intel.enriched_at,
                intel.source_file_count,
                intel.executive_assessment,
                serde_json::to_string(&intel.risks).ok(),
                serde_json::to_string(&intel.recent_wins).ok(),
                serde_json::to_string(&intel.current_state).ok(),
                serde_json::to_string(&intel.stakeholder_insights).ok(),
                serde_json::to_string(&intel.next_meeting_readiness).ok(),
                serde_json::to_string(&intel.company_context).ok(),
            ],
        )?;
        Ok(())
    }

    /// Get cached entity intelligence.
    pub fn get_entity_intelligence(
        &self,
        entity_id: &str,
    ) -> Result<Option<IntelligenceJson>, rusqlite::Error> {
        let mut stmt = self.conn_ref().prepare(
            "SELECT entity_id, entity_type, enriched_at, source_file_count,
                    executive_assessment, risks_json, recent_wins_json,
                    current_state_json, stakeholder_insights_json,
                    next_meeting_readiness_json, company_context_json
             FROM entity_intelligence WHERE entity_id = ?1",
        )?;

        let result = stmt.query_row(rusqlite::params![entity_id], |row| {
            let risks_json: Option<String> = row.get(5)?;
            let wins_json: Option<String> = row.get(6)?;
            let state_json: Option<String> = row.get(7)?;
            let stakeholder_json: Option<String> = row.get(8)?;
            let readiness_json: Option<String> = row.get(9)?;
            let company_json: Option<String> = row.get(10)?;

            Ok(IntelligenceJson {
                version: 1,
                entity_id: row.get(0)?,
                entity_type: row.get(1)?,
                enriched_at: row.get::<_, Option<String>>(2)?.unwrap_or_default(),
                source_file_count: row.get::<_, Option<usize>>(3)?.unwrap_or(0),
                source_manifest: Vec::new(), // Not cached in DB
                executive_assessment: row.get(4)?,
                risks: risks_json
                    .and_then(|j| serde_json::from_str(&j).ok())
                    .unwrap_or_default(),
                recent_wins: wins_json
                    .and_then(|j| serde_json::from_str(&j).ok())
                    .unwrap_or_default(),
                current_state: state_json
                    .and_then(|j| serde_json::from_str(&j).ok()),
                stakeholder_insights: stakeholder_json
                    .and_then(|j| serde_json::from_str(&j).ok())
                    .unwrap_or_default(),
                value_delivered: Vec::new(), // Not cached in DB (stored in file only)
                next_meeting_readiness: readiness_json
                    .and_then(|j| serde_json::from_str(&j).ok()),
                company_context: company_json
                    .and_then(|j| serde_json::from_str(&j).ok()),
            })
        });

        match result {
            Ok(intel) => Ok(Some(intel)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e),
        }
    }

    /// Delete cached entity intelligence.
    pub fn delete_entity_intelligence(
        &self,
        entity_id: &str,
    ) -> Result<(), rusqlite::Error> {
        self.conn_ref().execute(
            "DELETE FROM entity_intelligence WHERE entity_id = ?1",
            rusqlite::params![entity_id],
        )?;
        Ok(())
    }
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    fn test_db() -> ActionDb {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("entity_intel_test.db");
        std::mem::forget(dir);
        ActionDb::open_at(path).expect("open test db")
    }

    fn sample_intel() -> IntelligenceJson {
        IntelligenceJson {
            version: 1,
            entity_id: "acme-corp".to_string(),
            entity_type: "account".to_string(),
            enriched_at: "2026-02-01T10:00:00Z".to_string(),
            source_file_count: 3,
            source_manifest: vec![SourceManifestEntry {
                filename: "qbr-notes.md".to_string(),
                modified_at: "2026-01-30T10:00:00Z".to_string(),
                format: Some("markdown".to_string()),
            }],
            executive_assessment: Some(
                "Acme is in a strong position with steady renewal trajectory.".to_string(),
            ),
            risks: vec![IntelRisk {
                text: "Champion leaving in Q2".to_string(),
                source: Some("qbr-notes.md".to_string()),
                urgency: "critical".to_string(),
            }],
            recent_wins: vec![IntelWin {
                text: "Expanded to 3 new teams".to_string(),
                source: Some("capture".to_string()),
                impact: Some("20% seat growth".to_string()),
            }],
            current_state: Some(CurrentState {
                working: vec!["Onboarding flow".to_string()],
                not_working: vec!["Reporting integration".to_string()],
                unknowns: vec!["Budget for next year".to_string()],
            }),
            stakeholder_insights: vec![StakeholderInsight {
                name: "Alice VP".to_string(),
                role: Some("VP Engineering".to_string()),
                assessment: Some("Strong advocate, drives adoption.".to_string()),
                engagement: Some("high".to_string()),
                source: Some("meetings".to_string()),
            }],
            value_delivered: vec![ValueItem {
                date: Some("2026-01-15".to_string()),
                statement: "Reduced onboarding time by 40%".to_string(),
                source: Some("qbr-deck.pdf".to_string()),
                impact: Some("$50k savings".to_string()),
            }],
            next_meeting_readiness: Some(MeetingReadiness {
                meeting_title: Some("Weekly sync".to_string()),
                meeting_date: Some("2026-02-05".to_string()),
                prep_items: vec![
                    "Review reporting blockers".to_string(),
                    "Prepare champion transition plan".to_string(),
                ],
            }),
            company_context: Some(CompanyContext {
                description: Some("Enterprise SaaS platform.".to_string()),
                industry: Some("Technology".to_string()),
                size: Some("500-1000".to_string()),
                headquarters: Some("San Francisco, USA".to_string()),
                additional_context: None,
            }),
        }
    }

    #[test]
    fn test_intelligence_json_roundtrip() {
        let intel = sample_intel();
        let json_str = serde_json::to_string_pretty(&intel).expect("serialize");
        let parsed: IntelligenceJson = serde_json::from_str(&json_str).expect("deserialize");

        assert_eq!(parsed.entity_id, "acme-corp");
        assert_eq!(parsed.entity_type, "account");
        assert_eq!(parsed.risks.len(), 1);
        assert_eq!(parsed.risks[0].urgency, "critical");
        assert_eq!(parsed.recent_wins.len(), 1);
        assert_eq!(parsed.stakeholder_insights.len(), 1);
        assert_eq!(parsed.value_delivered.len(), 1);
        assert!(parsed.next_meeting_readiness.is_some());
        assert!(parsed.company_context.is_some());
        assert_eq!(parsed.source_manifest.len(), 1);
    }

    #[test]
    fn test_intelligence_json_missing_fields() {
        // Minimal JSON — serde should fill defaults for all missing fields
        let json_str = r#"{"entityId": "beta", "entityType": "project"}"#;
        let parsed: IntelligenceJson = serde_json::from_str(json_str).expect("deserialize");

        assert_eq!(parsed.entity_id, "beta");
        assert_eq!(parsed.entity_type, "project");
        assert_eq!(parsed.version, 1);
        assert!(parsed.risks.is_empty());
        assert!(parsed.recent_wins.is_empty());
        assert!(parsed.executive_assessment.is_none());
        assert!(parsed.current_state.is_none());
        assert!(parsed.company_context.is_none());
    }

    #[test]
    fn test_write_read_intelligence_file() {
        let dir = tempfile::tempdir().expect("tempdir");
        let intel = sample_intel();

        write_intelligence_json(dir.path(), &intel).expect("write");
        assert!(intelligence_exists(dir.path()));

        let read_back = read_intelligence_json(dir.path()).expect("read");
        assert_eq!(read_back.entity_id, "acme-corp");
        assert_eq!(read_back.risks.len(), 1);
        assert_eq!(read_back.source_file_count, 3);
    }

    #[test]
    fn test_migrate_company_overview() {
        let dir = tempfile::tempdir().expect("tempdir");
        let workspace = dir.path();

        // Create account directory
        let acct_dir = workspace.join("Accounts/Acme Corp");
        std::fs::create_dir_all(&acct_dir).expect("mkdir");

        let account = DbAccount {
            id: "acme-corp".to_string(),
            name: "Acme Corp".to_string(),
            lifecycle: None,
            arr: None,
            health: None,
            contract_start: None,
            contract_end: None,
            csm: None,
            champion: None,
            nps: None,
            tracker_path: Some("Accounts/Acme Corp".to_string()),
            parent_id: None,
            updated_at: Utc::now().to_rfc3339(),
        };

        let overview = CompanyOverview {
            description: Some("Cloud platform company.".to_string()),
            industry: Some("SaaS".to_string()),
            size: Some("200-500".to_string()),
            headquarters: Some("NYC".to_string()),
            enriched_at: Some("2026-01-15T10:00:00Z".to_string()),
        };

        let result = migrate_company_overview_to_intelligence(workspace, &account, &overview);
        assert!(result.is_some());

        let intel = result.unwrap();
        assert_eq!(intel.entity_id, "acme-corp");
        assert_eq!(intel.entity_type, "account");
        assert!(intel.company_context.is_some());
        let ctx = intel.company_context.unwrap();
        assert_eq!(ctx.description.as_deref(), Some("Cloud platform company."));
        assert_eq!(ctx.industry.as_deref(), Some("SaaS"));

        // File should exist now
        assert!(intelligence_exists(&acct_dir));

        // Second migration should return None (file already exists)
        let second = migrate_company_overview_to_intelligence(workspace, &account, &overview);
        assert!(second.is_none());
    }

    #[test]
    fn test_migrate_empty_overview_skipped() {
        let dir = tempfile::tempdir().expect("tempdir");
        let workspace = dir.path();
        let acct_dir = workspace.join("Accounts/Empty Corp");
        std::fs::create_dir_all(&acct_dir).expect("mkdir");

        let account = DbAccount {
            id: "empty-corp".to_string(),
            name: "Empty Corp".to_string(),
            lifecycle: None,
            arr: None,
            health: None,
            contract_start: None,
            contract_end: None,
            csm: None,
            champion: None,
            nps: None,
            tracker_path: Some("Accounts/Empty Corp".to_string()),
            parent_id: None,
            updated_at: Utc::now().to_rfc3339(),
        };

        let overview = CompanyOverview {
            description: None,
            industry: None,
            size: None,
            headquarters: None,
            enriched_at: None,
        };

        let result = migrate_company_overview_to_intelligence(workspace, &account, &overview);
        assert!(result.is_none());
    }

    #[test]
    fn test_db_upsert_get_entity_intelligence() {
        let db = test_db();
        let intel = sample_intel();

        db.upsert_entity_intelligence(&intel).expect("upsert");

        let fetched = db
            .get_entity_intelligence("acme-corp")
            .expect("get")
            .expect("should exist");

        assert_eq!(fetched.entity_id, "acme-corp");
        assert_eq!(fetched.entity_type, "account");
        assert_eq!(fetched.executive_assessment, intel.executive_assessment);
        assert_eq!(fetched.risks.len(), 1);
        assert_eq!(fetched.risks[0].urgency, "critical");
        assert_eq!(fetched.recent_wins.len(), 1);
        assert_eq!(fetched.stakeholder_insights.len(), 1);
        assert!(fetched.company_context.is_some());
    }

    #[test]
    fn test_db_intelligence_missing_returns_none() {
        let db = test_db();
        let result = db
            .get_entity_intelligence("nonexistent")
            .expect("should not error");
        assert!(result.is_none());
    }

    #[test]
    fn test_db_delete_entity_intelligence() {
        let db = test_db();
        let intel = sample_intel();

        db.upsert_entity_intelligence(&intel).expect("upsert");
        assert!(db.get_entity_intelligence("acme-corp").unwrap().is_some());

        db.delete_entity_intelligence("acme-corp").expect("delete");
        assert!(db.get_entity_intelligence("acme-corp").unwrap().is_none());
    }

    #[test]
    fn test_db_upsert_overwrites() {
        let db = test_db();
        let mut intel = sample_intel();

        db.upsert_entity_intelligence(&intel).expect("first upsert");

        // Update the assessment
        intel.executive_assessment = Some("Updated assessment.".to_string());
        intel.risks.push(IntelRisk {
            text: "New risk".to_string(),
            source: None,
            urgency: "watch".to_string(),
        });

        db.upsert_entity_intelligence(&intel).expect("second upsert");

        let fetched = db
            .get_entity_intelligence("acme-corp")
            .unwrap()
            .unwrap();
        assert_eq!(
            fetched.executive_assessment.as_deref(),
            Some("Updated assessment.")
        );
        assert_eq!(fetched.risks.len(), 2);
    }
}
