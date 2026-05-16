use std::collections::BTreeMap;
use std::time::Duration;

use chrono::{DateTime, Duration as ChronoDuration, Timelike, Utc};
use serde::Serialize;

use crate::signals::SignalType;

use super::{CatalogName, MetricDimensions, MetricValue, Outcome};

const BUFFER_WINDOW_HOURS: i64 = 24;

#[derive(Debug, Clone)]
pub struct BufferedAggregateMetric {
    pub metric_name: &'static str,
    pub metric_value: MetricValue,
    pub ability_name: Option<&'static str>,
    pub ability_version: Option<&'static str>,
    pub signal_type: Option<SignalType>,
    pub outcome: Option<Outcome>,
    pub bucket_start: DateTime<Utc>,
    pub build_version: &'static str,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AggregateMetricPreview {
    pub metric_name: &'static str,
    pub metric_value: MetricValuePreview,
    pub ability_name: Option<&'static str>,
    pub ability_version: Option<&'static str>,
    pub signal_type: Option<String>,
    pub outcome: Option<Outcome>,
    pub bucket_start: DateTime<Utc>,
    pub build_version: &'static str,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum MetricValuePreview {
    Count {
        value: u64,
    },
    Duration {
        #[serde(rename = "valueMs")]
        value_ms: u64,
    },
    Percentile {
        quantile: String,
        #[serde(rename = "valueMs")]
        value_ms: u64,
    },
    Boolean {
        value: bool,
    },
}

#[derive(Debug, Default)]
pub struct AggregateMetricBuffer {
    buckets: BTreeMap<MetricBucketKey, BufferedAggregateMetric>,
}

impl AggregateMetricBuffer {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn record(
        &mut self,
        metric_name: CatalogName,
        metric_value: MetricValue,
        dimensions: MetricDimensions,
        now: DateTime<Utc>,
        build_version: &'static str,
    ) {
        let bucket_start = hour_bucket_start(now);
        let key = MetricBucketKey::new(
            metric_name.as_str(),
            &metric_value,
            &dimensions,
            bucket_start,
        );
        self.buckets
            .entry(key)
            .and_modify(|existing| merge_metric_value(&mut existing.metric_value, &metric_value))
            .or_insert_with(|| BufferedAggregateMetric {
                metric_name: metric_name.as_str(),
                metric_value,
                ability_name: dimensions.ability_name,
                ability_version: dimensions.ability_version,
                signal_type: dimensions.signal_type,
                outcome: dimensions.outcome,
                bucket_start,
                build_version,
            });
        self.drop_older_than(now);
    }

    pub fn drain_ready(&mut self, now: DateTime<Utc>) -> Vec<BufferedAggregateMetric> {
        let current_bucket = hour_bucket_start(now);
        let ready_keys = self
            .buckets
            .keys()
            .filter(|key| key.bucket_start < current_bucket)
            .cloned()
            .collect::<Vec<_>>();
        let mut ready = Vec::with_capacity(ready_keys.len());
        for key in ready_keys {
            if let Some(metric) = self.buckets.remove(&key) {
                ready.push(metric);
            }
        }
        self.drop_older_than(now);
        ready
    }

    pub fn requeue(&mut self, metrics: Vec<BufferedAggregateMetric>, now: DateTime<Utc>) {
        for metric in metrics {
            let key = MetricBucketKey::from_metric(&metric);
            self.buckets
                .entry(key)
                .and_modify(|existing| {
                    merge_metric_value(&mut existing.metric_value, &metric.metric_value)
                })
                .or_insert(metric);
        }
        self.drop_older_than(now);
    }

    pub fn preview(&self, now: DateTime<Utc>) -> Vec<AggregateMetricPreview> {
        let cutoff = hour_bucket_start(now) - ChronoDuration::hours(BUFFER_WINDOW_HOURS - 1);
        self.buckets
            .values()
            .filter(|metric| metric.bucket_start >= cutoff)
            .map(BufferedAggregateMetric::to_preview)
            .collect()
    }

