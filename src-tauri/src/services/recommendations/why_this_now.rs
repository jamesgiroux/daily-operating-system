//! Deterministic `why this now` rationale construction.
//!
//! Given a scored candidate, returns a typed `WhyThisNow` payload
//! built from the primary factor and a closed set of triggers. The
//! render layer formats this for display; no string templating
//! happens here. No provider / LLM call from this module.
