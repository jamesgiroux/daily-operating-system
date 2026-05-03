-- Add dimensions_json column for the 6-dimension intelligence schema.
-- Stores CompetitiveInsight, StrategicPriority, CoverageAssessment, OrgChange,
-- InternalTeamMember, CadenceAssessment, ResponsivenessAssessment, Blocker,
-- ContractContext, ExpansionSignal, AgreementOutlook, SupportHealth,
-- AdoptionSignals, SatisfactionData, and source_attribution as a single JSON blob.
ALTER TABLE entity_assessment ADD COLUMN dimensions_json TEXT;
