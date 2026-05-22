---
title: Local loopback multisite blog ID invariant
problem_type: architecture_pattern
track: knowledge
module: wp-runtime-loopback
tags: [wordpress, multisite, loopback, trust-boundary, dos-761]
date: 2026-05-21
related_linear: DOS-761
---

# Local loopback multisite blog ID invariant

DailyOS is single-tenant per OS user. Runtime substrate routing is never selected by a WordPress blog ID.

The signed SurfaceClient path may carry `X-DailyOS-Multisite-Blog-Id` as part of the WordPress pairing/signing identity, but the substrate treats that value as decorative transport metadata. It is not an account selector, database selector, tenant selector, or claim-substrate routing key.

For first-party local loopback calls such as `POST /v1/local/invoke`, the runtime materializes `Actor::User` from the same-user loopback boundary. The request body and headers must not introduce a `blog_id` routing contract. The server-derived audit origin is `loopback_origin: "wp_plugin"`; WordPress multisite identity does not participate in local invoke authorization or substrate reads.

If DailyOS ever becomes multi-tenant per OS user, that product model needs an explicit tenant boundary and substrate contract. Do not infer one from `X-DailyOS-Multisite-Blog-Id`.
