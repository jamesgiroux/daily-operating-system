pub mod buffer;
pub mod emitter;
pub mod install_id;
pub mod lint;

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;

use chrono::{DateTime, Utc};
use parking_lot::Mutex;
use serde::ser::SerializeStruct;
use serde::{Serialize, Serializer};
use thiserror::Error;

use crate::signals::policy_registry::known_signal_type_names;
use crate::signals::SignalType;

pub use buffer::{AggregateMetricBuffer, AggregateMetricPreview};
pub use emitter::{collector_url, emit, HttpsUrl, PRODUCTION_COLLECTOR_URL};
pub use install_id::{anon_id_path, default_app_support_dir, AnonInstallId, InstallIdError};

pub const BUILD_VERSION: &str = env!("CARGO_PKG_VERSION");
pub const AGGREGATE_METRIC_CATALOG: &[&str] = &[
    "correction_rate",
    "ability_invocation_count",
    "glean_availability_pct",
    "ghost_resurrection_incidents",
    "eval_replay_match_pct",
];

pub struct AggregateMetric {
    pub anon_install_id: AnonInstallId,
    pub metric_name: &'static str,
    pub metric_value: MetricValue,
    pub ability_name: Option<&'static str>,
    pub ability_version: Option<&'static str>,
    pub signal_type: Option<SignalType>,
    pub outcome: Option<Outcome>,
    pub bucket_start: DateTime<Utc>,
    pub build_version: &'static str,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum MetricValue {
    Count(u64),
    Duration(Duration),
    Percentile { quantile: f32, value_ms: u64 },
    Boolean(bool),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Outcome {
    Success,
    Failure,
    Skipped,
    Timeout,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct CatalogName {
    value: &'static str,
}

impl CatalogName {
    pub const fn as_str(self) -> &'static str {
        self.value
    }
}

#[derive(Debug, Default, Clone)]
pub struct MetricDimensions {
    pub ability_name: Option<&'static str>,
    pub ability_version: Option<&'static str>,
    pub signal_type: Option<SignalType>,
    pub outcome: Option<Outcome>,
}

impl MetricDimensions {
    pub fn ability(mut self, name: &'static str, version: &'static str) -> Self {
        self.ability_name = Some(name);
        self.ability_version = Some(version);
        self
    }

    pub fn signal_type(mut self, signal_type: SignalType) -> Self {
        self.signal_type = sanitize_signal_type(signal_type);
        self
    }

