<?php
/**
 * EvidenceDrawer primitive server-side render (DOS-689).
 *
 * 10-channel ADR-0130 §3.1 field allowlist (PHP-side enforcement on the
 * closed-state markup; view.js stays inside the same allowlist on reveal):
 *
 *   1. band_label         — TrustBand human label
 *   2. score_value        — Score band token (no raw numerics surface)
 *   3. factor_breakdown   — Factor names (band-coded; no raw scores)
 *   4. evidence_summary   — Short sanitized summary
 *   5. citation_list      — Source labels (human-readable; no IDs)
 *   6. freshness_caveat   — Relative age label
 *   7. trust_band         — Trust band token
 *   8. lifecycle_state    — Lifecycle token
 *   9. provenance_ref     — Actor-filtered render-projection only
 *  10. corrected_text     — Sanitized correction (no debug carriers)
 *
 * Disallowed (per DOS-689 AC, ADR-0108 actor-filtered render projection):
 *  - raw source-internal identifiers
 *  - email addresses
 *  - internal note bodies
 *  - debug carriers (request IDs, raw error payloads, internal flags)
 *
 * @package DailyOS
 */

declare(strict_types=1);

if ( ! defined( 'ABSPATH' ) ) {
	return '';
}

if ( ! function_exists( 'dailyos_evidence_drawer_render' ) ) {
	/**
	 * Render the EvidenceDrawer closed-state markup.
	 *
	 * @param array<string,mixed> $attributes Block attributes.
	 * @param array<string,mixed> $ctx        Block context (envelopeHandle).
	 * @return string Rendered HTML.
	 */
	function dailyos_evidence_drawer_render( array $attributes, array $ctx = [] ): string {
		$handle   = isset( $ctx['dailyos/envelopeHandle'] ) ? (string) $ctx['dailyos/envelopeHandle'] : '';
		$envelope = null;
		if ( '' !== $handle && function_exists( 'dailyos_envelope_cache_get' ) ) {
			$envelope = dailyos_envelope_cache_get( $handle );
		}

		$label = isset( $attributes['label'] ) ? (string) $attributes['label'] : '';
		if ( '' === $label ) {
			$label = __( 'Show evidence', 'dailyos' );
		}

		$wrapper_class = 'wp-block-dailyos-evidence-drawer dailyos-evidence-drawer';
		$wrapper_attrs = function_exists( 'get_block_wrapper_attributes' )
			? get_block_wrapper_attributes(
				[
					'class'                  => $wrapper_class,
					'data-ds-tier'           => 'primitive',
					'data-ds-name'           => 'EvidenceDrawer',
					'data-ds-spec'           => 'primitives/EvidenceDrawer.md',
					'data-envelope-handle'   => $handle,
					'data-open'              => 'false',
				]
			)
			: sprintf(
				'class="%s" data-ds-tier="primitive" data-ds-name="EvidenceDrawer" data-ds-spec="primitives/EvidenceDrawer.md" data-envelope-handle="%s" data-open="false"',
				esc_attr( $wrapper_class ),
				esc_attr( $handle )
			);

		$summary = dailyos_evidence_drawer_summary( $envelope );

		$toggle_id = 'dailyos-evidence-drawer-' . wp_generate_uuid4();
		$panel_id  = $toggle_id . '-panel';

		$out  = '<section ' . $wrapper_attrs . '>';
		$out .= sprintf(
			'<button type="button" class="dailyos-evidence-drawer__toggle" id="%s" aria-expanded="false" aria-controls="%s" data-action="toggle">%s</button>',
			esc_attr( $toggle_id ),
			esc_attr( $panel_id ),
			esc_html( $label )
		);
		$out .= sprintf(
			'<div class="dailyos-evidence-drawer__panel" id="%s" role="region" aria-labelledby="%s" hidden>',
			esc_attr( $panel_id ),
			esc_attr( $toggle_id )
		);

		if ( '' !== $summary['band_label'] ) {
			$out .= '<p class="dailyos-evidence-drawer__band"><span class="dailyos-evidence-drawer__band-label" data-channel="band_label">'
				. esc_html( $summary['band_label'] )
				. '</span></p>';
		}
		if ( '' !== $summary['evidence_summary'] ) {
			$out .= '<p class="dailyos-evidence-drawer__summary" data-channel="evidence_summary">'
				. esc_html( $summary['evidence_summary'] )
				. '</p>';
		}
		if ( ! empty( $summary['citations'] ) ) {
			$out .= '<ul class="dailyos-evidence-drawer__citations" data-channel="citation_list">';
			foreach ( $summary['citations'] as $citation ) {
				$out .= '<li class="dailyos-evidence-drawer__citation">'
					. esc_html( (string) $citation )
					. '</li>';
			}
			$out .= '</ul>';
		}
		if ( '' !== $summary['freshness_caveat'] ) {
			$out .= '<p class="dailyos-evidence-drawer__freshness" data-channel="freshness_caveat">'
				. esc_html( $summary['freshness_caveat'] )
				. '</p>';
		}
		if ( null === $envelope ) {
			$out .= '<span class="dailyos-empty-chip" data-empty-reason="no_envelope">'
				. esc_html__( 'No evidence available yet.', 'dailyos' )
				. '</span>';
		}

		$out .= '</div>';
		$out .= '</section>';

		return $out;
	}

	/**
	 * Project the envelope into the 10-channel allowlist summary used by
	 * the drawer's closed-state markup. Only allowlisted, display-safe
	 * fields are emitted.
	 *
	 * @param array<string,mixed>|null $envelope Envelope payload.
	 * @return array{
	 *   band_label:string,
	 *   evidence_summary:string,
	 *   citations:array<int,string>,
	 *   freshness_caveat:string
	 * }
	 */
	function dailyos_evidence_drawer_summary( ?array $envelope ): array {
		$out = [
			'band_label'       => '',
			'evidence_summary' => '',
			'citations'        => [],
			'freshness_caveat' => '',
		];
		if ( ! is_array( $envelope ) ) {
			return $out;
		}

		$band_labels = [
			'likely_current'     => __( 'Likely current', 'dailyos' ),
			'use_with_caution'   => __( 'Use with caution', 'dailyos' ),
			'needs_verification' => __( 'Needs verification', 'dailyos' ),
		];
		$band = '';
		if ( isset( $envelope['trust_band'] ) ) {
			$band = is_array( $envelope['trust_band'] ) && isset( $envelope['trust_band']['band'] )
				? (string) $envelope['trust_band']['band']
				: (string) $envelope['trust_band'];
		}
		if ( isset( $band_labels[ $band ] ) ) {
			$out['band_label'] = $band_labels[ $band ];
		}

		// evidence_summary — short sanitized description; reject if it looks
		// like an email address or raw source-internal identifier.
		if ( isset( $envelope['evidence_summary'] ) && is_string( $envelope['evidence_summary'] ) ) {
			$candidate = trim( $envelope['evidence_summary'] );
			if ( '' !== $candidate && ! dailyos_evidence_drawer_field_disallowed( $candidate ) ) {
				$out['evidence_summary'] = $candidate;
			}
		}

		// citation_list — source labels only (human-readable). Reject any
		// candidate containing raw IDs, emails, or note-body markers.
		$citations = [];
		if ( isset( $envelope['citations'] ) && is_array( $envelope['citations'] ) ) {
			foreach ( $envelope['citations'] as $citation ) {
				$label = '';
				if ( is_string( $citation ) ) {
					$label = $citation;
				} elseif ( is_array( $citation ) ) {
					if ( isset( $citation['label'] ) && is_string( $citation['label'] ) ) {
						$label = $citation['label'];
					} elseif ( isset( $citation['source'] ) && is_string( $citation['source'] ) ) {
						$label = $citation['source'];
					}
				}
				$label = trim( $label );
				if ( '' === $label || dailyos_evidence_drawer_field_disallowed( $label ) ) {
					continue;
				}
				$citations[] = $label;
			}
		}
		$out['citations'] = array_slice( $citations, 0, 8 );

		// freshness_caveat — short relative-age phrase.
		if ( isset( $envelope['source_asof'] ) && is_string( $envelope['source_asof'] ) ) {
			$asof = trim( $envelope['source_asof'] );
			if ( '' !== $asof ) {
				$ts = strtotime( $asof );
				if ( false !== $ts ) {
					$delta = time() - $ts;
					$out['freshness_caveat'] = $delta > 86400
						? sprintf(
							/* translators: %d: days */
							__( 'Sourced %d days ago', 'dailyos' ),
							(int) floor( $delta / 86400 )
						)
						: __( 'Sourced recently', 'dailyos' );
				}
			}
		}

		return $out;
	}

	/**
	 * Negative-fixture gate: reject candidates that look like disallowed
	 * channels (raw source-internal identifiers, email addresses, debug
	 * carriers). Conservative — false positives lose display labels, never
	 * leak sensitive content.
	 *
	 * @param string $candidate Field candidate.
	 * @return bool True if the candidate must be suppressed.
	 */
	function dailyos_evidence_drawer_field_disallowed( string $candidate ): bool {
		$normalized = strtolower( trim( $candidate ) );
		if ( '' === $normalized ) {
			return false;
		}
		// Email-shaped.
		if ( preg_match( '/[a-z0-9._%+\-]+@[a-z0-9.\-]+\.[a-z]{2,}/i', $normalized ) ) {
			return true;
		}
		// Raw source-internal identifier prefixes (UUIDs, urn:, src:internal:).
		if ( preg_match( '/^(urn:|src:internal:|sfid:|gid:|uid:)/', $normalized ) ) {
			return true;
		}
		if ( preg_match( '/^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$/', $normalized ) ) {
			return true;
		}
		// Debug carriers.
		if ( false !== strpos( $normalized, 'debug_' ) || false !== strpos( $normalized, 'request_id=' ) ) {
			return true;
		}
		// Internal-note body markers.
		if ( 0 === strpos( $normalized, 'internal note:' ) || 0 === strpos( $normalized, 'internal-note:' ) ) {
			return true;
		}
		return false;
	}
}
