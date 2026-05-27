<?php
/**
 * Meeting Prep Status inner-block server-side render.
 *
 * Conditional indicator pill near the meeting header. Renders only when
 * the meeting is NOT in `ready` state — `preparing` shows a spinner-style
 * developing badge, `stale` / `needs_preparation` show a warning chip per
 * the `editorial-briefing_prepFlag` pattern in
 * `.docs/design/reference/surfaces/briefing.html` lines 75-78 (the same
 * primitive surfaces from briefings into the meeting page).
 *
 * In `ready` state the block emits an empty wrapper so chrome.js can
 * still observe its presence for FolioBar readiness signaling, but no
 * visible chip — readiness chrome lives in the magazine shell, not in
 * the block body.
 *
 * Reads `dailyos/entityId` from outer-block context (provided by
 * `dailyos/meeting-detail`), invokes `meeting_prep_status` (DOS-335)
 * directly. Two-call composition with `get_entity_intelligence` per
 * V1.2.1 §5.4 — the envelope cache de-duplicates within a request.
 *
 * @package DailyOS
 *
 * @var array<string, mixed> $attributes Block attributes.
 */

declare(strict_types=1);

if ( ! defined( 'ABSPATH' ) ) {
	return '';
}

if ( ! function_exists( 'dailyos_meeting_prep_status_render' ) ) {
	/**
	 * Render the meeting-prep-status inner block.
	 *
	 * @param array<string, mixed> $attributes Block attributes.
	 * @param mixed                $block      WP_Block instance (provides context).
	 * @return string Rendered HTML.
	 */
	function dailyos_meeting_prep_status_render( array $attributes, $block = null ): string {
		$meeting_id = '';
		if ( is_object( $block ) && isset( $block->context ) && is_array( $block->context ) ) {
			$meeting_id = isset( $block->context['dailyos/entityId'] )
				? (string) $block->context['dailyos/entityId']
				: '';
		}

		if ( '' === $meeting_id ) {
			return dailyos_meeting_prep_status_empty( 'missing_meeting_context' );
		}

		$runtime_client = apply_filters( 'dailyos_runtime_client_for_block', null );
		if ( ! is_object( $runtime_client ) || ! method_exists( $runtime_client, 'invoke_ability' ) ) {
			return dailyos_meeting_prep_status_empty( 'runtime_unavailable' );
		}

		$scope_set = apply_filters( 'dailyos_surfaceclient_resolved_scopes', [] );
		if ( ! is_array( $scope_set ) ) {
			$scope_set = [];
		}

		$response = $runtime_client->invoke_ability(
			'meeting_prep_status',
			[ 'meeting_id' => $meeting_id ],
			$scope_set
		);

		if ( is_wp_error( $response ) ) {
			return dailyos_meeting_prep_status_empty( 'prep_status_error' );
		}

		$dto = dailyos_meeting_prep_status_unwrap( $response );
		if ( null === $dto ) {
			return dailyos_meeting_prep_status_empty( 'no_dto' );
		}

		$status           = isset( $dto['status'] ) ? (string) $dto['status'] : '';
		$blocking_reason  = isset( $dto['blocking_reason'] ) ? (string) $dto['blocking_reason'] : '';
		$stale_reason     = isset( $dto['stale_reason'] ) ? (string) $dto['stale_reason'] : '';
		$last_prepared_at = isset( $dto['last_prepared_at'] ) ? (string) $dto['last_prepared_at'] : '';

		// Ready / running / queued render no visible chip — chrome.js still
		// observes the wrapper for FolioBar signaling.
		$silent_states = [ 'ready', 'running', 'queued' ];
		if ( in_array( $status, $silent_states, true ) ) {
			return dailyos_meeting_prep_status_silent( $status, $last_prepared_at );
		}

		$label = dailyos_meeting_prep_status_label( $status, $blocking_reason, $stale_reason );
		$tone  = dailyos_meeting_prep_status_tone( $status );

		$wrapper_attrs = function_exists( 'get_block_wrapper_attributes' )
			? get_block_wrapper_attributes(
				[
					'class'            => 'wp-block-dailyos-meeting-prep-status meeting-intel_prepStatusWrap',
					'data-ds-tier'     => 'primitive',
					'data-ds-name'     => 'MeetingPrepStatus',
					'data-prep-status' => $status,
				]
			)
			: 'class="wp-block-dailyos-meeting-prep-status meeting-intel_prepStatusWrap" data-prep-status="' . esc_attr( $status ) . '"';

		return '<div ' . $wrapper_attrs . '>'
			. '<span class="Pill_pill Pill_' . esc_attr( $tone ) . ' Pill_compact">'
			. esc_html( $label )
			. '</span>'
			. '</div>';
	}
}

if ( ! function_exists( 'dailyos_meeting_prep_status_unwrap' ) ) {
	/**
	 * Unwrap the prep status DTO from a runtime response envelope.
	 *
	 * @param array<string, mixed> $response Raw runtime client response.
	 * @return array<string, mixed>|null
	 */
	function dailyos_meeting_prep_status_unwrap( array $response ): ?array {
		$ability = $response['ability'] ?? null;
		if ( is_array( $ability ) && isset( $ability['data'] ) && is_array( $ability['data'] ) ) {
			return $ability['data'];
		}
		if ( isset( $response['data'] ) && is_array( $response['data'] ) ) {
			return $response['data'];
		}
		if ( isset( $response['meeting_id'] ) ) {
			// Already-unwrapped DTO at top level.
			return $response;
		}
		return null;
	}
}