    pub fn outcome(mut self, outcome: Outcome) -> Self {
        self.outcome = Some(outcome);
        self
    }
}

#[derive(Debug, Error)]
pub enum AggregateTelemetryError {
    #[error(transparent)]
    InstallId(#[from] InstallIdError),
    #[error(transparent)]
    CollectorUrl(#[from] emitter::HttpsUrlError),
    #[error(transparent)]
    Emit(#[from] emitter::EmitError),
}

pub struct AggregateTelemetry {
    enabled: AtomicBool,
    pre_opt_in_buffering: AtomicBool,
    install_id: Mutex<Option<AnonInstallId>>,
    buffer: Mutex<AggregateMetricBuffer>,
}

impl AggregateTelemetry {
    pub fn new(config_enabled: bool) -> Self {
        let existing_install_id = match AnonInstallId::load_existing() {
            Ok(existing) => existing,
            Err(err) => {
                log::warn!("aggregate telemetry disabled because anon_id could not load: {err}");
                None
            }
        };
        let install_id = if config_enabled {
            existing_install_id
        } else {
            None
        };
        let pre_opt_in_buffering = !config_enabled && existing_install_id.is_none();

        Self {
            enabled: AtomicBool::new(config_enabled && install_id.is_some()),
            pre_opt_in_buffering: AtomicBool::new(pre_opt_in_buffering),
            install_id: Mutex::new(install_id),
            buffer: Mutex::new(AggregateMetricBuffer::new()),
        }
    }

    pub fn is_enabled(&self) -> bool {
        self.enabled.load(Ordering::SeqCst)
    }

    pub fn enable_with_install_id(&self, install_id: AnonInstallId) {
        *self.install_id.lock() = Some(install_id);
        self.pre_opt_in_buffering.store(false, Ordering::SeqCst);
        self.enabled.store(true, Ordering::SeqCst);
    }

    pub fn disable_and_drop_pending(&self) {
        self.enabled.store(false, Ordering::SeqCst);
        self.pre_opt_in_buffering.store(false, Ordering::SeqCst);
        *self.install_id.lock() = None;
        self.buffer.lock().clear();
    }

    pub fn record(
        &self,
        metric_name: CatalogName,
        metric_value: MetricValue,
        dimensions: MetricDimensions,
    ) {
        if !self.is_enabled() && !self.pre_opt_in_buffering.load(Ordering::SeqCst) {
            return;
        }
        self.buffer.lock().record(
            metric_name,
            metric_value,
            dimensions,
            Utc::now(),
            BUILD_VERSION,
        );
    }

    pub fn preview(&self) -> Vec<AggregateMetricPreview> {
        self.buffer.lock().preview(Utc::now())
    }

    pub fn pending_len(&self) -> usize {
        self.buffer.lock().len()
    }

    pub async fn flush_ready(&self) -> Result<(), AggregateTelemetryError> {
        if !self.is_enabled() {
            return Ok(());
        }
        let install_id = match *self.install_id.lock() {
            Some(id) => id,
            None => return Ok(()),
        };
        let now = Utc::now();
        let ready = self.buffer.lock().drain_ready(now);
        if ready.is_empty() {
            return Ok(());
        }
        if !self.is_enabled() {
            self.buffer.lock().requeue(ready, Utc::now());
            return Ok(());
        }
        let metrics = ready
            .iter()
            .map(|metric| AggregateMetric {
                anon_install_id: install_id,
                metric_name: metric.metric_name,
                metric_value: metric.metric_value,
                ability_name: metric.ability_name,
                ability_version: metric.ability_version,
                signal_type: metric.signal_type.clone(),
                outcome: metric.outcome,
                bucket_start: metric.bucket_start,
                build_version: metric.build_version,
            })
            .collect::<Vec<_>>();
        let url = collector_url()?;
        if let Err(err) = emitter::emit(&metrics, &url).await {
            if self.is_enabled() {
                self.buffer.lock().requeue(ready, Utc::now());
            }
            return Err(err.into());
        }
        Ok(())
    }
}

pub fn emit_aggregate_metric(
    state: &crate::state::AppState,
    metric_name: CatalogName,
    metric_value: MetricValue,
    dimensions: MetricDimensions,
) {
    state
        .aggregate_telemetry
        .record(metric_name, metric_value, dimensions);
}

pub async fn run_emission_loop(telemetry: Arc<AggregateTelemetry>) {
    let mut interval = tokio::time::interval(Duration::from_secs(60));
    loop {
        interval.tick().await;
        if let Err(err) = telemetry.flush_ready().await {
            log::warn!("aggregate telemetry flush failed: {err}");
        }
    }
}

impl Serialize for AggregateMetric {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut state = serializer.serialize_struct("AggregateMetric", 9)?;
        state.serialize_field("anon_install_id", &self.anon_install_id)?;
        state.serialize_field("metric_name", self.metric_name)?;
        state.serialize_field("metric_value", &SerializableMetricValue(self.metric_value))?;
        state.serialize_field("ability_name", &self.ability_name)?;
        state.serialize_field("ability_version", &self.ability_version)?;
        state.serialize_field(
            "signal_type",
            &self
                .signal_type
                .as_ref()
                .map(|signal_type| signal_type.canonical_name()),
        )?;
        state.serialize_field("outcome", &self.outcome)?;
        state.serialize_field("bucket_start", &self.bucket_start)?;
        state.serialize_field("build_version", self.build_version)?;
        state.end()
    }
}

struct SerializableMetricValue(MetricValue);

impl Serialize for SerializableMetricValue {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match self.0 {
            MetricValue::Count(value) => {
                let mut state = serializer.serialize_struct("MetricValue", 2)?;
                state.serialize_field("type", "count")?;
                state.serialize_field("value", &value)?;
                state.end()
            }
            MetricValue::Duration(duration) => {
                let mut state = serializer.serialize_struct("MetricValue", 2)?;
                state.serialize_field("type", "duration")?;
                state.serialize_field("value_ms", &duration_millis_u64(duration))?;
                state.end()
            }
            MetricValue::Percentile { quantile, value_ms } => {
                let mut state = serializer.serialize_struct("MetricValue", 3)?;
                state.serialize_field("type", "percentile")?;
                state.serialize_field("quantile", &quantile)?;
                state.serialize_field("value_ms", &value_ms)?;
                state.end()
            }
            MetricValue::Boolean(value) => {
                let mut state = serializer.serialize_struct("MetricValue", 2)?;
                state.serialize_field("type", "boolean")?;
                state.serialize_field("value", &value)?;
                state.end()
            }
        }
    }
}

#[doc(hidden)]
pub mod __catalog_macro_support {
    use super::{catalog_contains, CatalogName};

    pub const fn assert_catalog_name(name: &'static str) {
        if !catalog_contains(name) {
            panic!("aggregate metric name is not in AGGREGATE_METRIC_CATALOG");
        }
    }

