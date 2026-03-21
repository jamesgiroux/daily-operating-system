//! Production demo data for first-run experience (I56).
//!
//! Seeds a curated dataset into the live database with `is_demo = 1` markers
//! so it can be cleanly removed when the user connects real data.
//! Unlike devtools, this runs in release builds and writes to the real workspace.
//! I633: health_score_history seeds + email_signals populated via enrichment pipeline.

use std::path::Path;

use chrono::{Local, TimeZone};

use crate::db::ActionDb;

/// Install demo data into the database and write fixture files to the workspace.
///
/// All rows are marked `is_demo = 1`. Sets `app_state.demo_mode_active = 1`.
pub fn install_demo(db: &ActionDb, workspace: Option<&Path>) -> Result<(), String> {
    let conn = db.conn_ref();
    let now = chrono::Utc::now();
    let today = now.to_rfc3339();

    let date_only = |n: i64| -> String {
        (chrono::Local::now() + chrono::Duration::days(n))
            .format("%Y-%m-%d")
            .to_string()
    };
    let days_ago = |n: i64| -> String { (now - chrono::Duration::days(n)).to_rfc3339() };

    // --- Accounts (3 with varied health/stage) ---
    let accounts: Vec<(&str, &str, &str, f64, &str, Option<&str>)> = vec![
        (
            "demo-acme",
            "Acme Corp",
            "nurture",
            1_200_000.0,
            "green",
            None,
        ),
        (
            "demo-globex",
            "Globex Industries",
            "renewal",
            800_000.0,
            "yellow",
            Some("2026-04-30"),
        ),
        (
            "demo-initech",
            "Initech",
            "onboarding",
            350_000.0,
            "green",
            None,
        ),
    ];

    for (id, name, lifecycle, arr, health, contract_end) in &accounts {
        conn.execute(
            "INSERT OR REPLACE INTO accounts (id, name, lifecycle, arr, health, contract_end, updated_at, is_demo) \
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, 1)",
            rusqlite::params![id, name, lifecycle, arr, health, contract_end, &today],
        )
        .map_err(|e| format!("Demo account insert: {}", e))?;

        // Mirror in entities table
        conn.execute(
            "INSERT OR REPLACE INTO entities (id, name, entity_type, updated_at) VALUES (?1, ?2, 'account', ?3)",
            rusqlite::params![id, name, &today],
        )
        .map_err(|e| format!("Demo entity insert: {}", e))?;
    }

    // --- People (associated with accounts) ---
    let people: Vec<(&str, &str, &str, &str, Option<&str>)> = vec![
        (
            "demo-sarah",
            "sarah@acme.com",
            "Sarah Chen",
            "external",
            Some("demo-acme"),
        ),
        (
            "demo-jamie",
            "jamie@globex.com",
            "Jamie Morrison",
            "external",
            Some("demo-globex"),
        ),
        (
            "demo-dana",
            "dana@initech.com",
            "Dana Patel",
            "external",
            Some("demo-initech"),
        ),
        (
            "demo-priya",
            "priya@initech.com",
            "Priya Sharma",
            "external",
            Some("demo-initech"),
        ),
    ];

    for (id, email, name, relationship, account_id) in &people {
        conn.execute(
            "INSERT OR REPLACE INTO people (id, email, name, relationship, last_seen, is_demo) \
             VALUES (?1, ?2, ?3, ?4, ?5, 1)",
            rusqlite::params![id, email, name, relationship, &today],
        )
        .map_err(|e| format!("Demo person insert: {}", e))?;

        // Link to account if specified
        if let Some(acct) = account_id {
            conn.execute(
                "INSERT INTO account_stakeholders (account_id, person_id, role, relationship_type) \
                 VALUES (?1, ?2, 'associated', 'stakeholder')
                 ON CONFLICT(account_id, person_id) DO NOTHING",
                rusqlite::params![acct, id],
            )
            .map_err(|e| format!("Demo account-stakeholder link: {}", e))?;
        }
    }

    // --- Actions (5 with varied priorities) ---
    let actions: Vec<(&str, &str, &str, &str, Option<&str>, Option<String>, Option<&str>)> = vec![
        (
            "demo-act-1",
            "Send updated SOW to Acme legal team",
            "P1",
            "pending",
            Some("demo-acme"),
            Some(date_only(-1)),
            Some("Sarah Chen confirmed Phase 2 executive sponsorship. Legal needs the updated SOW before scoping."),
        ),
        (
            "demo-act-2",
            "Review Globex QBR deck with AE",
            "P1",
            "pending",
            Some("demo-globex"),
            Some(date_only(0)),
            Some("QBR is the highest-stakes meeting this quarter. Renewal decision expected."),
        ),
        (
            "demo-act-3",
            "Schedule Phase 2 kickoff with Initech",
            "P2",
            "pending",
            Some("demo-initech"),
            Some(date_only(1)),
            Some("Phase 1 wrapped successfully. Dana expressed interest in Phase 2 but budget approval pending."),
        ),
        (
            "demo-act-4",
            "Follow up on NPS survey responses",
            "P2",
            "pending",
            Some("demo-acme"),
            Some(date_only(-7)),
            Some("3 detractors identified in the latest NPS survey. Scores trending down."),
        ),
        (
            "demo-act-5",
            "Draft quarterly impact summary",
            "P3",
            "pending",
            None,
            Some(date_only(7)),
            Some("End-of-quarter impact summary for leadership."),
        ),
    ];

    for (id, title, priority, status, account_id, due_date, context) in &actions {
        conn.execute(
            "INSERT OR REPLACE INTO actions (id, title, priority, status, created_at, due_date, \
             account_id, context, updated_at, is_demo) \
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, 1)",
            rusqlite::params![
                id, title, priority, status, &today, due_date, account_id, context, &today
            ],
        )
        .map_err(|e| format!("Demo action insert: {}", e))?;
    }

    // --- Meetings history (4 with prep content) ---
    let today_local = Local::now();
    let make_iso = |hour: u32, min: u32| -> String {
        today_local
            .date_naive()
            .and_hms_opt(hour, min, 0)
            .map(|naive| {
                Local
                    .from_local_datetime(&naive)
                    .single()
                    .map(|dt| dt.to_rfc3339())
                    .unwrap_or_default()
            })
            .unwrap_or_default()
    };

    let meetings: Vec<(&str, &str, &str, String, Option<&str>, Option<&str>)> = vec![
        (
            "demo-mtg-acme",
            "Acme Corp Weekly Sync",
            "customer",
            make_iso(9, 0),
            Some("demo-acme"),
            Some("Reviewed Phase 1 benchmarks. Performance exceeded targets by 15%. Phase 2 scoping on track."),
        ),
        (
            "demo-mtg-globex",
            "Globex Industries QBR",
            "qbr",
            make_iso(11, 0),
            Some("demo-globex"),
            Some("Expansion to 3 teams going well. Key stakeholder departing Q2. Team B usage declining."),
        ),
        (
            "demo-mtg-initech",
            "Initech Phase 2 Kickoff",
            "customer",
            make_iso(14, 0),
            Some("demo-initech"),
            Some("Phase 1 delivered on time. Dana interested in Phase 2. Budget approval pending."),
        ),
        (
            "demo-mtg-standup",
            "Engineering Standup",
            "team_sync",
            make_iso(10, 0),
            None,
            None,
        ),
    ];

    // Build prep_frozen_json for meetings with prep content
    let build_prep_json = |account_name: Option<&str>, summary: Option<&str>| -> Option<String> {
        let acct = account_name?;
        let sum = summary?;
        Some(
            serde_json::json!({
                "account_context": format!("{} — active customer relationship", acct),
                "meeting_narrative": sum,
                "recommended_actions": ["Review recent activity", "Prepare discussion points"],
                "attendee_context": format!("Key stakeholders from {}", acct)
            })
            .to_string(),
        )
    };

    let account_names: std::collections::HashMap<&str, &str> = [
        ("demo-acme", "Acme Corp"),
        ("demo-globex", "Globex Industries"),
        ("demo-initech", "Initech"),
    ]
    .into_iter()
    .collect();

    for (id, title, mtype, start_time, account_id, summary) in &meetings {
        let prep_json = account_id.and_then(|acct| {
            let acct_name = account_names.get(acct).copied();
            build_prep_json(acct_name, *summary)
        });

        conn.execute(
            "INSERT OR REPLACE INTO meetings (id, title, meeting_type, start_time, created_at) \
             VALUES (?1, ?2, ?3, ?4, ?5)",
            rusqlite::params![id, title, mtype, start_time, &today],
        )
        .map_err(|e| format!("Demo meeting insert: {}", e))?;

        conn.execute(
            "INSERT OR REPLACE INTO meeting_transcripts (meeting_id, summary) VALUES (?1, ?2)",
            rusqlite::params![id, summary],
        )
        .map_err(|e| format!("Demo meeting transcript insert: {}", e))?;

        conn.execute(
            "INSERT OR REPLACE INTO meeting_prep (meeting_id, prep_frozen_json, prep_frozen_at) \
             VALUES (?1, ?2, ?3)",
            rusqlite::params![id, prep_json, &today],
        )
        .map_err(|e| format!("Demo meeting prep insert: {}", e))?;

        // Link customer meetings to their account entity
        if let Some(acct) = account_id {
            conn.execute(
                "INSERT OR IGNORE INTO meeting_entities (meeting_id, entity_id, entity_type) \
                 VALUES (?1, ?2, 'account')",
                rusqlite::params![id, acct],
            )
            .map_err(|e| format!("Demo meeting-entity link: {}", e))?;
        }
    }

    // --- Email signals (2 to show email triage) ---
    conn.execute(
        "INSERT OR REPLACE INTO email_signals (email_id, sender_email, entity_id, entity_type, \
         signal_type, signal_text) \
         VALUES ('demo-email-1', 'sarah@acme.com', 'demo-acme', 'account', 'timeline', \
                 'Acme requesting revised launch date for Phase 2')",
        [],
    )
    .map_err(|e| format!("Demo email signal 1: {}", e))?;

    conn.execute(
        "INSERT OR REPLACE INTO email_signals (email_id, sender_email, entity_id, entity_type, \
         signal_type, signal_text) \
         VALUES ('demo-email-2', 'jamie@globex.com', 'demo-globex', 'account', 'risk', \
                 'Globex budget review — potential scope reduction for Q3')",
        [],
    )
    .map_err(|e| format!("Demo email signal 2: {}", e))?;

    // --- Historical meetings for richer demo experience ---
    let historical: Vec<(&str, &str, &str, String, Option<&str>, Option<&str>)> = vec![
        (
            "demo-mh-acme-7d",
            "Acme Corp Status Call",
            "customer",
            days_ago(7),
            Some("demo-acme"),
            Some("Phase 1 migration completed ahead of schedule. NPS trending down with 3 detractors."),
        ),
        (
            "demo-mh-globex-14d",
            "Globex Sprint Demo",
            "customer",
            days_ago(14),
            Some("demo-globex"),
            Some("New dashboard features demoed. Jamie enthusiastic. Casey raised Team B engagement concerns."),
        ),
        (
            "demo-mh-initech-21d",
            "Initech Phase 1 Review",
            "customer",
            days_ago(21),
            Some("demo-initech"),
            Some("Phase 1 on track. Integration testing completed. Dana confirmed go-live date."),
        ),
    ];

    for (id, title, mtype, start_time, account_id, summary) in &historical {
        conn.execute(
            "INSERT OR REPLACE INTO meetings (id, title, meeting_type, start_time, created_at) \
             VALUES (?1, ?2, ?3, ?4, ?5)",
            rusqlite::params![id, title, mtype, start_time, &today],
        )
        .map_err(|e| format!("Demo historical meeting: {}", e))?;

        conn.execute(
            "INSERT OR REPLACE INTO meeting_transcripts (meeting_id, summary) VALUES (?1, ?2)",
            rusqlite::params![id, summary],
        )
        .map_err(|e| format!("Demo historical meeting transcript: {}", e))?;
        // I635: Historical meetings need prep_frozen_json for prediction scorecard
        let hist_prep = account_id.and_then(|acct| {
            let acct_name = account_names.get(acct).copied();
            build_prep_json(acct_name, *summary)
        });
        conn.execute(
            "INSERT OR REPLACE INTO meeting_prep (meeting_id, prep_frozen_json, prep_frozen_at) \
             VALUES (?1, ?2, ?3)",
            rusqlite::params![id, hist_prep, &today],
        )
        .map_err(|e| format!("Demo historical meeting prep: {}", e))?;

        if let Some(acct) = account_id {
            conn.execute(
                "INSERT OR IGNORE INTO meeting_entities (meeting_id, entity_id, entity_type) \
                 VALUES (?1, ?2, 'account')",
                rusqlite::params![id, acct],
            )
            .map_err(|e| format!("Demo historical meeting-entity link: {}", e))?;
        }
    }

    // ── v1.0.3: Full meeting intelligence mock data ──
    // Mark all customer meetings as transcript-processed (Meeting Record stage)
    // Today's meetings AND historical meetings get full intelligence so they
    // show as Meeting Records once their start time passes.
    for hist_id in &[
        "demo-mtg-acme", "demo-mtg-globex",
        "demo-mh-acme-7d", "demo-mh-globex-14d", "demo-mh-initech-21d",
    ] {
        conn.execute(
            "UPDATE meeting_transcripts SET transcript_processed_at = datetime('now', '-1 hour') \
             WHERE meeting_id = ?1",
            rusqlite::params![hist_id],
        )
        .ok();
    }

    // Enriched captures — full schema with speaker, evidence, urgency, impact, sub_type
    conn.execute_batch(
        "INSERT OR IGNORE INTO captures (id, meeting_id, meeting_title, account_id, capture_type, content, sub_type, urgency, impact, evidence_quote, speaker, captured_at) VALUES
         -- Acme today: full intelligence for the weekly sync
         ('demo-cap-at1', 'demo-mtg-acme', 'Acme Corp Weekly Sync', 'demo-acme', 'win', 'Phase 2 scoping completed with full executive alignment', 'expansion', NULL, 'Sets up Q2 expansion deal worth 6% ARR increase', 'Sarah said: \"Leadership signed off on the Phase 2 scope yesterday. We are ready to move forward.\"', 'Sarah Chen', datetime('now')),
         ('demo-cap-at2', 'demo-mtg-acme', 'Acme Corp Weekly Sync', 'demo-acme', 'win', 'Support ticket volume down 60% since migration', 'value_realized', NULL, 'Validates platform stability narrative for renewal', 'Alex confirmed: \"We went from 15 tickets a week to about 6. The new platform is much more stable.\"', 'Alex Torres', datetime('now')),
         ('demo-cap-at3', 'demo-mtg-acme', 'Acme Corp Weekly Sync', 'demo-acme', 'risk', 'Billing team still struggling with custom report builder', NULL, 'yellow', 'Could slow Phase 2 adoption if not addressed', '\"The billing team tried building their monthly reconciliation report and gave up after 20 minutes.\"', 'Alex Torres', datetime('now')),
         ('demo-cap-at4', 'demo-mtg-acme', 'Acme Corp Weekly Sync', 'demo-acme', 'risk', 'Budget review in April — CFO wants ROI justification', NULL, 'red', 'Renewal at risk without clear ROI story', 'Sarah flagged: \"Our CFO is asking for hard numbers on cost savings before the April budget review.\"', 'Sarah Chen', datetime('now')),
         ('demo-cap-at5', 'demo-mtg-acme', 'Acme Corp Weekly Sync', 'demo-acme', 'decision', 'Build custom report template for billing team before Phase 2 kickoff', NULL, NULL, NULL, 'Agreed to prioritize a billing-specific report template in the next sprint.', 'Sarah Chen', datetime('now')),
         ('demo-cap-at6', 'demo-mtg-acme', 'Acme Corp Weekly Sync', 'demo-acme', 'commitment', 'Deliver ROI analysis deck to CFO by end of month', 'deliverable', NULL, NULL, '\"I will put together the ROI deck with the support ticket data and send it over by the 31st.\"', NULL, datetime('now')),
         ('demo-cap-at7', 'demo-mtg-acme', 'Acme Corp Weekly Sync', 'demo-acme', 'commitment', 'Schedule billing team training session for next week', 'follow_up', NULL, NULL, '\"Let us get a 90-minute session on the calendar for the billing team.\"', NULL, datetime('now')),

         -- Globex today: intelligence for the QBR
         ('demo-cap-gt1', 'demo-mtg-globex', 'Globex Industries QBR', 'demo-globex', 'win', 'Expansion to 3 teams going well — Team A and C fully adopted', 'adoption', NULL, 'Two of three teams at full utilization', 'Jamie said: \"Teams A and C are all in. Usage is exactly where we want it.\"', 'Jamie Reeves', datetime('now')),
         ('demo-cap-gt2', 'demo-mtg-globex', 'Globex Industries QBR', 'demo-globex', 'risk', 'Key stakeholder Jamie departing Q2 — champion transition urgent', NULL, 'red', 'Relationship continuity at risk without formal handoff', '\"My last day is May 15th. Casey and I have started the transition but there is a lot to cover.\"', 'Jamie Reeves', datetime('now')),
         ('demo-cap-gt3', 'demo-mtg-globex', 'Globex Industries QBR', 'demo-globex', 'risk', 'Team B usage declining — 40% drop in logins over last 30 days', NULL, 'yellow', 'Risk of seat reduction at renewal', 'Casey noted: \"Team B is back to spreadsheets. We need to re-engage them before the renewal conversation.\"', 'Casey Kim', datetime('now')),
         ('demo-cap-gt4', 'demo-mtg-globex', 'Globex Industries QBR', 'demo-globex', 'decision', 'Fast-track champion transition — Casey Kim to be primary contact by April 15', NULL, NULL, NULL, 'Both sides agreed Casey needs full context transfer before Jamie leaves.', 'Casey Kim', datetime('now')),
         ('demo-cap-gt5', 'demo-mtg-globex', 'Globex Industries QBR', 'demo-globex', 'commitment', 'Create Team B re-engagement plan with usage targets', 'deliverable', NULL, NULL, '\"We will put together a 30-60-90 plan for Team B with specific adoption milestones.\"', NULL, datetime('now')),

         -- Acme 7d ago: 2 wins, 2 risks, 1 decision, 1 commitment
         ('demo-cap-a1', 'demo-mh-acme-7d', 'Acme Corp Status Call', 'demo-acme', 'win', 'Phase 1 migration completed ahead of schedule', 'adoption', NULL, 'Reduced onboarding time by 40%', 'Sarah confirmed: \"We finished the migration two weeks early and the team is already productive.\"', 'Sarah Chen', datetime('now', '-7 days')),
         ('demo-cap-a2', 'demo-mh-acme-7d', 'Acme Corp Status Call', 'demo-acme', 'win', 'API integration passing all validation checks', 'value_realized', NULL, 'Zero production incidents since go-live', 'Alex noted the integration has been flawless since launch.', 'Alex Torres', datetime('now', '-7 days')),
         ('demo-cap-a3', 'demo-mh-acme-7d', 'Acme Corp Status Call', 'demo-acme', 'risk', 'NPS trending down — 3 detractors in latest survey', NULL, 'red', 'Churn risk if sentiment continues declining', 'Sarah flagged: \"We''re seeing some frustration from the billing team about the reporting interface.\"', 'Sarah Chen', datetime('now', '-7 days')),
         ('demo-cap-a4', 'demo-mh-acme-7d', 'Acme Corp Status Call', 'demo-acme', 'risk', 'Billing team unhappy with reporting interface', NULL, 'yellow', 'Could block Phase 2 expansion', '\"The reports don''t match what they see in their legacy system.\"', 'Alex Torres', datetime('now', '-7 days')),
         ('demo-cap-a5', 'demo-mh-acme-7d', 'Acme Corp Status Call', 'demo-acme', 'decision', 'Proceed with Phase 2 scoping next quarter', NULL, NULL, NULL, 'Both sides agreed to begin Phase 2 discovery in April.', 'Sarah Chen', datetime('now', '-7 days')),
         ('demo-cap-a6', 'demo-mh-acme-7d', 'Acme Corp Status Call', 'demo-acme', 'commitment', 'Send updated ROI analysis by Friday', 'deliverable', NULL, NULL, '\"I''ll have the updated numbers to you by end of week.\"', NULL, datetime('now', '-7 days')),

         -- Globex 14d ago: 1 win, 2 risks, 1 decision, 1 commitment
         ('demo-cap-g1', 'demo-mh-globex-14d', 'Globex Sprint Demo', 'demo-globex', 'win', 'New dashboard features received enthusiastic response', 'engagement', NULL, 'Drove 30% increase in daily active usage', 'Jamie said: \"The team loved the new filtering — usage jumped the day we deployed.\"', 'Jamie Reeves', datetime('now', '-14 days')),
         ('demo-cap-g2', 'demo-mh-globex-14d', 'Globex Sprint Demo', 'demo-globex', 'risk', 'Team B engagement declining — may need dedicated enablement', NULL, 'yellow', 'Team B represents 35% of seats', 'Casey raised concern: \"Team B hasn''t logged in since the UI refresh. They''re still using spreadsheets.\"', 'Casey Kim', datetime('now', '-14 days')),
         ('demo-cap-g3', 'demo-mh-globex-14d', 'Globex Sprint Demo', 'demo-globex', 'risk', 'Key stakeholder Jamie departing in Q2', NULL, 'red', 'Champion loss — relationship continuity at risk', 'Jamie mentioned: \"I''m moving to a new role in May. We should start transitioning Casey as your main point of contact.\"', 'Jamie Reeves', datetime('now', '-14 days')),
         ('demo-cap-g4', 'demo-mh-globex-14d', 'Globex Sprint Demo', 'demo-globex', 'decision', 'Schedule dedicated Team B enablement session', NULL, NULL, NULL, 'Agreed to run a 2-hour hands-on workshop for Team B next sprint.', 'Casey Kim', datetime('now', '-14 days')),
         ('demo-cap-g5', 'demo-mh-globex-14d', 'Globex Sprint Demo', 'demo-globex', 'commitment', 'Coordinate Team B workshop logistics with Casey', 'follow_up', NULL, NULL, '\"Casey will send the invite, we''ll bring the training materials.\"', NULL, datetime('now', '-14 days'));"
    ).ok();

    // Interaction dynamics — talk balance, speaker sentiments, engagement signals
    // Acme today
    conn.execute(
        "INSERT OR IGNORE INTO meeting_interaction_dynamics \
         (meeting_id, talk_balance_customer_pct, talk_balance_internal_pct, \
          speaker_sentiments_json, question_density, decision_maker_active, \
          forward_looking, monologue_risk, competitor_mentions_json, \
          escalation_language_json, created_at) \
         VALUES (?1, 58, 42, ?2, 'high', 'yes', 'high', 0, ?3, ?4, datetime('now'))",
        rusqlite::params![
            "demo-mtg-acme",
            r#"[{"name":"Sarah Chen","sentiment":"positive","evidence":"Enthusiastic about Phase 2 alignment and eager to show ROI to CFO"},{"name":"Alex Torres","sentiment":"cautious","evidence":"Flagged billing team friction but acknowledged support improvements"}]"#,
            r#"[]"#,
            r#"[{"quote":"If we cannot show the CFO hard savings numbers by April, this gets a lot harder","speaker":"Sarah Chen"}]"#,
        ],
    ).ok();

    // Globex today
    conn.execute(
        "INSERT OR IGNORE INTO meeting_interaction_dynamics \
         (meeting_id, talk_balance_customer_pct, talk_balance_internal_pct, \
          speaker_sentiments_json, question_density, decision_maker_active, \
          forward_looking, monologue_risk, competitor_mentions_json, \
          escalation_language_json, created_at) \
         VALUES (?1, 50, 50, ?2, 'medium', 'yes', 'high', 0, ?3, ?4, datetime('now'))",
        rusqlite::params![
            "demo-mtg-globex",
            r#"[{"name":"Jamie Reeves","sentiment":"bittersweet","evidence":"Positive about product but open about departure — wants smooth transition"},{"name":"Casey Kim","sentiment":"determined","evidence":"Taking ownership of the relationship, direct about Team B challenges"}]"#,
            r#"[{"competitor":"Looker","context":"Casey mentioned Team B explored Looker dashboards during the adoption gap"}]"#,
            r#"[{"quote":"If Team B does not adopt by Q3, we will need to reconsider the seat count","speaker":"Casey Kim"}]"#,
        ],
    ).ok();

    // Acme 7d ago
    conn.execute(
        "INSERT OR IGNORE INTO meeting_interaction_dynamics \
         (meeting_id, talk_balance_customer_pct, talk_balance_internal_pct, \
          speaker_sentiments_json, question_density, decision_maker_active, \
          forward_looking, monologue_risk, competitor_mentions_json, \
          escalation_language_json, created_at) \
         VALUES (?1, 62, 38, ?2, 'high', 'yes', 'high', 0, ?3, ?4, datetime('now'))",
        rusqlite::params![
            "demo-mh-acme-7d",
            r#"[{"name":"Sarah Chen","sentiment":"positive","evidence":"Expressed satisfaction with migration timeline and team productivity"},{"name":"Alex Torres","sentiment":"cautious","evidence":"Raised concerns about billing team frustration but acknowledged integration success"}]"#,
            r#"[{"competitor":"Gainsight","context":"Sarah mentioned they evaluated Gainsight before choosing us"}]"#,
            r#"[]"#,
        ],
    ).ok();

    conn.execute(
        "INSERT OR IGNORE INTO meeting_interaction_dynamics \
         (meeting_id, talk_balance_customer_pct, talk_balance_internal_pct, \
          speaker_sentiments_json, question_density, decision_maker_active, \
          forward_looking, monologue_risk, competitor_mentions_json, \
          escalation_language_json, created_at) \
         VALUES (?1, 55, 45, ?2, 'medium', 'yes', 'medium', 0, ?3, ?4, datetime('now'))",
        rusqlite::params![
            "demo-mh-globex-14d",
            r#"[{"name":"Jamie Reeves","sentiment":"positive","evidence":"Enthusiastic about dashboard improvements and transparent about departure timeline"},{"name":"Casey Kim","sentiment":"concerned","evidence":"Raised Team B engagement issues and wants more enablement support"}]"#,
            r#"[]"#,
            r#"[{"quote":"If Team B doesn't adopt by Q3, we'll need to reconsider the seat count","speaker":"Casey Kim"}]"#,
        ],
    ).ok();

    // Champion health assessments
    // Acme today
    conn.execute(
        "INSERT OR IGNORE INTO meeting_champion_health \
         (meeting_id, champion_name, champion_status, champion_evidence, champion_risk, created_at) \
         VALUES (?1, 'Sarah Chen', 'strong', \
          'Driving Phase 2 internally. Proactively managing CFO expectations on ROI. Brought Alex to meeting to build multi-threaded relationship.', \
          'CFO budget review in April could slow momentum if ROI deck is not compelling.', datetime('now'))",
        rusqlite::params!["demo-mtg-acme"],
    ).ok();

    // Globex today
    conn.execute(
        "INSERT OR IGNORE INTO meeting_champion_health \
         (meeting_id, champion_name, champion_status, champion_evidence, champion_risk, created_at) \
         VALUES (?1, 'Jamie Reeves', 'weak', \
          'Departing May 15th. Still fully engaged in transition planning but actively shifting responsibilities to Casey Kim.', \
          'Champion loss imminent. Casey Kim shows strong potential but needs accelerated context transfer. Competitor evaluation by Team B adds urgency.', datetime('now'))",
        rusqlite::params!["demo-mtg-globex"],
    ).ok();

    // Acme 7d ago
    conn.execute(
        "INSERT OR IGNORE INTO meeting_champion_health \
         (meeting_id, champion_name, champion_status, champion_evidence, champion_risk, created_at) \
         VALUES (?1, 'Sarah Chen', 'strong', \
          'Actively advocating for Phase 2 expansion. Provided executive sponsorship for migration acceleration.', \
          'No immediate risk — strong alignment and engagement.', datetime('now'))",
        rusqlite::params!["demo-mh-acme-7d"],
    ).ok();

    conn.execute(
        "INSERT OR IGNORE INTO meeting_champion_health \
         (meeting_id, champion_name, champion_status, champion_evidence, champion_risk, created_at) \
         VALUES (?1, 'Jamie Reeves', 'weak', \
          'Departing in Q2. Still engaged but transitioning responsibilities to Casey Kim.', \
          'Champion loss imminent. Casey Kim is the succession candidate but hasn''t been formally onboarded as champion.', datetime('now'))",
        rusqlite::params!["demo-mh-globex-14d"],
    ).ok();

    // Role changes
    // Globex today
    conn.execute(
        "INSERT OR IGNORE INTO meeting_role_changes \
         (id, meeting_id, person_name, old_status, new_status, evidence_quote, created_at) \
         VALUES ('demo-rc-gt1', 'demo-mtg-globex', 'Casey Kim', 'champion_candidate', 'champion', \
          'Casey is now the primary point of contact. Jamie confirmed the handoff is complete for day-to-day operations.', datetime('now'))",
        [],
    ).ok();

    // Globex 14d ago
    conn.execute(
        "INSERT OR IGNORE INTO meeting_role_changes \
         (id, meeting_id, person_name, old_status, new_status, evidence_quote, created_at) \
         VALUES ('demo-rc-g1', 'demo-mh-globex-14d', 'Jamie Reeves', 'champion', 'departing', \
          'I''m moving to a new role in May. We should start transitioning Casey.', datetime('now'))",
        [],
    ).ok();
    conn.execute(
        "INSERT OR IGNORE INTO meeting_role_changes \
         (id, meeting_id, person_name, old_status, new_status, evidence_quote, created_at) \
         VALUES ('demo-rc-g2', 'demo-mh-globex-14d', 'Casey Kim', 'stakeholder', 'champion_candidate', \
          'Casey will be your main point of contact going forward.', datetime('now'))",
        [],
    ).ok();

    // Meeting attendees for continuity thread
    for (meeting_id, attendees) in &[
        ("demo-mh-acme-7d", vec![("demo-sarah", "sarah@acme.com"), ("demo-alex", "alex@acme.com")]),
        ("demo-mtg-acme", vec![("demo-sarah", "sarah@acme.com"), ("demo-alex", "alex@acme.com"), ("demo-newface", "jordan@acme.com")]),
        ("demo-mh-globex-14d", vec![("demo-jamie", "jamie@globex.com"), ("demo-casey", "casey@globex.com")]),
        ("demo-mtg-globex", vec![("demo-jamie", "jamie@globex.com"), ("demo-casey", "casey@globex.com")]),
    ] {
        for (person_id, email) in attendees {
            conn.execute(
                "INSERT OR IGNORE INTO meeting_attendees (meeting_id, person_id, email) \
                 VALUES (?1, ?2, ?3)",
                rusqlite::params![meeting_id, person_id, email],
            )
            .ok();
        }
    }

    // Transcript-extracted actions for demo meetings
    conn.execute_batch(
        "INSERT OR IGNORE INTO actions (id, title, status, priority, source_type, source_id, account_id, created_at, updated_at, is_demo) VALUES
         -- Today's meetings
         ('demo-act-at1', 'Deliver ROI analysis deck to Acme CFO', 'proposed', 'P1', 'transcript', 'demo-mtg-acme', 'demo-acme', datetime('now'), datetime('now'), 1),
         ('demo-act-at2', 'Build custom billing report template for Acme', 'proposed', 'P1', 'transcript', 'demo-mtg-acme', 'demo-acme', datetime('now'), datetime('now'), 1),
         ('demo-act-at3', 'Schedule billing team training session', 'proposed', 'P2', 'transcript', 'demo-mtg-acme', 'demo-acme', datetime('now'), datetime('now'), 1),
         ('demo-act-gt1', 'Create Team B 30-60-90 re-engagement plan', 'proposed', 'P1', 'transcript', 'demo-mtg-globex', 'demo-globex', datetime('now'), datetime('now'), 1),
         ('demo-act-gt2', 'Complete champion context transfer to Casey Kim', 'proposed', 'P1', 'transcript', 'demo-mtg-globex', 'demo-globex', datetime('now'), datetime('now'), 1),
         -- Historical meetings
         ('demo-act-a1', 'Send updated ROI analysis to Acme', 'completed', 'P1', 'transcript', 'demo-mh-acme-7d', 'demo-acme', datetime('now', '-7 days'), datetime('now', '-3 days'), 1),
         ('demo-act-a2', 'Schedule billing team feedback session', 'pending', 'P1', 'transcript', 'demo-mh-acme-7d', 'demo-acme', datetime('now', '-7 days'), datetime('now', '-7 days'), 1),
         ('demo-act-g1', 'Coordinate Team B enablement workshop', 'pending', 'P1', 'transcript', 'demo-mh-globex-14d', 'demo-globex', datetime('now', '-14 days'), datetime('now', '-14 days'), 1),
         ('demo-act-g2', 'Draft champion transition plan for Casey Kim', 'pending', 'P2', 'transcript', 'demo-mh-globex-14d', 'demo-globex', datetime('now', '-14 days'), datetime('now', '-14 days'), 1);"
    ).ok();

    // I633: Seed health score history for trend computation demo
    for (acct, scores) in &[
        ("demo-acme", vec![(72.0, "green"), (68.0, "yellow"), (75.0, "green")]),
        ("demo-globex", vec![(45.0, "yellow"), (42.0, "yellow"), (38.0, "red")]),
    ] {
        for (i, (score, band)) in scores.iter().enumerate() {
            conn.execute(
                "INSERT OR IGNORE INTO health_score_history (account_id, score, band, confidence, computed_at) \
                 VALUES (?1, ?2, ?3, 0.75, datetime('now', ?4))",
                rusqlite::params![acct, score, band, format!("-{} days", (scores.len() - i) * 7)],
            )
            .map_err(|e| format!("Demo health history: {}", e))?;
        }
    }

    // Set demo mode active
    conn.execute("UPDATE app_state SET demo_mode_active = 1 WHERE id = 1", [])
        .map_err(|e| format!("Set demo_mode_active: {}", e))?;

    // Write fixture files if workspace provided
    if let Some(ws) = workspace {
        write_demo_fixtures(ws)?;
    }

    log::info!("Demo data installed (3 accounts, 4 people, 5 actions, 7 meetings)");
    Ok(())
}

