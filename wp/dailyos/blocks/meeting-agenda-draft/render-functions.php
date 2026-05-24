<?php
/**
 * Meeting Agenda Draft inner-block server-side render.
 *
 * Translation of `.docs/design/reference/surfaces/meeting.html` lines
 * 402-418 — the "Before This Meeting" readiness checklist on the
 * meeting briefing page.
 *
 * Reads `dailyos/entityId` from outer-block context (provided by
 * `dailyos/meeting-detail`), invokes `get_entity_intelligence`
 * (entity_type=meeting) via the runtime client, and projects the
 * top-level `readiness_items` array from the meeting envelope.
 *
 * @package DailyOS
 *
 * @var array<string, mixed> $attributes Block attributes.
 */

declare(strict_types=1);

if ( ! defined( 'ABSPATH' ) ) {
	return '';
}

if ( ! function_exists( 'dailyos_meeting_agenda_draft_render' ) ) {
	/**
	 * Render the meeting-agenda-draft inner block.
	 *
	 * @param array<string, mixed> $attributes Block attributes.
	 * @param mixed                $block      WP_Block instance (provides context).
	 * @return string Rendered HTML.
	 */
	function dailyos_meeting_agenda_draft_render( array $attributes, $block = null ): string {
		$meeting_id = '';
		if ( is_object( $block ) && isset( $block->context ) && is_array( $block->context ) ) {
			$meeting_id = isset( $block->context['dailyos/entityId'] )
				? (string) $block->context['dailyos/entityId']
				: '';
		}

		if ( '' === $meeting_id ) {
			return dailyos_meeting_agenda_draft_empty_chip( 'missing_meeting_context', __( 'No meeting context.', 'dailyos' ) );
		}

		$runtime_client = apply_filters( 'dailyos_runtime_client_for_block', null );
		if ( ! is_object( $runtime_client ) || ! method_exists( $runtime_client, 'invoke_ability' ) ) {
			return dailyos_meeting_agenda_draft_empty_chip( 'runtime_unavailable', __( 'Readiness checklist unavailable.', 'dailyos' ) );
		}

		$scope_set = apply_filters( 'dailyos_surfaceclient_resolved_scopes', [] );
		if ( ! is_array( $scope_set ) ) {
			$scope_set = [];
		}

		// W1 producer: get_entity_intelligence (entity_type=meeting) — readiness_items.
		$response = $runtime_client->invoke_ability(
			'get_entity_intelligence',
			[
				'entity_type' => 'meeting',
				'entity_id'   => $meeting_id,
			],
			$scope_set
		);

		if ( is_wp_error( $response ) || ! is_array( $response ) || ( isset( $response['ok'] ) && false === $response['ok'] ) ) {
			return dailyos_meeting_agenda_draft_empty_chip( 'envelope_error', __( 'Readiness checklist unavailable.', 'dailyos' ) );
		}

		$readiness_items = dailyos_meeting_agenda_draft_extract_readiness_items( $response );
		if ( null === $readiness_items || empty( $readiness_items ) ) {
			return dailyos_meeting_agenda_draft_empty_chip( 'no_readiness_items', __( 'No readiness items.', 'dailyos' ) );
		}

		$dot_tone_classes = [
			'turmeric_muted' => 'meeting-intel_bulletDotTurmericMuted',
			'sage'           => 'meeting-intel_bulletDotSage',
			'terracotta'     => 'meeting-intel_bulletDotTerracotta',
		];

		$rows = '';
		foreach ( $readiness_items as $item ) {
			if ( ! is_array( $item ) || ! isset( $item['text'] ) ) {
				continue;
			}

			$text = (string) $item['text'];
			if ( '' === trim( $text ) ) {
				continue;
			}

			$dot_tone  = isset( $item['dot_tone'] ) ? strtolower( trim( (string) $item['dot_tone'] ) ) : '';
			$dot_class = $dot_tone_classes[ $dot_tone ] ?? $dot_tone_classes['turmeric_muted'];

			$rows .= '<li class="meeting-intel_readinessItem">';
			$rows .= '<span class="meeting-intel_bulletDot ' . esc_attr( $dot_class ) . '"></span>';
			$rows .= '<span>' . esc_html( $text ) . '</span>';
			$rows .= '</li>';
		}

		if ( '' === $rows ) {
			return dailyos_meeting_agenda_draft_empty_chip( 'no_readiness_items', __( 'No readiness items.', 'dailyos' ) );
		}

		$wrapper_attrs = function_exists( 'get_block_wrapper_attributes' )
			? get_block_wrapper_attributes(
				[
					'id'           => 'your-plan',
					'class'        => 'wp-block-dailyos-meeting-agenda-draft meeting-intel_readinessWrap',
					'data-ds-tier' => 'primitive',
					'data-ds-name' => 'MeetingAgendaDraft',
				]
			)
			: 'class="wp-block-dailyos-meeting-agenda-draft meeting-intel_readinessWrap" data-ds-tier="primitive" data-ds-name="MeetingAgendaDraft"';

		$out  = '<div ' . $wrapper_attrs . '>';
		$out .= '<p class="meeting-intel_chapterHeading meeting-intel_readinessHeading">'
			. esc_html__( 'Before This Meeting', 'dailyos' )
			. '</p>';
		$out .= '<ul class="meeting-intel_readinessList">';
		$out .= $rows;
		$out .= '</ul>';
		$out .= '</div>';

		return $out;
	}
}

if ( ! function_exists( 'dailyos_meeting_agenda_draft_extract_readiness_items' ) ) {
	/**
	 * Pull the readiness_items array from an EntityIntelligenceEnvelope response.
	 *
	 * @param array<string, mixed> $response Raw runtime client response.
	 * @return array<int, array<string, mixed>>|null
	 */
	function dailyos_meeting_agenda_draft_extract_readiness_items( array $response ): ?array {
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
		$readiness_items = $envelope['readiness_items'] ?? null;
		if ( ! is_array( $readiness_items ) ) {
			return null;
		}
		return $readiness_items;
	}
}

if ( ! function_exists( 'dailyos_meeting_agenda_draft_empty_chip' ) ) {
	/**
	 * Visible empty-state chip per §10 invariant — never silent-hidden.
	 *
	 * @param string $reason Machine-readable reason for diagnostics.
	 * @param string $label  Human-readable label.
	 * @return string Rendered HTML chip.
	 */
	function dailyos_meeting_agenda_draft_empty_chip( string $reason, string $label ): string {
		return sprintf(
			'<div id="your-plan" class="wp-block-dailyos-meeting-agenda-draft meeting-intel_readinessWrap meeting-intel_readinessWrap--empty is-empty"><span class="dailyos-empty-chip" data-empty-reason="%s">%s</span></div>',
			esc_attr( $reason ),
			esc_html( $label )
		);
	}
}
