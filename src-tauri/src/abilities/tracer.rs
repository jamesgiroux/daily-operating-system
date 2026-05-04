//! Ability tracing seam.
//!
//! The production tracer wiring lands later. This module provides the shared
//! trait that abilities can depend on now plus a no-op implementation for
//! bridge paths that do not have a concrete sink yet.

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpanHandle {
    pub id: u64,
}

impl SpanHandle {
    pub const fn noop() -> Self {
        Self { id: 0 }
    }
}

pub trait AbilityTracer: Send + Sync {
    fn start_span(&self, name: &str) -> SpanHandle;
    fn record_event(&self, span: &SpanHandle, name: &str, fields: serde_json::Value);
}

#[derive(Debug, Default)]
pub struct NoopAbilityTracer;

impl AbilityTracer for NoopAbilityTracer {
    fn start_span(&self, _name: &str) -> SpanHandle {
        SpanHandle::noop()
    }

    fn record_event(&self, _span: &SpanHandle, _name: &str, _fields: serde_json::Value) {}
}

pub static NOOP_ABILITY_TRACER: NoopAbilityTracer = NoopAbilityTracer;