/// Remove all demo data and reset demo mode.
pub fn clear_demo(db: &ActionDb, workspace: Option<&Path>) -> Result<(), String> {
    let conn = db.conn_ref();

    // Delete demo rows from all marked tables
    conn.execute("DELETE FROM actions WHERE is_demo = 1", [])
        .map_err(|e| format!("Clear demo actions: {}", e))?;

    // Clean up meeting-entity links for demo meetings
    conn.execute(
        "DELETE FROM meeting_entities WHERE meeting_id LIKE 'demo-%'",
        [],
    )
    .map_err(|e| format!("Clear demo meeting-entity links: {}", e))?;

    // Clean up health score history for demo accounts
    conn.execute(
        "DELETE FROM health_score_history WHERE account_id LIKE 'demo-%'",
        [],
    )
    .map_err(|e| format!("Clear demo health history: {}", e))?;

    // Clean up v1.0.3 meeting intelligence tables for demo meetings
    conn.execute("DELETE FROM captures WHERE meeting_id LIKE 'demo-%'", [])
        .map_err(|e| format!("Clear demo captures: {}", e))?;
    conn.execute("DELETE FROM meeting_interaction_dynamics WHERE meeting_id LIKE 'demo-%'", [])
        .map_err(|e| format!("Clear demo interaction dynamics: {}", e))?;
    conn.execute("DELETE FROM meeting_champion_health WHERE meeting_id LIKE 'demo-%'", [])
        .map_err(|e| format!("Clear demo champion health: {}", e))?;
    conn.execute("DELETE FROM meeting_role_changes WHERE meeting_id LIKE 'demo-%'", [])
        .map_err(|e| format!("Clear demo role changes: {}", e))?;
    conn.execute("DELETE FROM meeting_attendees WHERE meeting_id LIKE 'demo-%'", [])
        .map_err(|e| format!("Clear demo meeting attendees: {}", e))?;

    // Clean up account_stakeholders links for demo people
    conn.execute(
        "DELETE FROM account_stakeholders WHERE person_id LIKE 'demo-%'",
        [],
    )
    .map_err(|e| format!("Clear demo account-stakeholder links: {}", e))?;

    // Clean up entity_members links for demo people
    conn.execute(
        "DELETE FROM entity_members WHERE person_id LIKE 'demo-%'",
        [],
    )
    .map_err(|e| format!("Clear demo entity-member links: {}", e))?;

    // Clean up meetings for demo meetings (CASCADE handles meeting_prep + meeting_transcripts)
    conn.execute("DELETE FROM meetings WHERE id LIKE 'demo-%'", [])
        .map_err(|e| format!("Clear demo meetings: {}", e))?;

    // Clean up email_signals for demo
    conn.execute("DELETE FROM email_signals WHERE email_id LIKE 'demo-%'", [])
        .map_err(|e| format!("Clear demo email signals: {}", e))?;

    // Clean up entities for demo accounts
    conn.execute("DELETE FROM entities WHERE id LIKE 'demo-%'", [])
        .map_err(|e| format!("Clear demo entities: {}", e))?;

    conn.execute("DELETE FROM people WHERE is_demo = 1", [])
        .map_err(|e| format!("Clear demo people: {}", e))?;
    conn.execute("DELETE FROM accounts WHERE is_demo = 1", [])
        .map_err(|e| format!("Clear demo accounts: {}", e))?;

    // Reset demo mode
    conn.execute("UPDATE app_state SET demo_mode_active = 0 WHERE id = 1", [])
        .map_err(|e| format!("Clear demo_mode_active: {}", e))?;

    // Remove demo fixture files (best-effort — don't fail if files don't exist)
    if let Some(ws) = workspace {
        let data_dir = ws.join("_today").join("data");
        let _ = std::fs::remove_file(data_dir.join("schedule.json"));
        let _ = std::fs::remove_dir_all(data_dir.join("preps"));
    }

    log::info!("Demo data cleared");
    Ok(())
}

