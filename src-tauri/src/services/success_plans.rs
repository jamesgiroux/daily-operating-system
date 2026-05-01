use chrono::{Duration, Utc};

use crate::db::ActionDb;
use crate::services::context::ServiceContext;
use crate::state::AppState;
use crate::types::{
    AccountMilestone, AccountObjective, MilestoneCandidate, StatedObjective, SuccessPlanSignals,
    SuccessPlanTemplate, SuggestedMilestone, SuggestedObjective, TemplateMilestone,
    TemplateObjective,
};

fn default_template_catalog() -> Vec<SuccessPlanTemplate> {
    vec![
        SuccessPlanTemplate {
            id: "onboarding-standard".to_string(),
            name: "Onboarding Success Plan".to_string(),
            description: "A starter plan for new customers getting to first value.".to_string(),
            lifecycle_trigger: "onboarding".to_string(),
            objectives: vec![
                TemplateObjective {
                    title: "Technical Setup & Integration".to_string(),
                    description: "Get the implementation running cleanly.".to_string(),
                    milestones: vec![
                        TemplateMilestone {
                            title: "Kickoff completed".to_string(),
                            offset_days: 0,
                            auto_detect_signal: Some("kickoff".to_string()),
                        },
                        TemplateMilestone {
                            title: "Integration configured".to_string(),
                            offset_days: 14,
                            auto_detect_signal: None,
                        },
                        TemplateMilestone {
                            title: "Go-live achieved".to_string(),
                            offset_days: 30,
                            auto_detect_signal: Some("go_live".to_string()),
                        },
                    ],
                },
                TemplateObjective {
                    title: "Initial Value Delivery".to_string(),
                    description: "Get the first meaningful outcome live.".to_string(),
                    milestones: vec![
                        TemplateMilestone {
                            title: "First use case live".to_string(),
                            offset_days: 45,
                            auto_detect_signal: None,
                        },
                        TemplateMilestone {
                            title: "Initial value report shared".to_string(),
                            offset_days: 60,
                            auto_detect_signal: None,
                        },
                        TemplateMilestone {
                            title: "Customer confirms value".to_string(),
                            offset_days: 90,
                            auto_detect_signal: Some("onboarding_complete".to_string()),
                        },
                    ],
                },
                TemplateObjective {
                    title: "Relationship Foundation".to_string(),
                    description: "Build the right sponsor and operating rhythm.".to_string(),
                    milestones: vec![
                        TemplateMilestone {
                            title: "Key stakeholders identified".to_string(),
                            offset_days: 7,
                            auto_detect_signal: None,
                        },
                        TemplateMilestone {
                            title: "Executive sponsor confirmed".to_string(),
                            offset_days: 14,
                            auto_detect_signal: Some("executive_sponsor_change".to_string()),
                        },
                        TemplateMilestone {
                            title: "Regular cadence established".to_string(),
                            offset_days: 30,
                            auto_detect_signal: None,
                        },
                    ],
                },
            ],
        },
        SuccessPlanTemplate {
            id: "growth-expansion".to_string(),
            name: "Growth & Expansion Plan".to_string(),
            description: "A plan for active accounts with room to deepen value.".to_string(),
            lifecycle_trigger: "active".to_string(),
            objectives: vec![
                TemplateObjective {
                    title: "Deepen Product Adoption".to_string(),
                    description: "Grow into new product behavior and usage.".to_string(),
                    milestones: vec![
                        TemplateMilestone {
                            title: "Usage review completed".to_string(),
                            offset_days: 30,
                            auto_detect_signal: None,
                        },
                        TemplateMilestone {
                            title: "Expansion opportunities identified".to_string(),
                            offset_days: 45,
                            auto_detect_signal: None,
                        },
                        TemplateMilestone {
                            title: "New use case proposed".to_string(),
                            offset_days: 60,
                            auto_detect_signal: None,
                        },
                    ],
                },
                TemplateObjective {
                    title: "Expand Stakeholder Footprint".to_string(),
                    description: "Broaden the relationship beyond the current team.".to_string(),
                    milestones: vec![
                        TemplateMilestone {
                            title: "Map additional teams".to_string(),
                            offset_days: 14,
                            auto_detect_signal: None,
                        },
                        TemplateMilestone {
                            title: "Executive business review scheduled".to_string(),
                            offset_days: 30,
                            auto_detect_signal: Some("ebr_completed".to_string()),
                        },
                        TemplateMilestone {
                            title: "Cross-functional champions identified".to_string(),
                            offset_days: 60,
                            auto_detect_signal: None,
                        },
                    ],
                },
                TemplateObjective {
                    title: "Drive Measurable Outcomes".to_string(),
                    description: "Tie usage to business results the customer can see.".to_string(),
                    milestones: vec![
                        TemplateMilestone {
                            title: "Baseline metrics documented".to_string(),
                            offset_days: 14,
                            auto_detect_signal: None,
                        },
                        TemplateMilestone {
                            title: "QBR with ROI data".to_string(),
                            offset_days: 90,
                            auto_detect_signal: Some("qbr_completed".to_string()),
                        },
                        TemplateMilestone {
                            title: "Case study candidate identified".to_string(),
                            offset_days: 120,
                            auto_detect_signal: None,
                        },
                    ],
                },
            ],
        },
        SuccessPlanTemplate {
            id: "renewal-preparation".to_string(),
            name: "Renewal Preparation Plan".to_string(),
            description: "A renewal-readiness plan for accounts approaching term end.".to_string(),
            lifecycle_trigger: "renewing".to_string(),
            objectives: vec![
                TemplateObjective {
                    title: "Secure Renewal Decision".to_string(),
                    description: "Drive the decision process to signature.".to_string(),
                    milestones: vec![
                        TemplateMilestone {
                            title: "Renewal timeline confirmed".to_string(),
                            offset_days: 0,
                            auto_detect_signal: Some("renewal".to_string()),
                        },
                        TemplateMilestone {
                            title: "Decision-maker engaged".to_string(),
                            offset_days: 14,
                            auto_detect_signal: None,
                        },
                        TemplateMilestone {
                            title: "Contract signed".to_string(),
                            offset_days: 90,
                            auto_detect_signal: Some("contract_signed".to_string()),
                        },
                    ],
                },
                TemplateObjective {
                    title: "Demonstrate Value".to_string(),
                    description: "Give the customer a clear renewal case.".to_string(),
                    milestones: vec![
                        TemplateMilestone {
                            title: "ROI summary prepared".to_string(),
                            offset_days: 14,
                            auto_detect_signal: None,
                        },
                        TemplateMilestone {
                            title: "Customer success stories compiled".to_string(),
                            offset_days: 30,
                            auto_detect_signal: None,
                        },
                        TemplateMilestone {
                            title: "Executive review completed".to_string(),
                            offset_days: 45,
                            auto_detect_signal: Some("ebr_completed".to_string()),
                        },
                    ],
                },
                TemplateObjective {
                    title: "Mitigate Risks".to_string(),
                    description: "Reduce preventable friction before the close.".to_string(),
                    milestones: vec![
                        TemplateMilestone {
                            title: "Risk assessment completed".to_string(),
                            offset_days: 7,
                            auto_detect_signal: None,
                        },
                        TemplateMilestone {
                            title: "Competitive threats addressed".to_string(),
                            offset_days: 30,
                            auto_detect_signal: None,
                        },
                        TemplateMilestone {
                            title: "Open issues resolved".to_string(),
                            offset_days: 60,
                            auto_detect_signal: None,
                        },
                    ],
                },
            ],
        },
        SuccessPlanTemplate {
            id: "at-risk-recovery".to_string(),
            name: "At-Risk Recovery Plan".to_string(),
            description: "A short-cycle recovery plan for accounts that need intervention."
                .to_string(),
            lifecycle_trigger: "at_risk".to_string(),
            objectives: vec![
                TemplateObjective {
                    title: "Stabilize the Relationship".to_string(),
                    description: "Get the account back onto an agreed recovery path.".to_string(),
                    milestones: vec![
                        TemplateMilestone {
                            title: "Escalation acknowledged".to_string(),
                            offset_days: 0,
                            auto_detect_signal: Some("escalation".to_string()),
                        },
                        TemplateMilestone {
                            title: "Recovery meeting scheduled".to_string(),
                            offset_days: 3,
                            auto_detect_signal: None,
                        },
                        TemplateMilestone {
                            title: "Recovery plan agreed".to_string(),
                            offset_days: 14,
                            auto_detect_signal: Some("escalation_resolved".to_string()),
                        },
                    ],
                },
                TemplateObjective {
                    title: "Address Root Causes".to_string(),
                    description: "Move the blockers causing instability.".to_string(),
                    milestones: vec![
                        TemplateMilestone {
                            title: "Issues catalogued".to_string(),
                            offset_days: 3,
                            auto_detect_signal: None,
                        },
                        TemplateMilestone {
                            title: "Technical blockers resolved".to_string(),
                            offset_days: 30,
                            auto_detect_signal: None,
                        },
                        TemplateMilestone {
                            title: "Process gaps addressed".to_string(),
                            offset_days: 45,
                            auto_detect_signal: None,
                        },
                    ],
                },
                TemplateObjective {
                    title: "Rebuild Confidence".to_string(),
                    description: "Re-establish positive momentum with the customer.".to_string(),
                    milestones: vec![
                        TemplateMilestone {
                            title: "Quick win delivered".to_string(),
                            offset_days: 14,
                            auto_detect_signal: None,
                        },
                        TemplateMilestone {
                            title: "Health review completed".to_string(),
                            offset_days: 30,
                            auto_detect_signal: Some("health_review".to_string()),
                        },
                        TemplateMilestone {
                            title: "Regular check-in cadence restored".to_string(),
                            offset_days: 45,
                            auto_detect_signal: None,
                        },
                    ],
                },
            ],
        },
    ]
}

