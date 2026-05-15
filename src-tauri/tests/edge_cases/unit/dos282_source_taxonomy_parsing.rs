use dailyos_lib::abilities::provenance::{
    DataSource, GleanDownstream, LifecycleBehavior, ScoringClass,
};

#[test]
fn source_taxonomy_tracks_glean_downstream_lineage() {
    let salesforce = DataSource::Glean {
        downstream: GleanDownstream::Salesforce,
    };
    let slack = DataSource::Glean {
        downstream: GleanDownstream::Slack,
    };

    assert_eq!(salesforce.scoring_class(), ScoringClass::Scoring);
    assert_eq!(slack.scoring_class(), ScoringClass::Context);
    assert_eq!(salesforce.lifecycle_behavior(), LifecycleBehavior::Mask);
    assert_eq!(slack.display_name(), "Glean Slack");
    assert!(salesforce.is_structured_trusted_source());
    assert!(!slack.is_structured_trusted_source());
}