/// Get app_state row values.
pub fn get_app_state(db: &ActionDb) -> Result<AppStateRow, String> {
    let conn = db.conn_ref();
    conn.query_row(
        "SELECT demo_mode_active, has_completed_tour, wizard_completed_at, wizard_last_step \
         FROM app_state WHERE id = 1",
        [],
        |row| {
            Ok(AppStateRow {
                demo_mode_active: row.get::<_, i32>(0)? != 0,
                has_completed_tour: row.get::<_, i32>(1)? != 0,
                wizard_completed_at: row.get(2)?,
                wizard_last_step: row.get(3)?,
            })
        },
    )
    .map_err(|e| format!("Read app_state: {}", e))
}

/// Set tour as completed.
pub fn set_tour_completed(db: &ActionDb) -> Result<(), String> {
    let conn = db.conn_ref();
    conn.execute(
        "UPDATE app_state SET has_completed_tour = 1 WHERE id = 1",
        [],
    )
    .map_err(|e| format!("Set tour completed: {}", e))?;
    Ok(())
}

/// Set wizard as completed with current timestamp.
pub fn set_wizard_completed(db: &ActionDb) -> Result<(), String> {
    let conn = db.conn_ref();
    conn.execute(
        "UPDATE app_state SET wizard_completed_at = datetime('now') WHERE id = 1",
        [],
    )
    .map_err(|e| format!("Set wizard completed: {}", e))?;
    Ok(())
}