fn milestone_candidates_to_suggestions(
    candidates: &[MilestoneCandidate],
) -> Vec<SuggestedMilestone> {
    candidates
        .iter()
        .take(4)
        .map(|candidate| SuggestedMilestone {
            title: candidate.milestone.clone(),
            target_date: candidate.expected_by.clone(),
            auto_detect_event: candidate.auto_detect_event.clone(),
        })
        .collect()
}

fn stated_objective_to_suggestion(
    stated: &StatedObjective,
    milestone_candidates: &[MilestoneCandidate],
) -> SuggestedObjective {
    SuggestedObjective {
        title: stated.objective.clone(),
        description: stated
            .owner
            .clone()
            .map(|owner| format!("Primary owner: {owner}")),
        confidence: stated.confidence.clone(),
        source_evidence: stated.source.clone(),
        milestones: milestone_candidates_to_suggestions(milestone_candidates),
        source_commitment_ids: Vec::new(),
    }
}

pub fn list_templates() -> Vec<SuccessPlanTemplate> {
    default_template_catalog()
}

pub fn get_suggested_templates_for_account(
    lifecycle: Option<&str>,
    health_band: Option<&str>,
    contract_end: Option<&str>,
) -> Vec<SuccessPlanTemplate> {
    let today = Utc::now().date_naive();
    let renewal_near = contract_end
        .and_then(|value| value.get(..10))
        .and_then(|value| chrono::NaiveDate::parse_from_str(value, "%Y-%m-%d").ok())
        .map(|date| {
            let diff = (date - today).num_days();
            (0..=120).contains(&diff)
        })
        .unwrap_or(false);

    default_template_catalog()
        .into_iter()
        .filter(|template| {
            template.lifecycle_trigger == lifecycle.unwrap_or_default()
                || (template.id == "at-risk-recovery"
                    && matches!(health_band, Some("red" | "at_risk" | "at-risk")))
                || (template.id == "renewal-preparation" && renewal_near)
        })
        .collect()
}

