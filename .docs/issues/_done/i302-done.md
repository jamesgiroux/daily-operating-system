# I302 — Shareable PDF Export for Intelligence Reports (Editorial-Styled)

**Status:** Superseded
**Superseded by:** I397 (Report Infrastructure)
**Priority:** —
**Version:** 0.14.0

## Summary

I302 described a PDF export capability for entity intelligence reports using the existing `export_briefing_html` path and print CSS infrastructure from Sprint 25.

**This issue is superseded by I397**, which includes PDF export as part of the shared `ReportShell` renderer. The existing `export_briefing_html` command and print CSS foundation identified in I302 are exactly what I396 builds on.

## Foundation Still Used

The infrastructure I302 identified is carried forward:
- `export_briefing_html` Tauri command (added v0.8.2) — used by I397
- Print CSS from Sprint 25 — extended for report-specific layout in I396
- Offline rendering (no cloud service) — preserved in I396
