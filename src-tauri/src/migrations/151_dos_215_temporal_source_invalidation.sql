ALTER TABLE entity_engagement_curve
    ADD COLUMN source_invalidated_at TIMESTAMP NULL;

ALTER TABLE person_role_progression
    ADD COLUMN source_invalidated_at TIMESTAMP NULL;
