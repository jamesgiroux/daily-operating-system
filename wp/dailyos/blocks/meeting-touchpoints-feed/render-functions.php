<?php
/**
 * Meeting Touchpoints Feed inner-block server-side render.
 *
 * Translation of the touchpoints list from
 * `.docs/design/reference/surfaces/meeting.html` lines 64-91.
 *
 * Reads `dailyos/entityId` from outer-block context (provided by
 * `dailyos/meeting-detail`), invokes `get_entity_intelligence`
 * (entity_type=meeting) via the runtime client, and projects the
 * `touchpoints.items` paginated wrapper from the meeting envelope.
 *
 * @package DailyOS
 *
 * @var array<string, mixed> $attributes Block attributes.
 */

declare(strict_types=1);

if ( ! defined( 'ABSPATH' ) ) {
	return '';
}

if ( ! function_exists( 'dailyos_meeting_touchpoints_feed_render' ) ) {
	/**
	 * Render the meeting-touchpoints-feed inner block.
	 *
	 * @param array<string, mixed> $attributes Block attributes.
	 * @param mixed                $block      WP_Block instance (provides context).
	 * @return string Rendered HTML.
	 */
	function dailyos_meeting_touchpoints_feed_render( array $attributes, $block = null ): string {
		$meeting_id = '';
		if ( is_object( $block ) && isset( $block->context ) && is_array( $block->context ) ) {
			$meeting_id = isset( $block->context['dailyos/entityId'] )
				? (string) $block->context['dailyos/entityId']
				: '';
		}

		if ( '' === $meeting_id ) {
			return dailyos_meeting_touchpoints_feed_empty_chip( 'missing_meeting_context', __( 'No meeting context.', 'dailyos' ) );
		}

		$runtime_client = apply_filters( 'dailyos_runtime_client_for_block', null );
		if ( ! is_object( $runtime_client ) || ! method_exists( $runtime_client, 'invoke_ability' ) ) {
			return dailyos_meeting_touchpoints_feed_empty_chip( 'runtime_unavailable', __( 'Touchpoints unavailable.', 'dailyos' ) );
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
			return dailyos_meeting_touchpoints_feed_empty_chip( 'envelope_error', __( 'Touchpoints unavailable.', 'dailyos' ) );
		}

		$touchpoints = dailyos_meeting_touchpoints_feed_extract_touchpoints( $response );
		if ( null === $touchpoints || empty( $touchpoints ) ) {
			return dailyos_meeting_touchpoints_feed_empty_chip( 'no_touchpoints', __( 'No touchpoints yet.', 'dailyos' ) );
		}

		$side_classes = [
			'past'     => 'meeting-intel_touchpointItemPast',
			'present'  => 'meeting-intel_touchpointItemPresent',
			'upcoming' => 'meeting-intel_touchpointItemUpcoming',
		];

		$rows = [];
		foreach ( $touchpoints as $touchpoint ) {
			if ( ! is_array( $touchpoint ) ) {
				continue;
			}

			$touchpoint_id  = isset( $touchpoint['touchpoint_id'] ) ? (string) $touchpoint['touchpoint_id'] : '';
			$target_meeting = isset( $touchpoint['meeting_id'] ) ? (string) $touchpoint['meeting_id'] : '';
			$title          = isset( $touchpoint['title'] ) ? (string) $touchpoint['title'] : '';
			$when           = isset( $touchpoint['when'] ) ? (string) $touchpoint['when'] : '';
			$side           = isset( $touchpoint['side'] ) ? (string) $touchpoint['side'] : '';

			if (
				'' === $touchpoint_id
				|| '' === $target_meeting
				|| '' === $title
				|| '' === $when
				|| ! isset( $side_classes[ $side ] )
				|| ! isset( $touchpoint['attendee_count'] )
				|| ! is_numeric( $touchpoint['attendee_count'] )
			) {
				continue;
			}

			$timestamp = strtotime( $when );
			if ( false === $timestamp ) {
				continue;
			}

			$attendee_count = (int) $touchpoint['attendee_count'];
			if ( $attendee_count < 0 ) {
				continue;
			}

			$rows[] = [
				'attendee_count' => $attendee_count,
				'date'           => wp_date( get_option( 'date_format', 'M j, Y' ), $timestamp ),
				'href'           => '/meetings/' . rawurlencode( $target_meeting ) . '/',
				'side_class'     => $side_classes[ $side ],
				'timestamp'      => $timestamp,
				'title'          => $title,
				'touchpoint_id'  => $touchpoint_id,
			];
		}

		if ( empty( $rows ) ) {
			return dailyos_meeting_touchpoints_feed_empty_chip( 'no_touchpoints', __( 'No touchpoints yet.', 'dailyos' ) );
		}

		usort(
			$rows,
			static function ( array $a, array $b ): int {
				return $b['timestamp'] <=> $a['timestamp'];
			}
		);

		$out  = '<section class="meeting-intel_chapterSection" data-ds-name="MeetingTouchpointsFeed">';
		$out .= '<div class="ChapterHeading_heading">';
		$out .= '<hr class="ChapterHeading_rule">';
		$out .= '<div class="ChapterHeading_titleRow">';
		$out .= '<h2 class="ChapterHeading_title">' . esc_html__( 'Touchpoints', 'dailyos' ) . '</h2>';
		$out .= '</div>';
		$out .= '</div>';
		$out .= '<ul class="meeting-intel_touchpointList">';

		foreach ( $rows as $row ) {
			$row_classes = 'meeting-intel_touchpointItem ' . $row['side_class'];

			$out .= '<li class="' . esc_attr( $row_classes ) . '" data-touchpoint-id="' . esc_attr( $row['touchpoint_id'] ) . '">';
			$out .= '<a class="meeting-intel_touchpointLink" href="' . esc_url( $row['href'] ) . '">';
			$out .= '<span class="meeting-intel_touchpointTitle">' . esc_html( $row['title'] ) . '</span>';
			$out .= '<span class="meeting-intel_touchpointMeta">';
			$out .= '<span class="meeting-intel_touchpointDate">' . esc_html( $row['date'] ) . '</span>';
			$out .= '<span class="meeting-intel_touchpointSeparator">&middot;</span>';
			$out .= '<span class="meeting-intel_touchpointAttendees">'
				. esc_html( sprintf( __( '%d attendees', 'dailyos' ), $row['attendee_count'] ) )
				. '</span>';
			$out .= '</span>';
			$out .= '</a>';
			$out .= '</li>';
		}

		$out .= '</ul>';
		$out .= '</section>';

		return $out;
	}
}

