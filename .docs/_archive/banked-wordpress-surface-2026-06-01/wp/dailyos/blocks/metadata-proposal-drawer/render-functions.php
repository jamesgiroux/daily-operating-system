<?php
/**
 * Metadata Proposal Drawer (dailyos/metadata-proposal-drawer) — W2 L1 inner block
 * render-functions (DOS-328 / §5.6).
 *
 * Projects from `EntityIntelligenceEnvelope.metadata_proposals` and renders
 * the expanded drawer per AC-328.2:
 *
 *   - typed proposed value (vs current value)
 *   - target field_path
 *   - evidence summary
 *   - trust band + freshness caveat
 *   - display-safe provenance (ADR-0108; NO raw source-internal IDs)
 *   - accept / dismiss / edit affordances
 *
 * Affordance contract (per AC-328.3 + ADR-0123 typed feedback semantics):
 *
 *   Accept  → FeedbackAction::ConfirmCurrent { proposal_id }
 *             (the proposed value is correct; lifecycle confirms.)
 *   Dismiss → FeedbackAction::MarkFalse     { proposal_id, corrected_value? }
 *             (the proposed value is not true; lifecycle withdrawn.)
 *   Edit    → FeedbackAction::NeedsNuance   { proposal_id, corrected_text }
 *             (corrected_text passes ADR-0108 §3 sanitizer; ≤ 500 chars per
 *             AC-328.4 / wave AC #W4 free-text sanitizer clause.)
 *
 *   Wrong-subject (proposal applies elsewhere) is surfaced separately when
 *   the user explicitly flags it; that path emits
 *   FeedbackAction::WrongSubject { corrected_to: SubjectRef? } — see
 *   AC-328.5 envelope-set validation. The structural emission stays in
 *   `services::claims::record_claim_feedback` (W1 substrate — no W1 reopen).
 *
 * The WP block NEVER writes the DB directly — it emits typed feedback
 * data-attributes that the nonce-protected POST handler routes through
 * `services::claims::record_claim_feedback`. Envelope-set validation per
 * AC-328.5 lives in the service layer (W1 AC-477.13). Class-sweep per
 * AC-328.9: accept / dismiss / edit / wrong-subject channels are all
 * enumerated identically as data-feedback-action discriminators.
 *
 * Inner block declares usesContext for dailyos/envelopeHandle and consumes
 * the cached envelope via dailyos_resolve_envelope().
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

if ( ! function_exists( 'dailyos_metadata_proposal_drawer_render' ) ) {
	/**
	 * Render the metadata-proposal-drawer inner block.
	 *
	 * @param array<string, mixed> $attributes Block attributes.
	 * @param string               $content    Inner content (empty).
	 * @param \WP_Block|null       $block      Parsed block carrying usesContext.
	 * @return string
	 */
	function dailyos_metadata_proposal_drawer_render( array $attributes, string $content = '', $block = null ): string {
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

		// AC-328.1 audience filter — AgentMcp audience renders aggregate-only
		// (no per-proposal interactive affordances). Per W1 cycle-2 F2 pattern.
		$audience = is_array( $envelope ) && isset( $envelope['audience'] ) ? strtolower( (string) $envelope['audience'] ) : 'user';
		if ( 'agent_mcp' === $audience || 'agent' === $audience ) {
			$proposals_aggregate = dailyos_metadata_proposal_drawer_collect_proposals( $envelope );
			$count               = count( $proposals_aggregate );
			$wrapper_aggregate   = dailyos_inner_block_wrapper_attrs( 'wp-block-dailyos-metadata-proposal-drawer' );
			$label               = 0 === $count
				? __( 'No proposals waiting', 'dailyos' )
				: sprintf(
					/* translators: %d: count of unresolved proposals. */
					_n( '%d proposal waiting', '%d proposals waiting', $count, 'dailyos' ),
					$count
				);
			return '<aside ' . $wrapper_aggregate . ' data-dailyos-projection="metadata-proposal-drawer"'
				. ' data-audience="agent_mcp"'
				. ' data-proposal-count="' . esc_attr( (string) $count ) . '"'
				. ' data-empty-reason="">'
				. '<span class="wp-block-dailyos-metadata-proposal-drawer__aggregate-label">' . esc_html( $label ) . '</span>'
				. '</aside>';
		}

		$proposals = dailyos_metadata_proposal_drawer_collect_proposals( $envelope );
		if ( [] === $proposals ) {
			$state  = dailyos_envelope_section( is_array( $envelope ) ? $envelope : null, 'metadata_proposals' );
			$reason = '' !== $state['reason'] ? $state['reason'] : 'no_unresolved_proposals';
			return dailyos_empty_chip(
				$reason,
				__( 'No proposals waiting', 'dailyos' ),
				'wp-block-dailyos-metadata-proposal-drawer'
			);
		}

		$wrapper_attrs = dailyos_inner_block_wrapper_attrs( 'wp-block-dailyos-metadata-proposal-drawer' );

		$out  = '<section ' . $wrapper_attrs . ' data-dailyos-projection="metadata-proposal-drawer" data-empty-reason="">';
		$out .= '<header class="wp-block-dailyos-metadata-proposal-drawer__header">';
		$out .= '<h2 class="wp-block-dailyos-metadata-proposal-drawer__title">' . esc_html__( 'Proposed updates', 'dailyos' ) . '</h2>';
		$out .= '</header>';
		$out .= '<ul class="wp-block-dailyos-metadata-proposal-drawer__list" data-dailyos-envelope-sections="metadata_proposals">';
		foreach ( $proposals as $proposal ) {
			$out .= dailyos_metadata_proposal_drawer_render_row( $proposal );
		}
		$out .= '</ul>';
		$out .= '</section>';

		return $out;
	}
}