pub fn get_objective_suggestions(
    db: &ActionDb,
    account_id: &str,
) -> Result<Vec<SuggestedObjective>, String> {
    let mut suggestions = Vec::new();

    if let Some(raw) = db
        .get_success_plan_signals_json(account_id)
        .map_err(|e: crate::db::DbError| e.to_string())?
    {
        if let Ok(signals) = serde_json::from_str::<SuccessPlanSignals>(&raw) {
            for stated in signals.stated_objectives.iter().take(3) {
                suggestions.push(stated_objective_to_suggestion(
                    stated,
                    &signals.milestone_candidates,
                ));
            }
        }
    }

    let commitments = db
        .get_unconsumed_commitments(account_id)
        .map_err(|e: crate::db::DbError| e.to_string())?;
    for (id, title, owner, target_date, confidence, source) in commitments.into_iter().take(5) {
        suggestions.push(SuggestedObjective {
            title,
            description: owner.map(|value| format!("Primary owner: {value}")),
            confidence,
            source_evidence: source,
            milestones: target_date
                .map(|date| {
                    vec![SuggestedMilestone {
                        title: "Target date reached".to_string(),
                        target_date: Some(date),
                        auto_detect_event: None,
                    }]
                })
                .unwrap_or_default(),
            source_commitment_ids: vec![id],
        });
    }

    // Fallback: if no signals and no commitments, parse existing assessment fields
    if suggestions.is_empty() {
        if let Ok((metrics_json, commitments_json, risks_json)) =
            db.get_assessment_fallback_fields(account_id)
        {
            // Parse success_metrics → candidate objectives
            if let Some(raw) = metrics_json {
                if let Ok(metrics) =
                    serde_json::from_str::<Vec<crate::intelligence::io::SuccessMetric>>(&raw)
                {
                    for metric in metrics
                        .iter()
                        .filter(|m| m.status.as_deref() != Some("achieved"))
                        .take(2)
                    {
                        let desc = match (&metric.target, &metric.current) {
                            (Some(target), Some(current)) => {
                                Some(format!("Current: {current} → Target: {target}"))
                            }
                            (Some(target), None) => Some(format!("Target: {target}")),
                            _ => None,
                        };
                        suggestions.push(SuggestedObjective {
                            title: metric.name.clone(),
                            description: desc,
                            confidence: "medium".to_string(),
                            source_evidence: Some("From account intelligence metrics".to_string()),
                            milestones: Vec::new(),
                            source_commitment_ids: Vec::new(),
                        });
                    }
                }
            }
            // Parse open_commitments → candidate objectives
            if let Some(raw) = commitments_json {
                if let Ok(commitments) =
                    serde_json::from_str::<Vec<crate::intelligence::io::OpenCommitment>>(&raw)
                {
                    for commitment in commitments
                        .iter()
                        .filter(|c| c.status.as_deref() != Some("completed"))
                        .take(2)
                    {
                        let milestones = commitment
                            .due_date
                            .as_ref()
                            .map(|date| {
                                vec![SuggestedMilestone {
                                    title: "Target date reached".to_string(),
                                    target_date: Some(date.clone()),
                                    auto_detect_event: None,
                                }]
                            })
                            .unwrap_or_default();
                        suggestions.push(SuggestedObjective {
                            title: commitment.description.clone(),
                            description: commitment.owner.as_ref().map(|o| format!("Owner: {o}")),
                            confidence: "low".to_string(),
                            source_evidence: commitment.source.clone(),
                            milestones,
                            source_commitment_ids: Vec::new(),
                        });
                    }
                }
            }
            // Parse risks_json → mitigation objectives (critical/watch only)
            if let Some(raw) = risks_json {
                if let Ok(risks) =
                    serde_json::from_str::<Vec<crate::intelligence::io::IntelRisk>>(&raw)
                {
                    for risk in risks
                        .iter()
                        .filter(|r| r.urgency == "critical" || r.urgency == "red")
                        .take(1)
                    {
                        suggestions.push(SuggestedObjective {
                            title: format!("Mitigate: {}", risk.text),
                            description: None,
                            confidence: "low".to_string(),
                            source_evidence: risk.source.clone(),
                            milestones: Vec::new(),
                            source_commitment_ids: Vec::new(),
                        });
                    }
                }
            }
        }
    }

    suggestions.truncate(5);
    Ok(suggestions)
}

