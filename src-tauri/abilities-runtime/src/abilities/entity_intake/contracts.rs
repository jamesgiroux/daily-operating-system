use schemars::gen::SchemaGenerator;
use schemars::schema::Schema;
use schemars::JsonSchema;
use serde::{Deserialize, Deserializer, Serialize};

use crate::abilities::trust::types::TrustBand;
use crate::services::workspace_intake::EntityRefDto;
use crate::types::ClaimSensitivity;

#[derive(Debug, Clone, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct EntityIntakeInput {
    pub file_ref: String,
    pub entity_seed: Option<EntityRefDto>,
    pub category: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct EntityIntakeRenderInput {
    pub entity_type: String,
    pub entity_id: String,
    pub file_ref: String,
}

#[derive(Debug, Clone, Serialize, JsonSchema, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct EntityIntakeOutput {
    pub run_id: String,
    pub file_id: String,
    pub resolved_path: Option<String>,
    pub claims: Vec<EntityIntakeClaim>,
}

#[derive(Debug, Clone, Serialize, JsonSchema, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct EntityIntakeClaim {
    pub claim_id: String,
    pub display_text: String,
    pub trust_band: TrustBand,
    pub sensitivity: ClaimSensitivity,
}

#[derive(Debug, Clone, Serialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "camelCase", tag = "kind")]
pub enum EntityIntakeError {
    InvalidEntityType { value: String },
    InvalidEntityId { value: String },
    EntityNotFound,
    InvalidCategorySlug { value: String },
    CategoryNotAllowed { allowed: Vec<String> },
    FileNotFound,
    PathTraversalAttempt,
    IngestionFailed { message: String },
}

#[derive(Deserialize, JsonSchema)]
struct EntityRefDtoSchema {
    entity_type_slug: String,
    entity_id: String,
    entity_name: Option<String>,
}

impl<'de> Deserialize<'de> for EntityRefDto {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let wire = EntityRefDtoSchema::deserialize(deserializer)?;
        Ok(Self {
            entity_type_slug: wire.entity_type_slug,
            entity_id: wire.entity_id,
            entity_name: wire.entity_name,
        })
    }
}

impl JsonSchema for EntityRefDto {
    fn schema_name() -> String {
        "EntityRefDto".to_string()
    }

    fn json_schema(gen: &mut SchemaGenerator) -> Schema {
        EntityRefDtoSchema::json_schema(gen)
    }
}