if ( ! function_exists( 'dailyos_meeting_prep_status_label' ) ) {
	/**
	 * Human-readable label for a prep-status chip.
	 *
	 * @param string $status          Prep status enum value.
	 * @param string $blocking_reason Reason text when status is prep_needed.
	 * @param string $stale_reason    Reason text when status is stale.
	 *
	 * @return string Localized chip label.
	 */
	function dailyos_meeting_prep_status_label( string $status, string $blocking_reason, string $stale_reason ): string {
		switch ( $status ) {
			case 'preparing':
				return __( 'Preparing briefing…', 'dailyos' );
			case 'stale':
				if ( '' === $stale_reason ) {
					return __( 'Briefing stale — refresh recommended', 'dailyos' );
				}
				/* translators: %s: stale reason slug, underscores converted to spaces. */
				return sprintf( __( 'Briefing stale (%s)', 'dailyos' ), str_replace( '_', ' ', $stale_reason ) );
			case 'limited':
				return __( 'Briefing limited — partial data', 'dailyos' );
			case 'failed':
				return __( 'Briefing failed to prepare', 'dailyos' );
			case 'blocked_no_entity':
				return __( 'No linked entity — relink the meeting', 'dailyos' );
			case 'user_suppressed':
				return __( 'Briefing suppressed by user', 'dailyos' );
			case 'user_dismissed':
				return __( 'Briefing dismissed', 'dailyos' );
			case 'prep_needed':
			default:
				if ( '' === $blocking_reason ) {
					return __( 'No briefing yet', 'dailyos' );
				}
				/* translators: %s: blocking reason slug, underscores converted to spaces. */
				return sprintf( __( 'Briefing needed (%s)', 'dailyos' ), str_replace( '_', ' ', $blocking_reason ) );
		}
	}
}

if ( ! function_exists( 'dailyos_meeting_prep_status_tone' ) ) {
	/**
	 * Tone token for a prep-status chip.
	 *
	 * @param string $status Prep status enum value.
	 *
	 * @return string Design-token tone slug.
	 */
	function dailyos_meeting_prep_status_tone( string $status ): string {
		switch ( $status ) {
			case 'preparing':
				return 'turmeric';
			case 'stale':
			case 'limited':
				return 'terracotta';
			case 'failed':
			case 'blocked_no_entity':
				return 'rust';
			case 'user_suppressed':
			case 'user_dismissed':
				return 'neutral';
			case 'prep_needed':
			default:
				return 'terracotta';
		}
	}
}

if ( ! function_exists( 'dailyos_meeting_prep_status_silent' ) ) {
	/**
	 * Silent wrapper for ready / running / queued — chrome.js still observes.
	 *
	 * @param string $status            Prep status enum value.
	 * @param string $last_prepared_at  ISO-8601 timestamp of last successful prep.
	 *
	 * @return string Hidden div with data attributes for chrome.js to read.
	 */
	function dailyos_meeting_prep_status_silent( string $status, string $last_prepared_at ): string {
		return sprintf(
			'<div class="wp-block-dailyos-meeting-prep-status is-silent" data-prep-status="%s" data-last-prepared-at="%s" aria-hidden="true"></div>',
			esc_attr( $status ),
			esc_attr( $last_prepared_at )
		);
	}
}

if ( ! function_exists( 'dailyos_meeting_prep_status_empty' ) ) {
	/**
	 * Visible empty-state chip per §10 invariant — never silent-hidden when
	 * we genuinely have no data to project.
	 *
	 * @param string $reason Empty-state reason slug.
	 * @param string $label  Optional override label; falls back to slug-derived label.
	 *
	 * @return string Rendered empty-state chip HTML.
	 */
	function dailyos_meeting_prep_status_empty( string $reason, string $label = '' ): string {
		if ( '' === $label ) {
			$label = dailyos_meeting_prep_status_empty_label( $reason );
		}

		return sprintf(
			'<div class="wp-block-dailyos-meeting-prep-status wp-block-dailyos-meeting-prep-status--empty is-empty"><span class="dailyos-empty-chip" data-empty-reason="%s">%s</span></div>',
			esc_attr( $reason ),
			esc_html( $label )
		);
	}
}

if ( ! function_exists( 'dailyos_meeting_prep_status_empty_label' ) ) {
	/**
	 * Default empty-state label for a reason slug.
	 *
	 * @param string $reason Empty-state reason slug.
	 *
	 * @return string Localized label.
	 */
	function dailyos_meeting_prep_status_empty_label( string $reason ): string {
		switch ( $reason ) {
			case 'missing_meeting_context':
				return __( 'No meeting context.', 'dailyos' );
			case 'runtime_unavailable':
				return __( 'Briefing status unavailable.', 'dailyos' );
			case 'prep_status_error':
				return __( 'Briefing status unavailable.', 'dailyos' );
			case 'no_dto':
			default:
				return __( 'No briefing status yet.', 'dailyos' );
		}
	}
}