pub fn create_objective(
    ctx: &ServiceContext<'_>,
    db: &ActionDb,
    account_id: &str,
    title: &str,
    description: Option<&str>,
    target_date: Option<&str>,
    source: &str,
) -> Result<AccountObjective, String> {
    ctx.check_mutation_allowed().map_err(|e| e.to_string())?;
    db.create_objective(account_id, title, description, target_date, source)
        .map_err(|e: crate::db::DbError| e.to_string())
}

// DOS-209: ServiceContext adds 1 arg; request-object refactor is outside W2-A.
#[allow(clippy::too_many_arguments)]
pub fn update_objective(
    ctx: &ServiceContext<'_>,
    db: &ActionDb,
    objective_id: &str,
    title: Option<&str>,
    description: Option<&str>,
    target_date: Option<&str>,
    sort_order: Option<i32>,
    status: Option<&str>,
) -> Result<AccountObjective, String> {
    ctx.check_mutation_allowed().map_err(|e| e.to_string())?;
    db.update_objective(
        objective_id,
        title,
        description,
        target_date,
        sort_order,
        status,
    )
    .map_err(|e: crate::db::DbError| e.to_string())
}

pub fn complete_objective(
    ctx: &ServiceContext<'_>,
    db: &ActionDb,
    state: &AppState,
    objective_id: &str,
) -> Result<AccountObjective, String> {
    ctx.check_mutation_allowed().map_err(|e| e.to_string())?;
    db.with_transaction(|tx| {
        let objective = tx
            .complete_objective(objective_id)
            .map_err(|e: crate::db::DbError| e.to_string())?;
        crate::services::signals::emit_and_propagate(
            ctx,
            tx,
            &state.signals.engine,
            "account",
            &objective.account_id,
            "objective_completed",
            "user_action",
            Some(&format!("{{\"objective_id\":\"{}\"}}", objective.id)),
            0.95,
        )
        .map_err(|e| format!("signal emit failed: {e}"))?;
        Ok(objective)
    })
}

pub fn abandon_objective(db: &ActionDb, objective_id: &str) -> Result<AccountObjective, String> {
    db.abandon_objective(objective_id)
        .map_err(|e: crate::db::DbError| e.to_string())
}

pub fn delete_objective(
    ctx: &ServiceContext<'_>,
    db: &ActionDb,
    objective_id: &str,
) -> Result<(), String> {
    ctx.check_mutation_allowed().map_err(|e| e.to_string())?;
    db.delete_objective(objective_id)
        .map_err(|e: crate::db::DbError| e.to_string())
}

pub fn create_milestone(
    ctx: &ServiceContext<'_>,
    db: &ActionDb,
    objective_id: &str,
    title: &str,
    target_date: Option<&str>,
    auto_detect_signal: Option<&str>,
) -> Result<AccountMilestone, String> {
    ctx.check_mutation_allowed().map_err(|e| e.to_string())?;
    db.create_milestone(objective_id, title, target_date, auto_detect_signal)
        .map_err(|e: crate::db::DbError| e.to_string())
}

// DOS-209: ServiceContext adds 1 arg; request-object refactor is outside W2-A.
#[allow(clippy::too_many_arguments)]
pub fn update_milestone(
    ctx: &ServiceContext<'_>,
    db: &ActionDb,
    milestone_id: &str,
    title: Option<&str>,
    target_date: Option<&str>,
    auto_detect_signal: Option<&str>,
    sort_order: Option<i32>,
    status: Option<&str>,
) -> Result<AccountMilestone, String> {
    ctx.check_mutation_allowed().map_err(|e| e.to_string())?;
    db.update_milestone(
        milestone_id,
        title,
        target_date,
        auto_detect_signal,
        sort_order,
        status,
    )
    .map_err(|e: crate::db::DbError| e.to_string())
}

pub fn complete_milestone(
    ctx: &ServiceContext<'_>,
    db: &ActionDb,
    state: &AppState,
    milestone_id: &str,
) -> Result<AccountMilestone, String> {
    ctx.check_mutation_allowed().map_err(|e| e.to_string())?;
    db.with_transaction(|tx| {
        let (milestone, objective) = tx
            .complete_milestone(milestone_id)
            .map_err(|e: crate::db::DbError| e.to_string())?;
        crate::services::signals::emit_and_propagate(
            ctx,
            tx,
            &state.signals.engine,
            "account",
            &milestone.account_id,
            "milestone_completed",
            "user_action",
            Some(&format!(
                "{{\"milestone_id\":\"{}\",\"objective_id\":\"{}\"}}",
                milestone.id, milestone.objective_id
            )),
            0.9,
        )
        .map_err(|e| format!("signal emit failed: {e}"))?;
        if let Some(objective) = objective {
            crate::services::signals::emit_and_propagate(
            ctx,
                tx,
                &state.signals.engine,
                "account",
                &objective.account_id,
                "objective_completed",
                "user_action",
                Some(&format!("{{\"objective_id\":\"{}\"}}", objective.id)),
                0.95,
            )
            .map_err(|e| format!("signal emit failed: {e}"))?;
        }
        Ok(milestone)
    })
}

