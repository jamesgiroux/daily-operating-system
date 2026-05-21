<?php
/**
 * Metadata Proposal Cue (dailyos/metadata-proposal-cue) — W2 L1 inner block render-functions.
 *
 * Per L0-packet-W2-entity-surfaces.md V1.2.1 §5.6 (DOS-328). Projects from
 * `EntityIntelligenceEnvelope.metadata_proposals` and renders a calm
 * peripheral cue summarising unresolved proposals on the active entity.
 * The expanded accept/dismiss/edit affordance lives in
 * `dailyos/metadata-proposal-drawer`.
 *
 * Projection rule (§5.6): one cue per active subject; surfaces count of
 * unresolved proposals + dominant trust band; suppresses when the audience
 * is AgentMcp (aggregate-only) or when no proposals are present (empty
 * chip with data-empty-reason per §10 invariant).
 *
 * Trust-band suppression (AC-328.1): `needs_verification` band proposals
 * are folded into the count but the cue label remains calm (no nag).
 *
 * Inner blocks declare usesContext for dailyos/envelopeHandle and consume
 * the cached envelope via dailyos_resolve_envelope(); short-circuits to
 * the outer block's single producer invocation.
 *
 * @package DailyOS
 *
 * @var array<string, mixed> $attributes Block attributes (provided by core).
 * @var string               $content    Inner content (empty for dynamic blocks).
 * @var \WP_Block|null       $block      Parsed block (carries usesContext).
 */

declare(strict_types=1);

if ( ! defined( 'ABSPATH' ) ) {
	return '';
}

if ( ! function_exists( 'dailyos_resolve_envelope' ) ) {
	require_once dirname( __DIR__, 2 ) . '/blocks/_shared/envelope/envelope-resolver.php';
}

if ( ! function_exists( 'dailyos_metadata_proposal_cue_render' ) ) {
	/**
	 * Render the metadata-proposal-cue inner block.
	 *
	 * @param array<string, mixed> $attributes Block attributes.
	 * @param string               $content    Inner content (empty).
	 * @param \WP_Block|null       $block      Parsed block carrying usesContext.
	 * @return string
	 */
	function dailyos_metadata_proposal_cue_render( array $attributes, string $content = '', $block = null ): string {
		unset( $attributes, $content );

		$handle      = null;
		$entity_id   = '';
		$entity_type = '';
		if ( null !== $block && isset( $block->context ) && is_array( $block->context ) ) {
			$handle      = isset( $block->context['dailyos/envelopeHandle'] ) ? (string) $block->context['dailyos/envelopeHandle'] : null;
			$entity_id   = isset( $block->context['dailyos/entityId'] ) ? (string) $block->context['dailyos/entityId'] : '';
			$entity_type = isset( $block->context['dailyos/entityType'] ) ? (string) $block->context['dailyos/entityType'] : '';
		}
		if ( ( null === $handle || '' === $handle ) && isset( $GLOBALS['dailyos_envelope_handle_for_request'] ) ) {
			$handle = (string) $GLOBALS['dailyos_envelope_handle_for_request'];
		}
		if ( '' === $entity_type ) {
			$entity_type = 'account';
		}

		$scope_set = apply_filters( 'dailyos_surfaceclient_resolved_scopes', [] );
		if ( ! is_array( $scope_set ) ) {
			$scope_set = [];
		}

		$envelope = dailyos_resolve_envelope( $handle, $entity_type, $entity_id, $scope_set );

		// AC-328.1 audience filter — AgentMcp audience does not render the
		// interactive cue (aggregate-only per W1 cycle-2 F2 pattern).
		$audience = is_array( $envelope ) && isset( $envelope['audience'] ) ? strtolower( (string) $envelope['audience'] ) : 'user';
		if ( 'agent_mcp' === $audience || 'agent' === $audience ) {
			return dailyos_empty_chip(
				'audience_aggregate_only',
				__( 'Proposals hidden for this audience', 'dailyos' ),
				'wp-block-dailyos-metadata-proposal-cue'
			);
		}

		$proposals = dailyos_metadata_proposal_cue_collect_proposals( $envelope );
		if ( [] === $proposals ) {
			$state  = dailyos_envelope_section( is_array( $envelope ) ? $envelope : null, 'metadata_proposals' );
			$reason = '' !== $state['reason'] ? $state['reason'] : 'no_unresolved_proposals';
			return dailyos_empty_chip(
				$reason,
				__( 'No proposals waiting', 'dailyos' ),
				'wp-block-dailyos-metadata-proposal-cue'
			);
		}

		$count           = count( $proposals );
		$dominant_band   = dailyos_metadata_proposal_cue_dominant_band( $proposals );
		$wrapper_attrs   = dailyos_inner_block_wrapper_attrs( 'wp-block-dailyos-metadata-proposal-cue' );

		// AC-328.1: one-proposal = peripheral cue; multiple-proposals = calm
		// section-level cue. Both render the same primitive shape; the count
		// differentiator lives in data-attrs so chrome.js can adjust tone.
		$label = 1 === $count
			? __( '1 proposal waiting', 'dailyos' )
			: sprintf(
				/* translators: %d: count of unresolved proposals. */
				_n( '%d proposal waiting', '%d proposals waiting', $count, 'dailyos' ),
				$count
			);

		$out  = '<aside ' . $wrapper_attrs . ' data-dailyos-projection="metadata-proposal-cue"';
		$out .= ' data-proposal-count="' . esc_attr( (string) $count ) . '"';
		$out .= ' data-trust-band="' . esc_attr( $dominant_band ) . '"';
		$out .= ' data-empty-reason="">';
		$out .= '<span class="wp-block-dailyos-metadata-proposal-cue__label">' . esc_html( $label ) . '</span>';
		$out .= '</aside>';

		return $out;
	}
}