    pub fn clear(&mut self) {
        self.buckets.clear();
    }

    pub fn len(&self) -> usize {
        self.buckets.len()
    }

    pub fn is_empty(&self) -> bool {
        self.buckets.is_empty()
    }

    fn drop_older_than(&mut self, now: DateTime<Utc>) {
        let cutoff = hour_bucket_start(now) - ChronoDuration::hours(BUFFER_WINDOW_HOURS - 1);
        self.buckets.retain(|key, _| key.bucket_start >= cutoff);
    }
}

impl BufferedAggregateMetric {
    fn to_preview(&self) -> AggregateMetricPreview {
        AggregateMetricPreview {
            metric_name: self.metric_name,
            metric_value: MetricValuePreview::from(&self.metric_value),
            ability_name: self.ability_name,
            ability_version: self.ability_version,
            signal_type: self
                .signal_type
                .as_ref()
                .map(|signal_type| signal_type.canonical_name().to_string()),
            outcome: self.outcome,
            bucket_start: self.bucket_start,
            build_version: self.build_version,
        }
    }
}

impl From<&MetricValue> for MetricValuePreview {
    fn from(value: &MetricValue) -> Self {
        match value {
            MetricValue::Count(value) => Self::Count { value: *value },
            MetricValue::Duration(duration) => Self::Duration {
                value_ms: duration_millis_u64(*duration),
            },
            MetricValue::Percentile { quantile, value_ms } => Self::Percentile {
                quantile: format!("{quantile:.2}"),
                value_ms: *value_ms,
            },
            MetricValue::Boolean(value) => Self::Boolean { value: *value },
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct MetricBucketKey {
    metric_name: &'static str,
    value_kind: MetricValueKind,
    percentile_quantile: Option<u32>,
    ability_name: Option<&'static str>,
    ability_version: Option<&'static str>,
    signal_type: Option<String>,
    outcome: Option<Outcome>,
    bucket_start: DateTime<Utc>,
    build_version: &'static str,
}

impl MetricBucketKey {
    fn new(
        metric_name: &'static str,
        metric_value: &MetricValue,
        dimensions: &MetricDimensions,
        bucket_start: DateTime<Utc>,
    ) -> Self {
        Self {
            metric_name,
            value_kind: MetricValueKind::from(metric_value),
            percentile_quantile: percentile_quantile(metric_value),
            ability_name: dimensions.ability_name,
            ability_version: dimensions.ability_version,
            signal_type: dimensions
                .signal_type
                .as_ref()
                .map(|signal_type| signal_type.canonical_name().to_string()),
            outcome: dimensions.outcome,
            bucket_start,
            build_version: super::BUILD_VERSION,
        }
    }

    fn from_metric(metric: &BufferedAggregateMetric) -> Self {
        Self {
            metric_name: metric.metric_name,
            value_kind: MetricValueKind::from(&metric.metric_value),
            percentile_quantile: percentile_quantile(&metric.metric_value),
            ability_name: metric.ability_name,
            ability_version: metric.ability_version,
            signal_type: metric
                .signal_type
                .as_ref()
                .map(|signal_type| signal_type.canonical_name().to_string()),
            outcome: metric.outcome,
            bucket_start: metric.bucket_start,
            build_version: metric.build_version,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum MetricValueKind {
    Count,
    Duration,
    Percentile,
    Boolean,
}

impl From<&MetricValue> for MetricValueKind {
    fn from(value: &MetricValue) -> Self {
        match value {
            MetricValue::Count(_) => Self::Count,
            MetricValue::Duration(_) => Self::Duration,
            MetricValue::Percentile { .. } => Self::Percentile,
            MetricValue::Boolean(_) => Self::Boolean,
        }
    }
}

fn merge_metric_value(existing: &mut MetricValue, incoming: &MetricValue) {
    match (existing, incoming) {
        (MetricValue::Count(left), MetricValue::Count(right)) => {
            *left = left.saturating_add(*right);
        }
        (MetricValue::Duration(left), MetricValue::Duration(right)) => {
            *left = left.saturating_add(*right);
        }
        (
            MetricValue::Percentile {
                value_ms: left_ms, ..
            },
            MetricValue::Percentile {
                value_ms: right_ms, ..
            },
        ) => {
            *left_ms = (*left_ms).max(*right_ms);
        }
        (MetricValue::Boolean(left), MetricValue::Boolean(right)) => {
            *left = *left || *right;
        }
        _ => {}
    }
}

fn percentile_quantile(value: &MetricValue) -> Option<u32> {
    match value {
        MetricValue::Percentile { quantile, .. } => Some((quantile * 100.0).round() as u32),
        _ => None,
    }
}

pub fn hour_bucket_start(now: DateTime<Utc>) -> DateTime<Utc> {
    now.with_minute(0)
        .and_then(|dt| dt.with_second(0))
        .and_then(|dt| dt.with_nanosecond(0))
        .expect("valid UTC hour bucket")
}

fn duration_millis_u64(duration: Duration) -> u64 {
    duration.as_millis().min(u128::from(u64::MAX)) as u64
}

#[cfg(test)]
mod tests {
    use chrono::TimeZone;

    use super::*;
    #[test]
    fn counts_aggregate_into_the_same_hourly_bucket() {
        let mut buffer = AggregateMetricBuffer::new();
        let now = Utc.with_ymd_and_hms(2026, 5, 16, 14, 20, 0).unwrap();

        buffer.record(
            crate::aggregate_metric_name!("ability_invocation_count"),
            MetricValue::Count(1),
            MetricDimensions::default(),
            now,
            super::super::BUILD_VERSION,
        );
        buffer.record(
            crate::aggregate_metric_name!("ability_invocation_count"),
            MetricValue::Count(2),
            MetricDimensions::default(),
            now + ChronoDuration::minutes(10),
            super::super::BUILD_VERSION,
        );

        let preview = buffer.preview(now);
        assert_eq!(preview.len(), 1);
        assert_eq!(
            preview[0].metric_value,
            MetricValuePreview::Count { value: 3 }
        );
    }

    #[test]
    fn local_buffer_retains_only_the_last_twenty_four_hours() {
        let mut buffer = AggregateMetricBuffer::new();
        let start = Utc.with_ymd_and_hms(2026, 5, 15, 0, 0, 0).unwrap();

        for hour in 0..25 {
            buffer.record(
                crate::aggregate_metric_name!("correction_rate"),
                MetricValue::Count(1),
                MetricDimensions::default(),
                start + ChronoDuration::hours(hour),
                super::super::BUILD_VERSION,
            );
        }

        let preview = buffer.preview(start + ChronoDuration::hours(24));
        assert_eq!(preview.len(), 24);
        assert_eq!(preview[0].bucket_start, start + ChronoDuration::hours(1));
    }

    #[test]
    fn drain_ready_keeps_the_current_hour_open() {
        let mut buffer = AggregateMetricBuffer::new();
        let first = Utc.with_ymd_and_hms(2026, 5, 16, 13, 5, 0).unwrap();
        let second = Utc.with_ymd_and_hms(2026, 5, 16, 14, 5, 0).unwrap();

        buffer.record(
            crate::aggregate_metric_name!("ability_invocation_count"),
            MetricValue::Count(1),
            MetricDimensions::default(),
            first,
            super::super::BUILD_VERSION,
        );
        buffer.record(
            crate::aggregate_metric_name!("ability_invocation_count"),
            MetricValue::Count(1),
            MetricDimensions::default(),
            second,
            super::super::BUILD_VERSION,
        );

        let drained = buffer.drain_ready(second);
        assert_eq!(drained.len(), 1);
        assert_eq!(drained[0].bucket_start, hour_bucket_start(first));
        assert_eq!(buffer.len(), 1);
    }
}