    pub const fn catalog_name(name: &'static str) -> CatalogName {
        CatalogName { value: name }
    }
}

#[macro_export]
macro_rules! aggregate_metric_name {
    ($name:literal) => {{
        const _: () =
            $crate::observability::aggregate_metric::__catalog_macro_support::assert_catalog_name(
                $name,
            );
        $crate::observability::aggregate_metric::__catalog_macro_support::catalog_name($name)
    }};
}

pub const fn catalog_contains(name: &str) -> bool {
    let mut i = 0;
    while i < AGGREGATE_METRIC_CATALOG.len() {
        if str_eq(name.as_bytes(), AGGREGATE_METRIC_CATALOG[i].as_bytes()) {
            return true;
        }
        i += 1;
    }
    false
}

const fn str_eq(left: &[u8], right: &[u8]) -> bool {
    if left.len() != right.len() {
        return false;
    }
    let mut i = 0;
    while i < left.len() {
        if left[i] != right[i] {
            return false;
        }
        i += 1;
    }
    true
}

fn sanitize_signal_type(signal_type: SignalType) -> Option<SignalType> {
    let name = signal_type.canonical_name();
    if known_signal_type_names().contains(&name) {
        Some(SignalType::from_name(name))
    } else {
        None
    }
}

fn duration_millis_u64(duration: Duration) -> u64 {
    duration.as_millis().min(u128::from(u64::MAX)) as u64
}

#[cfg(test)]
mod tests {
    use chrono::TimeZone;

    use super::*;
    fn fixture_install_id() -> AnonInstallId {
        let dir = tempfile::tempdir().expect("tempdir");
        AnonInstallId::generate_on_opt_in_in(dir.path()).expect("install ID")
    }

    #[test]
    fn aggregate_metric_struct_shape_matches_adr_0120_section_10() {
        let metric = AggregateMetric {
            anon_install_id: fixture_install_id(),
            metric_name: crate::aggregate_metric_name!("correction_rate").as_str(),
            metric_value: MetricValue::Count(1),
            ability_name: Some("prepare_meeting"),
            ability_version: Some("1.0.0"),
            signal_type: None,
            outcome: Some(Outcome::Success),
            bucket_start: Utc.with_ymd_and_hms(2026, 5, 16, 12, 0, 0).unwrap(),
            build_version: BUILD_VERSION,
        };

        let _: AnonInstallId = metric.anon_install_id;
        let _: &'static str = metric.metric_name;
        let _: MetricValue = metric.metric_value;
        let _: Option<&'static str> = metric.ability_name;
        let _: Option<&'static str> = metric.ability_version;
        let _: Option<SignalType> = metric.signal_type;
        let _: Option<Outcome> = metric.outcome;
        let _: DateTime<Utc> = metric.bucket_start;
        let _: &'static str = metric.build_version;
    }

    #[test]
    fn initial_catalog_has_exactly_the_five_dos260_metrics() {
        assert_eq!(
            AGGREGATE_METRIC_CATALOG,
            &[
                "correction_rate",
                "ability_invocation_count",
                "glean_availability_pct",
                "ghost_resurrection_incidents",
                "eval_replay_match_pct",
            ]
        );
    }

    #[test]
    fn metric_value_serialization_has_no_free_text_payload_variant() {
        let metric = AggregateMetric {
            anon_install_id: fixture_install_id(),
            metric_name: crate::aggregate_metric_name!("ability_invocation_count").as_str(),
            metric_value: MetricValue::Duration(Duration::from_millis(42)),
            ability_name: None,
            ability_version: None,
            signal_type: None,
            outcome: Some(Outcome::Success),
            bucket_start: Utc.with_ymd_and_hms(2026, 5, 16, 12, 0, 0).unwrap(),
            build_version: BUILD_VERSION,
        };

        let serialized = serde_json::to_value(metric).expect("serialize metric");
        assert_eq!(serialized["metric_value"]["type"], "duration");
        assert_eq!(serialized["metric_value"]["value_ms"], 42);
        assert!(serialized.get("entity_id").is_none());
        assert!(serialized.get("claim_text").is_none());
        assert!(serialized.get("content_hash").is_none());
        assert!(serialized.get("invocation_id").is_none());
    }

    #[test]
    fn disabling_telemetry_drops_pending_buffer_without_flush() {
        // Construct disabled then explicitly enable with a fixture install_id so the
        // test is hermetic — `new(true)` reads from the user's macOS app-support dir
        // via `AnonInstallId::load_existing()`, which makes the test outcome depend
        // on host state (whether DailyOS was previously opted in on this machine).
        let telemetry = AggregateTelemetry::new(false);
        telemetry.enable_with_install_id(fixture_install_id());

        telemetry.record(
            crate::aggregate_metric_name!("ability_invocation_count"),
            MetricValue::Count(1),
            MetricDimensions::default(),
        );
        assert_eq!(telemetry.pending_len(), 1);

        telemetry.disable_and_drop_pending();

        assert!(!telemetry.is_enabled());
        assert_eq!(telemetry.pending_len(), 0);

        telemetry.record(
            crate::aggregate_metric_name!("ability_invocation_count"),
            MetricValue::Count(1),
            MetricDimensions::default(),
        );
        assert_eq!(telemetry.pending_len(), 0);
    }

    #[test]
    fn legacy_signal_type_payload_is_not_carried_into_dimensions() {
        let dimensions = MetricDimensions::default().signal_type(SignalType::Legacy {
            name: "free text".into(),
        });
        assert!(dimensions.signal_type.is_none());
    }
}