if ( ! function_exists( 'dailyos_metadata_proposal_drawer_collect_proposals' ) ) {
	/**
	 * Collect unresolved metadata proposals from the envelope. Pure projection
	 * over `EntityIntelligenceEnvelope.metadata_proposals`; no ability calls.
	 *
	 * @param array<string,mixed>|null $envelope Envelope payload.
	 * @return array<int,array<string,mixed>>
	 */
	function dailyos_metadata_proposal_drawer_collect_proposals( ?array $envelope ): array {
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
			$lifecycle = (string) ( $proposal['lifecycleState'] ?? $proposal['lifecycle_state'] ?? 'pending' );
			if ( ! in_array( $lifecycle, [ 'pending', 'unresolved', 'open' ], true ) ) {
				continue;
			}
			$out[] = $proposal;
		}
		return $out;
	}
}

if ( ! function_exists( 'dailyos_metadata_proposal_drawer_render_row' ) ) {
	/**
	 * Render one proposal row with typed value, evidence, trust band, and the
	 * accept/dismiss/edit affordance triplet.
	 *
	 * The button carries the canonical payload shape as data-attributes; the
	 * POST handler reads them and routes through `record_claim_feedback`
	 * with envelope-set validation (AC-328.5 / W1 AC-477.13). Display-safe
	 * provenance only (ADR-0108) — no raw source-internal identifiers.
	 *
	 * @param array<string,mixed> $proposal MetadataProposal payload.
	 * @return string
	 */
	function dailyos_metadata_proposal_drawer_render_row( array $proposal ): string {
		$proposal_id    = (string) ( $proposal['proposalId'] ?? $proposal['proposal_id'] ?? '' );
		$field_path     = (string) ( $proposal['fieldPath'] ?? $proposal['field_path'] ?? '' );
		$current_value  = (string) ( $proposal['currentValue'] ?? $proposal['current_value'] ?? '' );
		$proposed_value = (string) ( $proposal['proposedValue'] ?? $proposal['proposed_value'] ?? '' );
		$trust_band     = (string) ( $proposal['trustBand'] ?? $proposal['trust_band'] ?? 'unscored' );
		$evidence       = (string) ( $proposal['evidenceSummary'] ?? $proposal['evidence_summary'] ?? '' );
		$freshness      = (string) ( $proposal['freshness']['caveat'] ?? $proposal['freshness_caveat'] ?? '' );

		if ( '' === $proposal_id ) {
			return dailyos_empty_chip(
				'proposal_missing_id',
				__( 'Proposal unavailable', 'dailyos' ),
				'wp-block-dailyos-metadata-proposal-drawer'
			);
		}

		$row  = '<li class="wp-block-dailyos-metadata-proposal-drawer__row"';
		$row .= ' data-proposal-id="' . esc_attr( $proposal_id ) . '"';
		$row .= ' data-trust-band="' . esc_attr( $trust_band ) . '"';
		$row .= '>';
		$row .= '<div class="wp-block-dailyos-metadata-proposal-drawer__field">';
		$row .= '<span class="wp-block-dailyos-metadata-proposal-drawer__field-label">' . esc_html__( 'Field', 'dailyos' ) . '</span>';
		$row .= '<span class="wp-block-dailyos-metadata-proposal-drawer__field-value">' . esc_html( $field_path ) . '</span>';
		$row .= '</div>';
		if ( '' !== $current_value ) {
			$row .= '<div class="wp-block-dailyos-metadata-proposal-drawer__current">';
			$row .= '<span class="wp-block-dailyos-metadata-proposal-drawer__current-label">' . esc_html__( 'Current', 'dailyos' ) . '</span>';
			$row .= '<span class="wp-block-dailyos-metadata-proposal-drawer__current-value">' . esc_html( $current_value ) . '</span>';
			$row .= '</div>';
		}
		$row .= '<div class="wp-block-dailyos-metadata-proposal-drawer__proposed">';
		$row .= '<span class="wp-block-dailyos-metadata-proposal-drawer__proposed-label">' . esc_html__( 'Proposed', 'dailyos' ) . '</span>';
		$row .= '<span class="wp-block-dailyos-metadata-proposal-drawer__proposed-value">' . esc_html( $proposed_value ) . '</span>';
		$row .= '</div>';
		if ( '' !== $evidence ) {
			$row .= '<p class="wp-block-dailyos-metadata-proposal-drawer__evidence">' . esc_html( $evidence ) . '</p>';
		}
		if ( '' !== $freshness ) {
			$row .= '<p class="wp-block-dailyos-metadata-proposal-drawer__freshness">' . esc_html( $freshness ) . '</p>';
		}
		$row .= dailyos_metadata_proposal_drawer_render_affordances( $proposal_id );
		$row .= '</li>';

		return $row;
	}
}

