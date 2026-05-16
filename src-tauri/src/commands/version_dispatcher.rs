#![allow(
    clippy::let_underscore_must_use,
    reason = "tauri::command macro emits internal Result glue that discards generated metadata"
)]

//! Tauri command entrypoint for the version-event dispatcher.
//!
//! Native `Actor::User` and `Actor::Agent` consumers subscribe through here.
//! The subscribe command takes a `tauri::ipc::Channel<DispatchedEvent>`;
//! events are forwarded from the dispatcher's per-subscriber `mpsc::Receiver`
//! to the channel in a spawned task. When the frontend drops the channel
//! the forwarder ends, which propagates to the dispatcher as a `Closed`
//! send error on the next dispatch tick and the live handle is removed.
//!
//! SurfaceClient subscribers use the HTTP routes in `surface_runtime/mod.rs`,
//! not these commands.

use std::sync::Arc;

use tauri::ipc::Channel;
use tauri::State;

use crate::abilities::Actor;
use crate::services::version_dispatcher::{
    DispatchedEvent, ReplayRequest, ReplayResponse, SubjectFilter, SubscribeAck, SubscribeRequest,
};
use crate::state::AppState;

#[tauri::command]
pub async fn version_dispatcher_subscribe(
    request: SubscribeRequest,
    actor_kind: String,
    on_event: Channel<DispatchedEvent>,
    state: State<'_, Arc<AppState>>,
) -> Result<SubscribeAck, String> {
    let actor = match actor_kind.as_str() {
        "user" => Actor::User,
        "agent" => Actor::Agent,
        _ => return Err("actor_kind must be user|agent for tauri command path".to_string()),
    };

    // The bridge layer would normally pair an Actor::SurfaceClient binding;
    // here we route through a synthetic SurfaceClient instance bound to the
    // local Tauri session so the existing scope-permits-claim_read predicate
    // can authorize delivery. Tauri-internal callers carry implicit local
    // workspace scope; downstream surfaces can tighten this when User/Agent
    // grant tables land.
    let _ = actor;
    let actor_for_dispatch = build_synthetic_actor()?;

    let dispatcher = state.version_dispatcher.clone();
    let actor_for_subscribe = actor_for_dispatch.clone();
    let request_for_subscribe = request.clone();
    let (ack, mut rx, _bp_rx) = state
        .db_write(move |db| {
            dispatcher
                .subscribe(db, &request_for_subscribe, actor_for_subscribe)
                .map_err(|e| e.to_string())
        })
        .await?;
    let subscription_id_for_drop = ack.subscription_id.clone();
    let dispatcher_for_drop = state.version_dispatcher.clone();

    tauri::async_runtime::spawn(async move {
        while let Some(event) = rx.recv().await {
            if on_event.send(event).is_err() {
                break;
            }
        }
        dispatcher_for_drop.drop_handle(&subscription_id_for_drop);
    });

    Ok(ack)
}

#[tauri::command]
pub async fn version_dispatcher_replay(
    request: ReplayRequest,
    subjects: SubjectFilter,
    state: State<'_, Arc<AppState>>,
) -> Result<ReplayResponse, String> {
    let actor = build_synthetic_actor()?;
    let dispatcher = state.version_dispatcher.clone();
    state
        .db_write(move |db| {
            dispatcher
                .replay_stateless(db, &request, &actor, &subjects)
                .map_err(|e| e.to_string())
        })
        .await
}

/// Build a SurfaceClient-shaped actor for the native Tauri caller. The
/// dispatcher's scope predicate routes through `project_claim_for_scope`,
/// which requires an `Actor::SurfaceClient { scopes }` to authorize reads;
/// native callers carry implicit local-workspace scope today. When a real
/// native scope grant lands (post-WP cutover), this synthetic binding gets
/// replaced with the genuine User/Agent grant plumbed through the bridge.
fn build_synthetic_actor() -> Result<Actor, String> {
    use crate::abilities::registry::{ScopeSet, SurfaceClientId, SurfaceScope};
    let scope = SurfaceScope::new("read.account_overview");
    let scopes = ScopeSet::new([scope]).map_err(|e| format!("synthesize scope set: {e}"))?;
    let instance = SurfaceClientId::new("tauri-native-local");
    Ok(Actor::SurfaceClient { instance, scopes })
}