if ( ! function_exists( 'dailyos_meeting_touchpoints_feed_extract_touchpoints' ) ) {
	/**
	 * Pull touchpoint items from an EntityIntelligenceEnvelope response envelope.
	 *
	 * @param array<string, mixed> $response Raw runtime client response.
	 * @return array<int, mixed>|null
	 */
	function dailyos_meeting_touchpoints_feed_extract_touchpoints( array $response ): ?array {
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
		$touchpoints_section = $envelope['touchpoints'] ?? null;
		if ( ! is_array( $touchpoints_section ) ) {
			return null;
		}
		$items = $touchpoints_section['items'] ?? null;
		if ( ! is_array( $items ) ) {
			return null;
		}
		return $items;
	}
}

if ( ! function_exists( 'dailyos_meeting_touchpoints_feed_empty_chip' ) ) {
	/**
	 * Visible empty-state chip per §10 invariant — never silent-hidden.
	 *
	 * @param string $reason Machine-readable reason for diagnostics.
	 * @param string $label  Human-readable label.
	 * @return string Rendered HTML chip.
	 */
	function dailyos_meeting_touchpoints_feed_empty_chip( string $reason, string $label ): string {
		return sprintf(
			'<section class="meeting-intel_chapterSection meeting-intel_chapterSection--empty is-empty" data-ds-name="MeetingTouchpointsFeed"><span class="dailyos-empty-chip" data-empty-reason="%s">%s</span></section>',
			esc_attr( $reason ),
			esc_html( $label )
		);
	}
}