if ( ! function_exists( 'dailyos_metadata_proposal_drawer_render_affordances' ) ) {
	/**
	 * Emit the accept / dismiss / edit affordance triplet for one proposal.
	 * Each button carries `data-feedback-action` mapped to the canonical
	 * ADR-0123 FeedbackAction variant. All three channels are enumerated
	 * identically per AC-328.9 class-sweep.
	 *
	 * @param string $proposal_id Subject proposal id (already validated non-empty).
	 * @return string
	 */
	function dailyos_metadata_proposal_drawer_render_affordances( string $proposal_id ): string {
		$id_attr = esc_attr( $proposal_id );

		$out  = '<div class="wp-block-dailyos-metadata-proposal-drawer__affordances" data-dailyos-affordance="metadata-proposal">';

		// Accept → ConfirmCurrent.
		$out .= '<button type="button" class="dailyos-button dailyos-button--accept-proposal"';
		$out .= ' data-dailyos-action="accept-proposal"';
		$out .= ' data-proposal-id="' . $id_attr . '"';
		$out .= ' data-feedback-action="confirm_current"';
		$out .= '>' . esc_html__( 'Accept', 'dailyos' ) . '</button>';

		// Dismiss → MarkFalse.
		$out .= '<button type="button" class="dailyos-button dailyos-button--dismiss-proposal"';
		$out .= ' data-dailyos-action="dismiss-proposal"';
		$out .= ' data-proposal-id="' . $id_attr . '"';
		$out .= ' data-feedback-action="mark_false"';
		$out .= '>' . esc_html__( 'Dismiss', 'dailyos' ) . '</button>';

		// Edit → NeedsNuance (corrected_text routed through ADR-0108 §3 sanitizer).
		$out .= '<button type="button" class="dailyos-button dailyos-button--edit-proposal"';
		$out .= ' data-dailyos-action="edit-proposal"';
		$out .= ' data-proposal-id="' . $id_attr . '"';
		$out .= ' data-feedback-action="needs_nuance"';
		$out .= '>' . esc_html__( 'Edit', 'dailyos' ) . '</button>';

		$out .= '</div>';

		return $out;
	}
}
