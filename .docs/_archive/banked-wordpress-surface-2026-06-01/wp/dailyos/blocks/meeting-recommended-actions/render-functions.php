<?php
/**
 * MeetingRecommendedActions inner-block server-side render.
 *
 * Translation of `.docs/design/reference/surfaces/meeting.html` lines
 * 509-560 — the "Open Items" chapter on the meeting briefing page.
 *
 * Reads `dailyos/entityId` from outer-block context (provided by
 * `dailyos/meeting-detail`), invokes `get_entity_intelligence`
 * (entity_type=meeting) via the runtime client, and projects the top-level
 * `recommended_actions` array from the meeting envelope.
 *
 * @package DailyOS
 *
 * @var array<string, mixed> $attributes Block attributes.
 */

declare(strict_types=1);

if ( ! defined( 'ABSPATH' ) ) {
	return '';
}

if ( ! function_exists( 'dailyos_meeting_recommended_actions_render' ) ) {
	/**
	 * Render the meeting-recommended-actions inner block.
	 *
	 * @param array<string, mixed> $attributes Block attributes.
	 * @param mixed                $block      WP_Block instance (provides context).
	 * @return string Rendered HTML.
	 */
	function dailyos_meeting_recommended_actions_render( array $attributes, $block = null ): string {
		$meeting_id = '';
		if ( is_object( $block ) && isset( $block->context ) && is_array( $block->context ) ) {
			$meeting_id = isset( $block->context['dailyos/entityId'] )
				? (string) $block->context['dailyos/entityId']
				: '';
		}

		if ( '' === $meeting_id ) {
			return dailyos_meeting_recommended_actions_empty_chip( 'missing_meeting_context', __( 'No meeting context.', 'dailyos' ) );
		}

		$runtime_client = apply_filters( 'dailyos_runtime_client_for_block', null );
		if ( ! is_object( $runtime_client ) || ! method_exists( $runtime_client, 'invoke_ability' ) ) {
			return dailyos_meeting_recommended_actions_empty_chip( 'runtime_unavailable', __( 'Recommended actions unavailable.', 'dailyos' ) );
		}

		$scope_set = apply_filters( 'dailyos_surfaceclient_resolved_scopes', [] );
		if ( ! is_array( $scope_set ) ) {
			$scope_set = [];
		}

		$response = $runtime_client->invoke_ability(
			'get_entity_intelligence',
			[
				'entity_type' => 'meeting',
				'entity_id'   => $meeting_id,
			],
			$scope_set
		);

		if ( is_wp_error( $response ) || ! is_array( $response ) || ( isset( $response['ok'] ) && false === $response['ok'] ) ) {
			return dailyos_meeting_recommended_actions_empty_chip( 'envelope_error', __( 'Recommended actions unavailable.', 'dailyos' ) );
		}

		$actions = dailyos_meeting_recommended_actions_extract_actions( $response );
		if ( null === $actions || empty( $actions ) ) {
			return dailyos_meeting_recommended_actions_empty_chip( 'no_actions', __( 'No open items.', 'dailyos' ) );
		}

		$urgency_classes = [
			'overdue'   => 'meeting-intel_openItemOverdue',
			'today'     => 'meeting-intel_openItemToday',
			'this_week' => 'meeting-intel_openItemThisWeek',
			'later'     => '',
		];

		$rows = '';
		foreach ( $actions as $action ) {
			if ( ! is_array( $action ) ) {
				continue;
			}

			$action_id = isset( $action['action_id'] ) ? (string) $action['action_id'] : '';
			$headline  = isset( $action['headline'] ) ? (string) $action['headline'] : '';
			$context   = isset( $action['context'] ) ? (string) $action['context'] : '';
			$urgency   = isset( $action['urgency'] ) ? (string) $action['urgency'] : 'later';
			$why       = isset( $action['why'] ) ? (string) $action['why'] : '';

			if ( '' === $action_id || '' === $headline || '' === $context ) {
				continue;
			}

			$urgency_class = $urgency_classes[ $urgency ] ?? '';
			$row_classes   = 'meeting-intel_openItemRow';
			if ( '' !== $urgency_class ) {
				$row_classes .= ' ' . $urgency_class;
			}

			$rows .= '<div class="' . esc_attr( $row_classes ) . '" data-action-id="' . esc_attr( $action_id ) . '">';
			// TODO: wire complete affordance via close_open_loop ability where the checkbox/complete button would go. Skip the affordance for this commit.
			$rows .= '<p class="meeting-intel_openItemTitle">' . esc_html( $headline ) . '</p>';
			$rows .= '<p class="meeting-intel_openItemContext">' . esc_html( $context ) . '</p>';
			if ( '' !== $why ) {
				$rows .= '<p class="meeting-intel_openItemWhy">' . esc_html( $why ) . '</p>';
			}
			$rows .= '</div>';
		}

		if ( '' === $rows ) {
			return dailyos_meeting_recommended_actions_empty_chip( 'no_actions', __( 'No open items.', 'dailyos' ) );
		}

		$out  = '<section class="editorial-reveal meeting-intel_chapterSection" data-ds-name="MeetingRecommendedActions">';
		$out .= '<div class="ChapterHeading_heading">';
		$out .= '<hr class="ChapterHeading_rule">';
		$out .= '<div class="ChapterHeading_titleRow">';
		$out .= '<h2 class="ChapterHeading_title">' . esc_html__( 'Open Items', 'dailyos' ) . '</h2>';
		$out .= '</div>';
		$out .= '</div>';
		$out .= '<div class="meeting-intel_openItemsContainer">';
		$out .= $rows;
		$out .= '</div>';
		$out .= '</section>';

		return $out;
	}
}

if ( ! function_exists( 'dailyos_meeting_recommended_actions_extract_actions' ) ) {
	/**
	 * Pull the recommended_actions array from an EntityIntelligenceEnvelope response.
	 *
	 * @param array<string, mixed> $response Raw runtime client response.
	 * @return array<int, array<string, mixed>>|null
	 */
	function dailyos_meeting_recommended_actions_extract_actions( array $response ): ?array {
		$envelope = null;
		$ability  = $response['ability'] ?? null;
		if ( is_array( $ability ) && isset( $ability['data'] ) && is_array( $ability['data'] ) ) {
			$envelope = $ability['data'];
		} elseif ( isset( $response['data'] ) && is_array( $response['data'] ) ) {
			$envelope = $response['data'];
		} elseif ( isset( $response['envelope'] ) && is_array( $response['envelope'] ) ) {
			$envelope = $response['envelope'];
		} else {
			$envelope = $response;
		}
		if ( ! is_array( $envelope ) ) {
			return null;
		}
		$actions = $envelope['recommended_actions'] ?? null;
		if ( ! is_array( $actions ) ) {
			return null;
		}
		return $actions;
	}
}

if ( ! function_exists( 'dailyos_meeting_recommended_actions_empty_chip' ) ) {
	/**
	 * Visible empty-state chip per §10 invariant — never silent-hidden.
	 *
	 * @param string $reason Machine-readable reason for diagnostics.
	 * @param string $label  Human-readable label.
	 * @return string Rendered HTML chip.
	 */
	function dailyos_meeting_recommended_actions_empty_chip( string $reason, string $label ): string {
		return sprintf(
			'<section class="editorial-reveal meeting-intel_chapterSection meeting-intel_chapterSection--empty is-empty" data-ds-name="MeetingRecommendedActions"><span class="dailyos-empty-chip" data-empty-reason="%s">%s</span></section>',
			esc_attr( $reason ),
			esc_html( $label )
		);
	}
}