pub fn skip_milestone(
    ctx: &ServiceContext<'_>,
    db: &ActionDb,
    state: &AppState,
    milestone_id: &str,
) -> Result<AccountMilestone, String> {
    ctx.check_mutation_allowed().map_err(|e| e.to_string())?;
    db.with_transaction(|tx| {
        let (milestone, objective) = tx
            .skip_milestone(milestone_id)
            .map_err(|e: crate::db::DbError| e.to_string())?;
        if let Some(objective) = objective {
            crate::services::signals::emit_and_propagate(
            ctx,
                tx,
                &state.signals.engine,
                "account",
                &objective.account_id,
                "objective_completed",
                "user_action",
                Some(&format!("{{\"objective_id\":\"{}\"}}", objective.id)),
                0.95,
            )
            .map_err(|e| format!("signal emit failed: {e}"))?;
        }
        Ok(milestone)
    })
}

pub fn delete_milestone(
    ctx: &ServiceContext<'_>,
    db: &ActionDb,
    milestone_id: &str,
) -> Result<(), String> {
    ctx.check_mutation_allowed().map_err(|e| e.to_string())?;
    db.delete_milestone(milestone_id)
        .map_err(|e: crate::db::DbError| e.to_string())
}

pub fn link_action_to_objective(
    ctx: &ServiceContext<'_>,
    db: &ActionDb,
    action_id: &str,
    objective_id: &str,
) -> Result<(), String> {
    ctx.check_mutation_allowed().map_err(|e| e.to_string())?;
    db.link_action_to_objective(action_id, objective_id)
        .map_err(|e: crate::db::DbError| e.to_string())
}

pub fn unlink_action_from_objective(
    ctx: &ServiceContext<'_>,
    db: &ActionDb,
    action_id: &str,
    objective_id: &str,
) -> Result<(), String> {
    ctx.check_mutation_allowed().map_err(|e| e.to_string())?;
    db.unlink_action_from_objective(action_id, objective_id)
        .map_err(|e: crate::db::DbError| e.to_string())
}

pub fn reorder_objectives(
    db: &ActionDb,
    account_id: &str,
    ordered_ids: &[String],
) -> Result<(), String> {
    db.reorder_objectives(account_id, ordered_ids)
        .map_err(|e: crate::db::DbError| e.to_string())
}

pub fn reorder_milestones(
    db: &ActionDb,
    objective_id: &str,
    ordered_ids: &[String],
) -> Result<(), String> {
    db.reorder_milestones(objective_id, ordered_ids)
        .map_err(|e: crate::db::DbError| e.to_string())
}

pub fn create_objective_from_suggestion(
    ctx: &ServiceContext<'_>,
    db: &ActionDb,
    account_id: &str,
    suggestion: &SuggestedObjective,
) -> Result<AccountObjective, String> {
    ctx.check_mutation_allowed().map_err(|e| e.to_string())?;
    db.with_transaction(|tx| {
        let objective = tx
            .create_objective(
                account_id,
                &suggestion.title,
                suggestion.description.as_deref(),
                None,
                "ai_suggested",
            )
            .map_err(|e: crate::db::DbError| e.to_string())?;
        for milestone in &suggestion.milestones {
            tx.create_milestone(
                &objective.id,
                &milestone.title,
                milestone.target_date.as_deref(),
                milestone.auto_detect_event.as_deref(),
            )
            .map_err(|e: crate::db::DbError| e.to_string())?;
        }
        if !suggestion.source_commitment_ids.is_empty() {
            tx.mark_commitments_consumed(&suggestion.source_commitment_ids)
                .map_err(|e: crate::db::DbError| e.to_string())?;
        }
        tx.get_objective(&objective.id)
            .map_err(|e: crate::db::DbError| e.to_string())?
            .ok_or_else(|| "Objective could not be reloaded".to_string())
    })
}

pub fn apply_success_plan_template(
    ctx: &ServiceContext<'_>,
    db: &ActionDb,
    account_id: &str,
    template_id: &str,
) -> Result<Vec<AccountObjective>, String> {
    ctx.check_mutation_allowed().map_err(|e| e.to_string())?;
    let template = default_template_catalog()
        .into_iter()
        .find(|item| item.id == template_id)
        .ok_or_else(|| format!("Unknown success plan template: {template_id}"))?;
    let today = ctx.clock.now().date_naive();
    db.with_transaction(|tx| {
        let mut created = Vec::new();
        for objective in &template.objectives {
            let created_objective = tx
                .create_objective(
                    account_id,
                    &objective.title,
                    Some(&objective.description),
                    None,
                    "template",
                )
                .map_err(|e: crate::db::DbError| e.to_string())?;
            for milestone in &objective.milestones {
                let target_date = (today + Duration::days(milestone.offset_days as i64))
                    .format("%Y-%m-%d")
                    .to_string();
                tx.create_milestone(
                    &created_objective.id,
                    &milestone.title,
                    Some(&target_date),
                    milestone.auto_detect_signal.as_deref(),
                )
                .map_err(|e: crate::db::DbError| e.to_string())?;
            }
            created.push(
                tx.get_objective(&created_objective.id)
                    .map_err(|e: crate::db::DbError| e.to_string())?
                    .ok_or_else(|| "Template objective could not be reloaded".to_string())?,
            );
        }
        Ok(created)
    })
}