/// Set wizard last step for resume.
pub fn set_wizard_step(db: &ActionDb, step: &str) -> Result<(), String> {
    let conn = db.conn_ref();
    conn.execute(
        "UPDATE app_state SET wizard_last_step = ?1 WHERE id = 1",
        rusqlite::params![step],
    )
    .map_err(|e| format!("Set wizard step: {}", e))?;
    Ok(())
}

#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppStateRow {
    pub demo_mode_active: bool,
    pub has_completed_tour: bool,
    pub wizard_completed_at: Option<String>,
    pub wizard_last_step: Option<String>,
}

/// Write demo fixture JSON files to the workspace data directory.
fn write_demo_fixtures(workspace: &Path) -> Result<(), String> {
    let data_dir = workspace.join("_today").join("data");
    let preps_dir = data_dir.join("preps");

    std::fs::create_dir_all(&preps_dir)
        .map_err(|e| format!("Failed to create demo preps dir: {}", e))?;

    // Write a minimal schedule.json with today's demo meetings
    let today_local = Local::now();
    let today_str = today_local.format("%Y-%m-%d").to_string();
    let make_time = |hour: u32, min: u32| -> String {
        today_local
            .date_naive()
            .and_hms_opt(hour, min, 0)
            .map(|naive| {
                Local
                    .from_local_datetime(&naive)
                    .single()
                    .map(|dt| dt.to_rfc3339())
                    .unwrap_or_default()
            })
            .unwrap_or_default()
    };

    let schedule = format!(
        r#"{{
  "date": "{}",
  "meetings": [
    {{
      "id": "demo-mtg-acme",
      "title": "Acme Corp Weekly Sync",
      "start": "{}",
      "end": "{}",
      "meeting_type": "customer",
      "attendees": ["sarah@acme.com"],
      "account_id": "demo-acme",
      "has_prep": true
    }},
    {{
      "id": "demo-mtg-standup",
      "title": "Engineering Standup",
      "start": "{}",
      "end": "{}",
      "meeting_type": "team_sync",
      "attendees": [],
      "account_id": null,
      "has_prep": false
    }},
    {{
      "id": "demo-mtg-globex",
      "title": "Globex Industries QBR",
      "start": "{}",
      "end": "{}",
      "meeting_type": "qbr",
      "attendees": ["jamie@globex.com"],
      "account_id": "demo-globex",
      "has_prep": true
    }},
    {{
      "id": "demo-mtg-initech",
      "title": "Initech Phase 2 Kickoff",
      "start": "{}",
      "end": "{}",
      "meeting_type": "customer",
      "attendees": ["dana@initech.com", "priya@initech.com"],
      "account_id": "demo-initech",
      "has_prep": true
    }}
  ]
}}"#,
        today_str,
        make_time(9, 0),
        make_time(9, 30),
        make_time(10, 0),
        make_time(10, 15),
        make_time(11, 0),
        make_time(12, 0),
        make_time(14, 0),
        make_time(14, 45),
    );

    std::fs::write(data_dir.join("schedule.json"), schedule)
        .map_err(|e| format!("Failed to write demo schedule: {}", e))?;

    Ok(())
}
