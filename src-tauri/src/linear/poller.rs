//! Background Linear sync poller (I346).
//!
//! Follows the Clay poller architectural pattern.

use std::sync::Arc;

use crate::state::AppState;

pub async fn run_linear_poller(state: Arc<AppState>) {
    tokio::time::sleep(std::time::Duration::from_secs(60)).await;

    loop {
        let (enabled, api_key, poll_interval) = {
            let config = state.config.read().ok();
            match config.as_ref().and_then(|g| g.as_ref()) {
                Some(c) => (
                    c.linear.enabled,
                    c.linear.api_key.clone(),
                    c.linear.poll_interval_minutes,
                ),
                None => (false, None, 60),
            }
        };

        if !enabled || api_key.is_none() {
            tokio::select! {
                _ = tokio::time::sleep(std::time::Duration::from_secs(300)) => {},
                _ = state.linear_poller_wake.notified() => {
                    log::info!("Linear poller: woken by sync signal (disabled path)");
                },
            }
            continue;
        }

        let api_key = api_key.unwrap();
        log::info!("Linear poller: starting sync");

        let client = crate::linear::client::LinearClient::new(&api_key);

        // Sync issues
        match client.fetch_my_issues().await {
            Ok(issues) => {
                let count = issues.len();
                if let Err(e) = crate::linear::sync::upsert_issues(&state, &issues) {
                    log::warn!("Linear poller: issue sync failed: {}", e);
                } else {
                    log::info!("Linear poller: synced {} issues", count);
                }
            }
            Err(e) => log::warn!("Linear poller: failed to fetch issues: {}", e),
        }

        // Sync projects
        match client.fetch_my_projects().await {
            Ok(projects) => {
                let count = projects.len();
                if let Err(e) = crate::linear::sync::upsert_projects(&state, &projects) {
                    log::warn!("Linear poller: project sync failed: {}", e);
                } else {
                    log::info!("Linear poller: synced {} projects", count);
                }
            }
            Err(e) => log::warn!("Linear poller: failed to fetch projects: {}", e),
        }

        // Sleep until next poll or manual wake
        tokio::select! {
            _ = tokio::time::sleep(std::time::Duration::from_secs(
                poll_interval as u64 * 60,
            )) => {},
            _ = state.linear_poller_wake.notified() => {
                log::info!("Linear poller: woken by manual sync signal");
            },
        }
    }
}
