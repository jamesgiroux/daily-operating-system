# I258 — Report Mode — Export Account Detail as Leadership-Ready Deck/PDF

**Status:** Superseded
**Superseded by:** I397 (Report Infrastructure)
**Priority:** —
**Version:** 0.14.0

## Summary

I258 described a "Report Mode" toggle on the account detail page that reformatted intelligence into a leadership-ready export. The concept was correct but underspecified the architecture.

**This issue is superseded by I397**, which defines the complete report architecture: a `reports` DB table, intel_hash-based cache invalidation, shared `ReportShell` renderer, and PDF export. I397 implements everything I258 described and more.

The PDF export aspect was separately tracked in I302, which is also superseded by I397.

## What Was Absorbed

The core ideas from I258 that were carried forward into I397:
- Report content comes from existing `entity_intel` — no new AI call on view
- Inline editing of report content before export (draft state only, not persisted)
- PDF export produces an editorial-styled document
- Report Mode is accessible from the account detail page

What changed: I258 conceived this as a "mode toggle" on the existing account detail layout. I397 defines it as a separate report surface (`/accounts/:id/report/:type`) with a dedicated `ReportShell` renderer and a proper DB-backed caching layer.
