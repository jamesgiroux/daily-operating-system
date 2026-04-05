# I347 — SWOT Report Type — Account Analysis from Existing Intelligence

**Status:** Absorbed into I397
**Priority:** —
**Version:** 0.14.0

## Summary

I347 described a SWOT report generated from existing entity intelligence as a format within Report Mode.

**This issue is absorbed into I396** (Report Infrastructure). SWOT ships as one of the three report types bundled with the infrastructure (alongside Account Health Review and EBR/QBR). See I396 acceptance criterion 9 for the SWOT-specific requirements.

The SWOT synthesis approach: active positive signals → Strengths, active risk/warning signals → Weaknesses and Threats (distinction: weaknesses are internal/controllable, threats are external), identified opportunities → Opportunities. One AI synthesis call, stored in `reports` table, consumed mechanically on subsequent views.
