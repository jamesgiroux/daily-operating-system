# ADR-0104 Amendment: Phased Enrichment Persist Barrier

Status: Accepted

## Context

ADR-0104 defines execution-mode-aware services, and ADR-0133 section 4 keeps writer-queue transaction shape under the caller's control. The database throughput W1-A guidance calls for row-chunked caller-side transactions when a workload can grow large enough to occupy the single writer for user-visible time.

Enrichment persistence violated that shape by holding one transaction across cleared-dimension withdrawal, projection claim commits, legacy snapshot upsert, signal emission, objective reconciliation, and recompute enqueue. Large generated projection payloads could therefore occupy the writer queue while unrelated composition work waited behind the entire persist.

## Decision

Queue-driven enrichment persistence uses phase-ordered writer tasks:

1. Begin: withdraw claims for dimensions that the current enrichment explicitly cleared.
2. Projection batches: commit claim-shaped projection rows in bounded batches.
3. Finalizer: withdraw stale refreshed projection claims, upsert the legacy entity intelligence snapshot, emit the update signal, reconcile objectives, and enqueue recompute work.

The finalizer is the only phase that publishes the visible assessment row, signals, and recompute side effects. Surfaces that read entity intelligence therefore never observe a partially published assessment from a crashed or interrupted batch sequence.

## Idempotency

Projection claim commits remain rerunnable because the claim substrate supersedes by field path, source identity, and semantic claim key. If a process stops after batch `k`, the next run may encounter already committed generated claims; the rerun either reuses exact active claims, supersedes prior generated claims, or withdraws stale refreshed projection claims during the finalizer.

This preserves the ADR-0133 section 4 contract: the writer queue serializes tasks, but callers shape transaction boundaries. It also follows W1-A's row-chunking guidance for large write workloads while retaining a finalizer barrier for coherent publication.