if ( ! function_exists( 'dailyos_metadata_proposal_cue_collect_proposals' ) ) {
	/**
	 * Collect unresolved metadata proposals from the envelope. Pure projection
	 * over `EntityIntelligenceEnvelope.metadata_proposals`; does not invoke
	 * any abilities.
	 *
	 * @param array<string,mixed>|null $envelope Envelope payload.
	 * @return array<int,array<string,mixed>>
	 */
	function dailyos_metadata_proposal_cue_collect_proposals( ?array $envelope ): array {
		if ( null === $envelope ) {
			return [];
		}
		$slice = $envelope['metadata_proposals'] ?? $envelope['metadataProposals'] ?? $envelope['MetadataProposals'] ?? null;
		if ( ! is_array( $slice ) ) {
			return [];
		}
		$items = $slice['items'] ?? ( isset( $slice[0] ) ? $slice : [] );
		if ( ! is_array( $items ) ) {
			return [];
		}
		$out = [];
		foreach ( $items as $proposal ) {
			if ( ! is_array( $proposal ) ) {
				continue;
			}
			// Lifecycle gate: only unresolved states surface as cues.
			$lifecycle = (string) ( $proposal['lifecycleState'] ?? $proposal['lifecycle_state'] ?? 'pending' );
			if ( ! in_array( $lifecycle, [ 'pending', 'unresolved', 'open' ], true ) ) {
				continue;
			}
			$out[] = $proposal;
		}
		return $out;
	}
}

if ( ! function_exists( 'dailyos_metadata_proposal_cue_dominant_band' ) ) {
	/**
	 * Pick the dominant trust band across collected proposals.
	 *
	 * @param array<int,array<string,mixed>> $proposals Filtered proposal list.
	 * @return string Trust band slug, defaulting to "unscored".
	 */
	function dailyos_metadata_proposal_cue_dominant_band( array $proposals ): string {
		$counts = [];
		foreach ( $proposals as $proposal ) {
			$band = (string) ( $proposal['trustBand'] ?? $proposal['trust_band'] ?? 'unscored' );
			$counts[ $band ] = ( $counts[ $band ] ?? 0 ) + 1;
		}
		if ( [] === $counts ) {
			return 'unscored';
		}
		arsort( $counts );
		return (string) array_key_first( $counts );
	}
}
