# ADR-0002: Frontend-first implementation

**Date:** 2024-01
**Status:** Accepted

## Context

Building a full-stack app with a new framework (Tauri). Need to validate data shapes and UX before investing in backend infrastructure.

## Decision

Build UI with mock data first, then wire real backend. Phases go: static dashboard → file reading → scheduler → archive.

## Consequences

- Reveals data shape requirements before backend code is written
- Can demo and get feedback on UX immediately
- Risk: mock data may not reflect real-world complexity
- The frontend effectively becomes the spec for the backend API
