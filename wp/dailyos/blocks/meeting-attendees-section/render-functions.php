<?php
/**
 * MeetingAttendeesSection inner-block server-side render.
 *
 * Translation of `.docs/design/reference/surfaces/meeting.html` lines
 * 535-616 — the "The Room" attendee list on the meeting briefing page.
 *
 * Reads `dailyos/entityId` from outer-block context (provided by
 * `dailyos/meeting-detail`), invokes `get_entity_intelligence`
 * (entity_type=meeting) via the runtime client, and projects the top-level
 * `attendees` array from the meeting envelope.
 *
 * @package DailyOS
 *
 * @var array<string, mixed> $attributes Block attributes.
 */

declare(strict_types=1);

if ( ! defined( 'ABSPATH' ) ) {
	return '';
}

if ( ! function_exists( 'dailyos_meeting_attendees_section_render' ) ) {
	/**
	 * Render the meeting-attendees-section inner block.
	 *
	 * @param array<string, mixed> $attributes Block attributes.
	 * @param mixed                $block      WP_Block instance (provides context).
	 * @return string Rendered HTML.
	 */
	function dailyos_meeting_attendees_section_render( array $attributes, $block = null ): string {
		$meeting_id = '';
		if ( is_object( $block ) && isset( $block->context ) && is_array( $block->context ) ) {
			$meeting_id = isset( $block->context['dailyos/entityId'] )
				? (string) $block->context['dailyos/entityId']
				: '';
		}

		if ( '' === $meeting_id ) {
			return dailyos_meeting_attendees_section_empty_chip( 'missing_meeting_context', __( 'No meeting context.', 'dailyos' ) );
		}

		$runtime_client = apply_filters( 'dailyos_runtime_client_for_block', null );
		if ( ! is_object( $runtime_client ) || ! method_exists( $runtime_client, 'invoke_ability' ) ) {
			return dailyos_meeting_attendees_section_empty_chip( 'runtime_unavailable', __( 'Attendees unavailable.', 'dailyos' ) );
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

		if ( is_wp_error( $response ) ) {
			return dailyos_meeting_attendees_section_empty_chip( 'envelope_error', __( 'Attendees unavailable.', 'dailyos' ) );
		}

		$attendees = dailyos_meeting_attendees_section_extract_attendees( $response );
		if ( null === $attendees || empty( $attendees ) ) {
			return dailyos_meeting_attendees_section_empty_chip( 'no_attendees', __( 'No attendees on record.', 'dailyos' ) );
		}

		$wrapper_attrs = function_exists( 'get_block_wrapper_attributes' )
			? get_block_wrapper_attributes(
				[
					'class'        => 'wp-block-dailyos-meeting-attendees-section meeting-intel_chapterSection',
					'data-ds-tier' => 'pattern',
					'data-ds-name' => 'MeetingAttendeesSection',
					'data-ds-spec' => 'patterns/MeetingAttendeesSection.md',
				]
			)
			: 'class="wp-block-dailyos-meeting-attendees-section meeting-intel_chapterSection" data-ds-tier="pattern" data-ds-name="MeetingAttendeesSection" data-ds-spec="patterns/MeetingAttendeesSection.md"';

		$avatar_classes = [
			'default' => 'meeting-intel_attendeeAvatarDefault',
			'cold'    => 'meeting-intel_attendeeAvatarCold',
			'new'     => 'meeting-intel_attendeeAvatarNew',
			'self'    => 'meeting-intel_attendeeAvatarDefault',
		];
		$temperature_classes = [
			'warm' => 'Warm',
			'cold' => 'Cold',
			'cool' => 'Cool',
		];
		$engagement_classes = [
			'champion'  => 'Champion',
			'detractor' => 'Detractor',
			'supporter' => 'Supporter',
			'tentative' => 'Tentative',
		];

		$out  = '<section ' . $wrapper_attrs . '>';
		$out .= '<div class="ChapterHeading_heading">';
		$out .= '<hr class="ChapterHeading_rule" />';
		$out .= '<div class="ChapterHeading_titleRow">';
		$out .= '<h2 class="ChapterHeading_title">' . esc_html__( 'The Room', 'dailyos' ) . '</h2>';
		$out .= '</div>';
		$out .= '</div>';
		$out .= '<div class="meeting-intel_attendeeList">';

		foreach ( $attendees as $index => $attendee ) {
			if ( ! is_array( $attendee ) ) {
				return dailyos_meeting_attendees_section_empty_chip( 'invalid_attendee', __( 'Attendee data unavailable.', 'dailyos' ) );
			}

			$is_self = ( isset( $attendee['temperature'] ) && 'self' === (string) $attendee['temperature'] )
				|| ( isset( $attendee['engagement'] ) && 'self' === (string) $attendee['engagement'] )
				|| ( isset( $attendee['avatar_style'] ) && 'self' === (string) $attendee['avatar_style'] );
			$required_fields = [ 'person_id', 'display_name', 'avatar_initial', 'avatar_style' ];
			if ( ! $is_self ) {
				$required_fields = array_merge(
					$required_fields,
					[ 'role', 'organization', 'temperature', 'engagement', 'assessment', 'meeting_count', 'last_seen_label' ]
				);
			}

			foreach ( $required_fields as $field ) {
				if ( ! array_key_exists( $field, $attendee ) || '' === (string) $attendee[ $field ] ) {
					return dailyos_meeting_attendees_section_empty_chip(
						'missing_attendee_' . $field,
						__( 'Attendee data unavailable.', 'dailyos' )
					);
				}
			}

			$person_id       = (string) $attendee['person_id'];
			$display_name    = (string) $attendee['display_name'];
			$avatar_initial  = (string) $attendee['avatar_initial'];
			$avatar_style    = (string) $attendee['avatar_style'];
			$avatar_class    = $avatar_classes[ $avatar_style ] ?? '';
			$person_href     = '/people/' . rawurlencode( $person_id ) . '/';
			$attendee_number = is_int( $index ) ? $index + 1 : 0;

			if ( '' === $avatar_class ) {
				return dailyos_meeting_attendees_section_empty_chip(
					'invalid_attendee_avatar_style_' . (string) $attendee_number,
					__( 'Attendee data unavailable.', 'dailyos' )
				);
			}

			$out .= '<div class="meeting-intel_attendeeRowOuter">';
			$out .= '<a class="meeting-intel_attendeeLink" href="' . esc_url( $person_href ) . '">';
			$out .= '<div class="meeting-intel_attendeeRow">';
			$out .= '<div class="meeting-intel_attendeeAvatar ' . esc_attr( $avatar_class ) . '">'
				. esc_html( $avatar_initial )
				. '</div>';
			$out .= '<div class="meeting-intel_attendeeBody">';
			$out .= '<div class="meeting-intel_attendeeNameRow">';

			if ( ! $is_self && isset( $attendee['tooltip_assessment'] ) && '' !== (string) $attendee['tooltip_assessment'] ) {
				$meeting_count_label = (string) $attendee['meeting_count'] . ' meetings';
				$out .= '<span class="attendee-tooltip-wrap">';
				$out .= '<p class="meeting-intel_attendeeName">' . esc_html( $display_name ) . '</p>';
				$out .= '<span class="attendee-tooltip">';
				$out .= '<span class="meeting-intel_attendeeMetaMono meeting-intel_tooltipMetaBlockSpaced">'
					. esc_html( (string) $attendee['last_seen_label'] )
					. ' &middot; '
					. esc_html( $meeting_count_label )
					. '</span>';
				$out .= '<span class="meeting-intel_tooltipAssessment">'
					. esc_html( (string) $attendee['tooltip_assessment'] )
					. '</span>';
				$out .= '</span>';
				$out .= '</span>';
			} else {
				$out .= '<p class="meeting-intel_attendeeName">' . esc_html( $display_name ) . '</p>';
			}

			if ( $is_self ) {
				$out .= '<span class="meeting-intel_attendeeRole">' . esc_html__( 'You', 'dailyos' ) . '</span>';
				$out .= '</div>';
				$out .= '</div>';
				$out .= '</div>';
				$out .= '</a>';
				$out .= '</div>';
				continue;
			}

			$temperature = (string) $attendee['temperature'];
			$engagement  = (string) $attendee['engagement'];
			$temp_class  = $temperature_classes[ $temperature ] ?? '';
			if ( '' === $temp_class ) {
				return dailyos_meeting_attendees_section_empty_chip(
					'invalid_attendee_temperature_' . (string) $attendee_number,
					__( 'Attendee data unavailable.', 'dailyos' )
				);
			}

			$out .= '<span class="meeting-intel_attendeeRole">' . esc_html( (string) $attendee['role'] ) . '</span>';
			$out .= '<span class="meeting-intel_attendeeTempDot">';
			$out .= '<span class="meeting-intel_attendeeTempIndicator meeting-intel_tempIndicator' . esc_attr( $temp_class ) . '"></span>';
			$out .= '<span class="meeting-intel_attendeeTempLabel meeting-intel_tempLabel' . esc_attr( $temp_class ) . '">'
				. esc_html( $temperature )
				. '</span>';
			$out .= '</span>';

			if ( 'new_contact' === $engagement ) {
				$out .= '<span class="meeting-intel_attendeeNewContact">' . esc_html__( 'New contact', 'dailyos' ) . '</span>';
			} else {
				$engagement_class = $engagement_classes[ $engagement ] ?? '';
				if ( '' === $engagement_class ) {
					return dailyos_meeting_attendees_section_empty_chip(
						'invalid_attendee_engagement_' . (string) $attendee_number,
						__( 'Attendee data unavailable.', 'dailyos' )
					);
				}
				$out .= '<span class="meeting-intel_attendeeEngagement meeting-intel_engagement' . esc_attr( $engagement_class ) . '">'
					. esc_html( $engagement )
					. '</span>';
			}

			$meeting_count_label = (string) $attendee['meeting_count'] . ' meetings';
			$out .= '</div>';
			$out .= '<p class="meeting-intel_attendeeAssessment">' . esc_html( (string) $attendee['assessment'] ) . '</p>';
			$out .= '<div class="meeting-intel_attendeeMeta">';
			$out .= '<span class="meeting-intel_attendeeOrg">' . esc_html( (string) $attendee['organization'] ) . '</span>';
			$out .= '<span class="meeting-intel_attendeeMetaMono">' . esc_html( $meeting_count_label ) . '</span>';
			$out .= '<span class="meeting-intel_attendeeMetaMono">' . esc_html( (string) $attendee['last_seen_label'] ) . '</span>';
			$out .= '</div>';
			$out .= '</div>';
			$out .= '</div>';
			$out .= '</a>';
			$out .= '</div>';
		}

		$out .= '</div>';
		$out .= '</section>';

		return $out;
	}
}

if ( ! function_exists( 'dailyos_meeting_attendees_section_extract_attendees' ) ) {
	/**
	 * Pull the attendees array from an EntityIntelligenceEnvelope response.
	 *
	 * @param array<string, mixed> $response Raw runtime client response.
	 * @return array<int, array<string, mixed>>|null
	 */
	function dailyos_meeting_attendees_section_extract_attendees( array $response ): ?array {
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
		$attendees = $envelope['attendees'] ?? null;
		if ( ! is_array( $attendees ) ) {
			return null;
		}
		return $attendees;
	}
}

if ( ! function_exists( 'dailyos_meeting_attendees_section_empty_chip' ) ) {
	/**
	 * Visible empty-state chip per §10 invariant — never silent-hidden.
	 *
	 * @param string $reason Machine-readable reason for diagnostics.
	 * @param string $label  Human-readable label.
	 * @return string Rendered HTML chip.
	 */
	function dailyos_meeting_attendees_section_empty_chip( string $reason, string $label ): string {
		return sprintf(
			'<span class="dailyos-empty-chip wp-block-dailyos-meeting-attendees-section is-empty" data-empty-reason="%s">%s</span>',
			esc_attr( $reason ),
			esc_html( $label )
		);
	}
}