/// DOS-16: Match unconsumed commitments to milestone titles via Jaccard similarity.
///
/// For each unconsumed commitment on an account, compares its title against all
/// pending milestones across active objectives. If similarity > 0.7, links the
/// commitment to the milestone (and optionally backfills the milestone's target_date).
/// Emits a `commitment_milestone_linked` signal per match.
pub fn match_commitments_to_milestones(
    ctx: &ServiceContext<'_>,
    db: &ActionDb,
    account_id: &str,
) -> Result<usize, String> {
    ctx.check_mutation_allowed().map_err(|e| e.to_string())?;
    let commitments = db
        .get_unconsumed_commitments(account_id)
        .map_err(|e| e.to_string())?;
    if commitments.is_empty() {
        return Ok(0);
    }

    let objectives = db
        .get_account_objectives(account_id)
        .map_err(|e| e.to_string())?;

    let mut matched = 0usize;

    // Collect all pending milestones across active objectives
    let milestones: Vec<&crate::types::AccountMilestone> = objectives
        .iter()
        .filter(|o| o.status == "active")
        .flat_map(|o| o.milestones.iter())
        .filter(|m| m.status == "pending")
        .collect();

    // commitments tuple: (id, title, owner, target_date, confidence, source)
    for (commit_id, commit_title, _owner, commit_target_date, _confidence, _source) in &commitments
    {
        for milestone in &milestones {
            let score =
                crate::helpers::jaccard_word_similarity(commit_title, &milestone.title);
            if score > 0.7 {
                // Link commitment to milestone
                if let Err(e) = db.conn_ref().execute(
                    "UPDATE captured_commitments SET milestone_id = ?1, suggested_objective_id = ?2 WHERE id = ?3",
                    rusqlite::params![milestone.id, milestone.objective_id, commit_id],
                ) {
                    log::warn!(
                        "Failed to link commitment {} to milestone {}: {}",
                        commit_id,
                        milestone.id,
                        e
                    );
                    continue;
                }

                // Backfill milestone target_date if commitment has one and milestone doesn't
                if milestone.target_date.is_none() {
                    if let Some(ref td) = commit_target_date {
                        let _ = db.update_milestone(
                            &milestone.id,
                            None,
                            Some(td),
                            None,
                            None,
                            None,
                        );
                    }
                }

                // Emit signal
                let _ = crate::services::signals::emit(
                    ctx,
                    db,
                    "account",
                    account_id,
                    "commitment_milestone_linked",
                    "system",
                    Some(&format!(
                        "{{\"commitment_id\":\"{}\",\"milestone_id\":\"{}\",\"score\":{:.2}}}",
                        commit_id, milestone.id, score
                    )),
                    score,
                );

                log::info!(
                    "Linked commitment {} to milestone {} (score: {:.2})",
                    commit_id,
                    milestone.id,
                    score
                );
                matched += 1;
                break; // One milestone match per commitment
            }
        }
    }

    Ok(matched)
}

