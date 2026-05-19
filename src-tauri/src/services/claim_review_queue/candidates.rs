//! Candidate target_kind extension hook. Future surfaces that produce
//! recommendations or proactive candidates enqueue through this factory rather
//! than spawning a parallel review queue.

use crate::services::claim_receipt::contracts::ClaimReceipt;
use crate::services::claim_review_queue::queue::{QueueItem, QueueTargetKind};

pub fn queue_item_for_candidate(candidate_id: &str, receipt: ClaimReceipt) -> QueueItem {
    QueueItem {
        kind: QueueTargetKind::Candidate,
        target_id: candidate_id.to_string(),
        receipt: Some(receipt),
        deferred_until: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::services::claim_receipt::contracts::*;
    use abilities_runtime::abilities::provenance::subject::SubjectRef;
    use abilities_runtime::abilities::trust::types::TrustBand;
    use abilities_runtime::sensitivity::ClaimVerificationState;
    use abilities_runtime::types::{ClaimState, SurfacingState};

    fn fixture_receipt() -> ClaimReceipt {
        ClaimReceipt {
            target: ReceiptTarget::Claim {
                claim_id: "c-1".into(),
                subject: SubjectRef::Global,
                field_path: None,
            },
            surface_context: SurfaceContext::DailyBriefing,
            rendered_text: None,
            trust: ReceiptTrust {
                band: TrustBand::Unscored,
                source_asof: None,
                freshness: Freshness::Unknown,
                caveat: None,
                rationale: None,
            },
            lifecycle: ReceiptLifecycle {
                claim_state: ClaimState::Active,
                surfacing_state: SurfacingState::Active,
                verification_state: ClaimVerificationState::Active,
                updated_at: None,
            },
            provenance: ReceiptProvenance {
                sources: Vec::new(),
                field_path: None,
                evidence_summary: None,
                redaction: RedactionLevel::None,
            },
            actions: Vec::new(),
        }
    }

    #[test]
    fn factory_produces_candidate_kind_with_target_and_receipt() {
        let item = queue_item_for_candidate("cand-42", fixture_receipt());
        assert_eq!(item.kind, QueueTargetKind::Candidate);
        assert_eq!(item.target_id, "cand-42");
        assert!(item.receipt.is_some());
        assert!(item.deferred_until.is_none());
    }
}
