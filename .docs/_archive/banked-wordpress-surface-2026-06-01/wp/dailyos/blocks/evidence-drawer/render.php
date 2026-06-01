<?php
/**
 * EvidenceDrawer primitive dynamic block render entrypoint (DOS-689).
 *
 * Per L0 packet W2 V1.2.1 §5.7 DOS-689:
 *  - Renders the closed-state drawer markup; view.js handles open/close +
 *    lazy evidence reveal from envelope provenance (actor-filtered per
 *    ADR-0108).
 *
 * ADR-0130 §3.1 10-channel field allowlist enforcement (NEGATIVE — no
 * raw source-internal identifiers, no email addresses, no internal note
 * bodies, no debug carriers).
 *
 * @package DailyOS
 *
 * @var array<string, mixed> $attributes Block attributes from core.
 * @var array<string, mixed> $block      Block instance (carries context).
 */

declare(strict_types=1);

if ( ! defined( 'ABSPATH' ) ) {
	return '';
}

if ( ! function_exists( 'dailyos_evidence_drawer_render' ) ) {
	require_once __DIR__ . '/render-functions.php';
}

$attributes = isset( $attributes ) && is_array( $attributes ) ? $attributes : [];
$ctx        = [];
if ( isset( $block ) && is_object( $block ) && isset( $block->context ) && is_array( $block->context ) ) {
	$ctx = $block->context;
}
// phpcs:ignore WordPress.Security.EscapeOutput.OutputNotEscaped -- render function escapes internally.
echo dailyos_evidence_drawer_render( $attributes, $ctx );