#[allow(clippy::items_after_test_module)]
#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::test_utils::test_db;
    use crate::services::context::{ExternalClients, FixedClock, SeedableRng, ServiceContext};
    use chrono::TimeZone;
    use rusqlite::params;

    fn test_ctx<'a>(
        clock: &'a FixedClock,
        rng: &'a SeedableRng,
        ext: &'a ExternalClients,
    ) -> ServiceContext<'a> {
        ServiceContext::test_live(clock, rng, ext)
    }

    fn seed_account(db: &crate::db::ActionDb, id: &str) {
        db.conn_ref()
            .execute(
                "INSERT INTO accounts (id, name, account_type, updated_at)
                 VALUES (?1, ?2, 'customer', '2026-01-01T00:00:00Z')",
                params![id, format!("Account {id}")],
            )
            .expect("seed account");
    }

    #[test]
    fn test_create_objective() {
        let db = test_db();
        seed_account(&db, "acc-sp");
        let clock = FixedClock::new(chrono::Utc.with_ymd_and_hms(2026, 4, 30, 0, 0, 0).unwrap());
        let rng = SeedableRng::new(42);
        let ext = ExternalClients::default();
        let ctx = test_ctx(&clock, &rng, &ext);

        let obj = create_objective(
            &ctx,
            &db,
            "acc-sp",
            "Onboard customer",
            Some("Get them live"),
            None,
            "user",
        )
        .expect("create_objective");
        assert_eq!(obj.title, "Onboard customer");
        assert_eq!(obj.account_id, "acc-sp");
        assert_eq!(obj.status, "active");

        // Verify in DB
        let count: i64 = db
            .conn_ref()
            .query_row(
                "SELECT COUNT(*) FROM account_objectives WHERE account_id = 'acc-sp'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(count, 1);
    }

    #[test]
    fn test_apply_template() {
        let db = test_db();
        seed_account(&db, "acc-tmpl");
        let clock = FixedClock::new(chrono::Utc.with_ymd_and_hms(2026, 4, 30, 0, 0, 0).unwrap());
        let rng = SeedableRng::new(42);
        let ext = ExternalClients::default();
        let ctx = test_ctx(&clock, &rng, &ext);

        let objectives = apply_success_plan_template(&ctx, &db, "acc-tmpl", "onboarding-standard")
            .expect("apply_success_plan_template");
        // Onboarding template has 3 objectives
        assert_eq!(
            objectives.len(),
            3,
            "Onboarding template should create 3 objectives"
        );

        // Each objective should have milestones
        for obj in &objectives {
            assert!(
                !obj.milestones.is_empty(),
                "Each objective should have milestones"
            );
        }

        // Verify total milestone count in DB (3 objectives x 3 milestones each = 9)
        let milestone_count: i64 = db
            .conn_ref()
            .query_row(
                "SELECT COUNT(*) FROM account_milestones WHERE account_id = 'acc-tmpl'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(
            milestone_count, 9,
            "Onboarding template should create 9 milestones total"
        );
    }

    #[test]
    fn test_link_action_to_objective() {
        let db = test_db();
        seed_account(&db, "acc-link");
        let clock = FixedClock::new(chrono::Utc.with_ymd_and_hms(2026, 4, 30, 0, 0, 0).unwrap());
        let rng = SeedableRng::new(42);
        let ext = ExternalClients::default();
        let ctx = test_ctx(&clock, &rng, &ext);

        let obj = create_objective(&ctx, &db, "acc-link", "Goal A", None, None, "user")
            .expect("create objective");

        // Seed a minimal action row (status must match CHECK constraint)
        db.conn_ref()
            .execute(
                "INSERT INTO actions (id, title, status, created_at, updated_at)
                 VALUES ('act-1', 'Do the thing', 'unstarted', '2026-01-01T00:00:00Z', '2026-01-01T00:00:00Z')",
                [],
            )
            .expect("seed action");

        link_action_to_objective(&ctx, &db, "act-1", &obj.id).expect("link_action_to_objective");

        let link_count: i64 = db
            .conn_ref()
            .query_row(
                "SELECT COUNT(*) FROM action_objective_links WHERE action_id = 'act-1' AND objective_id = ?1",
                params![obj.id],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(link_count, 1, "Action-objective link should exist");
    }

    #[test]
    fn test_reorder_objectives() {
        let db = test_db();
        seed_account(&db, "acc-ro");
        let clock = FixedClock::new(chrono::Utc.with_ymd_and_hms(2026, 4, 30, 0, 0, 0).unwrap());
        let rng = SeedableRng::new(42);
        let ext = ExternalClients::default();
        let ctx = test_ctx(&clock, &rng, &ext);

        let obj1 = create_objective(&ctx, &db, "acc-ro", "First", None, None, "user").unwrap();
        let obj2 = create_objective(&ctx, &db, "acc-ro", "Second", None, None, "user").unwrap();
        let obj3 = create_objective(&ctx, &db, "acc-ro", "Third", None, None, "user").unwrap();

        // Reorder: Third, First, Second
        let new_order = vec![obj3.id.clone(), obj1.id.clone(), obj2.id.clone()];
        reorder_objectives(&db, "acc-ro", &new_order).expect("reorder_objectives");

        // Verify sort_order
        let order3: i32 = db
            .conn_ref()
            .query_row(
                "SELECT sort_order FROM account_objectives WHERE id = ?1",
                params![obj3.id],
                |row| row.get(0),
            )
            .unwrap();
        let order1: i32 = db
            .conn_ref()
            .query_row(
                "SELECT sort_order FROM account_objectives WHERE id = ?1",
                params![obj1.id],
                |row| row.get(0),
            )
            .unwrap();
        assert!(
            order3 < order1,
            "obj3 should have lower sort_order than obj1 after reorder"
        );
    }

    #[test]
    fn test_create_milestone() {
        let db = test_db();
        seed_account(&db, "acc-ms");
        let clock = FixedClock::new(chrono::Utc.with_ymd_and_hms(2026, 4, 30, 0, 0, 0).unwrap());
        let rng = SeedableRng::new(42);
        let ext = ExternalClients::default();
        let ctx = test_ctx(&clock, &rng, &ext);

        let obj = create_objective(&ctx, &db, "acc-ms", "Goal", None, None, "user").unwrap();
        let ms = create_milestone(
            &ctx,
            &db,
            &obj.id,
            "Kickoff done",
            Some("2026-04-01"),
            Some("kickoff"),
        )
        .expect("create_milestone");

        assert_eq!(ms.title, "Kickoff done");
        assert_eq!(ms.objective_id, obj.id);
        assert_eq!(ms.status, "pending");
    }

    #[test]
    fn test_complete_all_milestones_auto_completes_objective() {
        let db = test_db();
        seed_account(&db, "acc-auto");
        let engine = crate::signals::propagation::PropagationEngine::default();
        let clock = FixedClock::new(chrono::Utc.with_ymd_and_hms(2026, 4, 30, 0, 0, 0).unwrap());
        let rng = SeedableRng::new(42);
        let ext = ExternalClients::default();
        let ctx = test_ctx(&clock, &rng, &ext);

        let obj = create_objective(&ctx, &db, "acc-auto", "Onboard", None, None, "user").unwrap();
        let ms1 = create_milestone(&ctx, &db, &obj.id, "Kickoff", None, None).unwrap();
        let ms2 = create_milestone(&ctx, &db, &obj.id, "Go-live", None, None).unwrap();

        // Complete first milestone — objective should stay active
        db.with_transaction(|tx| {
            let (_, auto_obj) = tx
                .complete_milestone(&ms1.id)
                .map_err(|e: crate::db::DbError| e.to_string())?;
            assert!(
                auto_obj.is_none(),
                "Objective should not auto-complete with pending milestones"
            );
            Ok(())
        })
        .unwrap();

        // Complete second (last) milestone — objective should auto-complete
        db.with_transaction(|tx| {
            let (_, auto_obj) = tx
                .complete_milestone(&ms2.id)
                .map_err(|e: crate::db::DbError| e.to_string())?;
            assert!(
                auto_obj.is_some(),
                "Objective should auto-complete when all milestones done"
            );
            let completed_obj = auto_obj.unwrap();
            assert_eq!(completed_obj.status, "completed");

            // Emit signal as the service layer would
            crate::services::signals::emit_and_propagate(
                &ctx,
                tx,
                &engine,
                "account",
                "acc-auto",
                "objective_completed",
                "user_action",
                Some(&format!("{{\"objective_id\":\"{}\"}}", completed_obj.id)),
                0.95,
            )
            .map_err(|e| e.to_string())?;
            Ok(())
        })
        .unwrap();

        // Verify objective status in DB
        let status: String = db
            .conn_ref()
            .query_row(
                "SELECT status FROM account_objectives WHERE id = ?1",
                params![obj.id],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(status, "completed", "Objective should be completed in DB");

        // Verify signal emitted
        let signal_count: i64 = db
            .conn_ref()
            .query_row(
                "SELECT COUNT(*) FROM signal_events WHERE entity_id = 'acc-auto' AND signal_type = 'objective_completed'",
                [],
                |row| row.get(0),
            )
            .unwrap_or(0);
        assert!(signal_count > 0, "Expected objective_completed signal");
    }
}

// ─── DOS-14: Objective Reconciliation ────────────────────────────────

/// Reconcile AI-extracted statedObjectives against user-created objectives.
///
/// For each statedObjective from enrichment:
/// - If it fuzzy-matches an existing objective: append evidence
/// - If no match: leave it as a suggestion candidate (surfaced by get_objective_suggestions)
pub fn reconcile_objectives(
    ctx: &ServiceContext<'_>,
    db: &ActionDb,
    account_id: &str,
) -> Result<u32, String> {
    ctx.check_mutation_allowed().map_err(|e| e.to_string())?;
    let signals_json = db
        .get_success_plan_signals_json(account_id)
        .map_err(|e| e.to_string())?;

    let signals: Option<SuccessPlanSignals> =
        signals_json.and_then(|j| serde_json::from_str(&j).ok());

    let stated = match signals {
        Some(s) if !s.stated_objectives.is_empty() => s.stated_objectives,
        _ => return Ok(0),
    };

    let objectives = db
        .get_account_objectives(account_id)
        .map_err(|e| e.to_string())?;

    let now = ctx.clock.now().to_rfc3339();
    let mut matches_found = 0u32;

    for ai_obj in &stated {
        let ai_title_lower = ai_obj.objective.trim().to_lowercase();
        let ai_tokens: std::collections::HashSet<&str> =
            ai_title_lower.split_whitespace().collect();

        // Find best matching user objective (Jaccard similarity on word tokens)
        let mut best_match: Option<(&AccountObjective, f64)> = None;
        for obj in &objectives {
            let obj_lower = obj.title.trim().to_lowercase();
            let obj_tokens: std::collections::HashSet<&str> =
                obj_lower.split_whitespace().collect();

            let intersection = ai_tokens.intersection(&obj_tokens).count();
            let union = ai_tokens.union(&obj_tokens).count();
            let jaccard = if union > 0 {
                intersection as f64 / union as f64
            } else {
                0.0
            };

            if jaccard > 0.5 && best_match.as_ref().is_none_or(|(_, s)| jaccard > *s) {
                best_match = Some((obj, jaccard));
            }
        }

        if let Some((matched_obj, _score)) = best_match {
            // Append evidence to the matched objective
            let evidence_entry = serde_json::json!({
                "source": ai_obj.source,
                "date": now,
                "quote": ai_obj.objective,
                "confidence": ai_obj.confidence,
            });

            let mut evidence: Vec<serde_json::Value> = matched_obj
                .evidence_json
                .as_ref()
                .and_then(|j| serde_json::from_str(j).ok())
                .unwrap_or_default();

            evidence.push(evidence_entry);
            // Cap at 20 evidence items
            if evidence.len() > 20 {
                evidence.drain(0..evidence.len() - 20);
            }

            let evidence_str = serde_json::to_string(&evidence).unwrap_or_default();
            db.conn_ref()
                .execute(
                    "UPDATE account_objectives SET evidence_json = ?1, updated_at = ?2 WHERE id = ?3",
                    rusqlite::params![evidence_str, now, matched_obj.id],
                )
                .map_err(|e| e.to_string())?;

            matches_found += 1;
        }
        // If no match: leave as suggestion candidate (get_objective_suggestions already surfaces it)
    }

    Ok(matches_found)
}
