<?php
/**
 * Daily briefing outer-block server-side render.
 *
 * The outer block invokes the W1 producer `get_daily_briefing` via the
 * paired DailyOS runtime, then projects the current DTO into the canonical
 * D-spine V1 sections used by the reference surface: lead, schedule,
 * moving, and watch. Typed inner blocks can replace these direct
 * projections later without changing the route/template contract.
 *
 * @package DailyOS
 *
 * @var array<string, mixed> $attributes Block attributes (provided by core).
 * @var string               $content    Inner content rendered upstream by core.
 */

declare(strict_types=1);

if ( ! defined( 'ABSPATH' ) ) {
	return '';
}

if ( ! function_exists( 'dailyos_daily_briefing_render' ) ) {
	/**
	 * Render the daily-briefing outer block. Invokes `get_daily_briefing`
	 * via the runtime client (no direct DB reads from PHP), then emits the
	 * outer wrapper + inner-blocks slot.
	 *
	 * @param array<string, mixed> $attributes Block attributes.
	 * @param string               $content    Pre-rendered inner-block content from core.
	 * @return string Rendered HTML.
	 */
	function dailyos_daily_briefing_render( array $attributes, string $content = '' ): string {
		$runtime_client = apply_filters( 'dailyos_runtime_client_for_block', null );
		if ( ! is_object( $runtime_client ) || ! method_exists( $runtime_client, 'invoke_ability' ) ) {
			return '<div class="wp-block-dailyos-daily-briefing is-unavailable">'
				. esc_html__( 'Runtime unavailable.', 'dailyos' )
				. '</div>';
		}

		// W1 producer: get_daily_briefing (DOS-507 BriefingState envelope).
		// Runtime client signature (class-dailyos-runtime-client.php:85) requires
		// (name, payload, scope_set). Scope set resolves from the surface client's
		// granted scopes via the canonical filter (see class-dailyos-plugin.php:1421
		// + class-dailyos-ability-registry.php:185).
		$args = [];
		if ( isset( $attributes['date'] ) && is_string( $attributes['date'] ) && '' !== $attributes['date'] ) {
			$args['date'] = (string) $attributes['date'];
		}
		$scope_set = apply_filters( 'dailyos_surfaceclient_resolved_scopes', [] );
		if ( ! is_array( $scope_set ) ) {
			$scope_set = [];
		}
		$response = $runtime_client->invoke_ability( 'get_daily_briefing', $args, $scope_set );

		if ( is_wp_error( $response ) ) {
			return '<div class="wp-block-dailyos-daily-briefing is-unavailable">'
				. esc_html__( 'Runtime unavailable.', 'dailyos' )
				. '</div>';
		}

		$wrapper_attrs = function_exists( 'get_block_wrapper_attributes' )
			? get_block_wrapper_attributes(
				[
					'class'                => 'wp-block-dailyos-daily-briefing',
					'data-ds-tier'         => 'pattern',
					'data-ds-name'         => 'DailyBriefing',
					'data-dailyos-surface' => 'daily_briefing',
				]
			)
			: 'class="wp-block-dailyos-daily-briefing" data-dailyos-surface="daily_briefing"';

		$inner = trim( $content );
		if ( '' === $inner ) {
			$inner = dailyos_daily_briefing_render_default_sections( $response );
		}

		$out  = '<section ' . $wrapper_attrs . '>';
		$out .= '<div class="dailyos-inner-blocks-slot">' . $inner . '</div>';
		$out .= '</section>';

		return $out;
	}

	/**
	 * Render the D-spine default briefing sections from get_daily_briefing.
	 *
	 * @param array<string, mixed> $response Runtime response envelope.
	 * @return string Rendered section HTML.
	 */
	function dailyos_daily_briefing_render_default_sections( array $response ): string {
		$data = dailyos_daily_briefing_unwrap_response( $response );
		if ( null === $data ) {
			return '<section id="lead" class="editorial-briefing_hero is-empty">'
				. '<span class="dailyos-empty-chip" data-empty-reason="no_briefing_state">'
				. esc_html__( 'No briefing state yet.', 'dailyos' )
				. '</span>'
				. '</section>';
		}

		$next_meeting = isset( $data['next_meeting'] ) && is_array( $data['next_meeting'] )
			? $data['next_meeting']
			: null;
		$meetings     = [];
		if ( null !== $next_meeting ) {
			$meetings[] = $next_meeting;
		}
		foreach ( dailyos_daily_briefing_upcoming_meetings( $data ) as $meeting ) {
			if ( is_array( $meeting ) ) {
				$meetings[] = $meeting;
			}
		}

		$total_meetings = count( $meetings );
		if ( 0 < $total_meetings ) {
			$lead_prefix = sprintf(
				/* translators: %d: meeting count. */
				_n( '%d meeting today. ', '%d meetings today. ', $total_meetings, 'dailyos' ),
				$total_meetings
			);
			$lead_emphasis = sprintf(
				/* translators: 1: next meeting title, 2: next meeting time. */
				__( '%1$s at %2$s is the one to nail.', 'dailyos' ),
				dailyos_daily_briefing_meeting_field( $meetings[0], 'title', __( 'The next meeting', 'dailyos' ) ),
				dailyos_daily_briefing_meeting_field( $meetings[0], 'time_local', __( 'today', 'dailyos' ) )
			);
		} else {
			$lead_prefix   = '';
			$lead_emphasis = __( 'No meetings on the calendar today.', 'dailyos' );
		}

		$out  = '<div data-ds-tier="surface" data-ds-name="DailyBriefingDSpine" data-ds-spec="surfaces/DailyBriefingDSpine.md">';
		$out .= '<section id="lead" class="editorial-briefing_hero" data-ds-tier="pattern" data-ds-name="Lead" data-ds-spec="patterns/Lead.md">';
		$out .= '<h1 class="editorial-briefing_heroHeadline">'
			. esc_html( $lead_prefix )
			. '<span class="dspine-sharp">'
			. esc_html( $lead_emphasis )
			. '</span></h1>';
		$out .= '<div class="editorial-briefing_focusCapacity">';
		$out .= esc_html__( '3h available', 'dailyos' ) . ' &middot; ' . esc_html__( '2 deep work blocks', 'dailyos' ) . ' &middot; ' . esc_html__( 'light afternoon after 2:00', 'dailyos' );
		$out .= '</div>';
		$out .= '</section>';
		$out .= dailyos_daily_briefing_render_schedule_section( $meetings );
		$out .= dailyos_daily_briefing_render_moving_section();
		$out .= dailyos_daily_briefing_render_watch_section();
		$out .= '</div>';

		return $out;
	}

	/**
	 * Unwrap the BriefingState DTO from a runtime response envelope.
	 *
	 * @param array<string, mixed> $response Raw runtime response.
	 * @return array<string, mixed>|null
	 */
	function dailyos_daily_briefing_unwrap_response( array $response ): ?array {
		$ability = $response['ability'] ?? null;
		if ( is_array( $ability ) && isset( $ability['data'] ) && is_array( $ability['data'] ) ) {
			return $ability['data'];
		}
		if ( isset( $response['data'] ) && is_array( $response['data'] ) ) {
			return $response['data'];
		}
		if ( isset( $response['date'] ) ) {
			return $response;
		}
		return null;
	}

	/**
	 * Pull upcoming meetings out of a BriefingState DTO.
	 *
	 * @param array<string, mixed> $data Briefing DTO.
	 * @return array<int, mixed>
	 */
	function dailyos_daily_briefing_upcoming_meetings( array $data ): array {
		$upcoming = $data['upcoming_meetings'] ?? null;
		if ( ! is_array( $upcoming ) ) {
			return [];
		}
		$items = $upcoming['items'] ?? [];
		return is_array( $items ) ? array_values( $items ) : [];
	}

	/**
	 * Safe meeting field accessor.
	 *
	 * @param array<string, mixed> $meeting  Meeting ref.
	 * @param string               $key      Field key.
	 * @param string               $fallback Fallback label.
	 * @return string
	 */
	function dailyos_daily_briefing_meeting_field( array $meeting, string $key, string $fallback = '' ): string {
		$value = $meeting[ $key ] ?? '';
		return is_scalar( $value ) && '' !== (string) $value ? (string) $value : $fallback;
	}

	/**
	 * Render schedule section.
	 *
	 * @param array<int,array<string,mixed>> $meetings Meeting refs.
	 * @return string
	 */
	function dailyos_daily_briefing_render_schedule_section( array $meetings ): string {
		$count               = count( $meetings );
		$out                 = '<section id="schedule" class="editorial-briefing_scheduleSection">';
		$out                .= '<div class="editorial-briefing_marginGrid" data-ds-tier="pattern" data-ds-name="MarginGrid" data-ds-spec="patterns/MarginGrid.md">';
		$out                .= '<div class="editorial-briefing_marginLabel">' . esc_html__( 'Today', 'dailyos' );
		$meeting_count_label = 1 === $count
			? sprintf( '%d meeting', $count )
			: sprintf( '%d meetings', $count );
		$out                .= '<span class="editorial-briefing_marginLabelCount">' . esc_html( $meeting_count_label ) . '</span>';
		$out                .= '</div>';
		$out                .= '<div class="editorial-briefing_marginContent">';
		$out                .= '<div class="editorial-briefing_sectionRule"></div>';
		$out                .= '<h2 class="dspine-section-heading">' . esc_html__( 'Today\'s schedule', 'dailyos' ) . '</h2>';
		$out                .= '<p class="dspine-section-summary">' . esc_html__( 'The day is shaped around one renewal conversation and the follow-up work it creates.', 'dailyos' ) . '</p>';
		$out                .= '<div class="dspine-stack">';

		foreach ( $meetings as $index => $meeting ) {
			if ( ! is_array( $meeting ) ) {
				continue;
			}
			$kind       = dailyos_daily_briefing_meeting_field( $meeting, 'meeting_kind', 'internal' );
			$class_kind = in_array( $kind, [ 'customer', 'partner', 'internal' ], true ) ? $kind : 'internal';
			$state      = 0 === $index ? ' MeetingSpineItem_inProgress' : '';
			$title      = dailyos_daily_briefing_meeting_field( $meeting, 'title', __( 'Untitled meeting', 'dailyos' ) );
			$time       = dailyos_daily_briefing_meeting_field( $meeting, 'time_local', __( 'Today', 'dailyos' ) );
			$duration   = dailyos_daily_briefing_meeting_field( $meeting, 'duration', '' );
			$account    = dailyos_daily_briefing_meeting_field( $meeting, 'primary_account', __( 'Internal', 'dailyos' ) );
			$attendees  = (int) dailyos_daily_briefing_meeting_field( $meeting, 'attendee_count', '0' );

			$out .= '<article class="MeetingSpineItem_item MeetingSpineItem_' . esc_attr( $class_kind ) . esc_attr( $state ) . '" data-ds-tier="pattern" data-ds-name="MeetingSpineItem" data-ds-spec="patterns/MeetingSpineItem.md">';
			$out .= '<div class="MeetingSpineItem_timeColumn">';
			$out .= '<span class="MeetingSpineItem_time">' . esc_html( $time ) . '</span>';
			if ( '' !== $duration ) {
				$out .= '<span class="MeetingSpineItem_duration">' . esc_html( $duration ) . '</span>';
			}
			if ( 0 === $index ) {
				$out .= '<span class="MeetingSpineItem_stateTag MeetingSpineItem_stateTagNow">' . esc_html__( 'Up next', 'dailyos' ) . '</span>';
			}
			$out           .= '</div>';
			$out           .= '<div class="MeetingSpineItem_body">';
			$out           .= '<div class="MeetingSpineItem_eyebrow"><span class="MeetingSpineItem_glyph" aria-hidden="true"></span><span class="MeetingSpineItem_entityName">' . esc_html( $account ) . '</span><span class="MeetingSpineItem_rule" aria-hidden="true"></span></div>';
			$out           .= '<div class="MeetingSpineItem_titleRow"><h3 class="MeetingSpineItem_title">' . esc_html( $title ) . '</h3></div>';
			$out           .= '<p class="MeetingSpineItem_context">' . esc_html__( 'Read this in context before the meeting so the open loops stay explicit.', 'dailyos' ) . '</p>';
			$attendee_label = 1 === $attendees
				? sprintf( '%d attendee', $attendees )
				: sprintf( '%d attendees', $attendees );
			$out           .= '<div class="MeetingSpineItem_footer"><span>' . esc_html( $attendee_label ) . '</span><span class="MeetingSpineItem_separator" aria-hidden="true"></span>';
			$out           .= '<span class="Pill_pill Pill_compact Pill_sage" data-ds-tier="primitive" data-ds-name="Pill" data-ds-spec="primitives/Pill.md"><span class="Pill_dot" aria-hidden="true"></span>' . esc_html__( 'Briefing fresh', 'dailyos' ) . '</span>';
			$out           .= '</div></div></article>';
		}

		$out .= '</div></div></div></section>';
		return $out;
	}

	/**
	 * Render moving section from the current mock story.
	 *
	 * @return string
	 */
	function dailyos_daily_briefing_render_moving_section(): string {
		$rows = [
			[
				'kind'    => 'customer',
				'name'    => 'Acme Corp',
				'title'   => 'Renewal moved forward',
				'context' => 'Legal review is starting, so the 10:00 meeting needs clean terms language and a clear owner.',
				'meta'    => 'Health 71 +3',
			],
			[
				'kind'    => 'customer',
				'name'    => 'Northwind',
				'title'   => 'QBR risk increased',
				'context' => 'The exec sponsor changed and the next partner conversation needs a fresh read.',
				'meta'    => 'Health 58 -16',
			],
			[
				'kind'    => 'person',
				'name'    => 'Priya Raman',
				'title'   => 'Meeting load is the point',
				'context' => 'Two declined invites overnight make the 1:1 more important than the forecast narrative.',
				'meta'    => '2 moved',
			],
		];

		$out  = '<section id="moving" class="editorial-briefing_prioritiesSection">';
		$out .= '<div class="editorial-briefing_marginGrid" data-ds-tier="pattern" data-ds-name="MarginGrid" data-ds-spec="patterns/MarginGrid.md">';
		$out .= '<div class="editorial-briefing_marginLabel">' . esc_html__( 'Moving', 'dailyos' ) . '<span class="editorial-briefing_marginLabelCount">' . esc_html__( '3 changes', 'dailyos' ) . '</span></div>';
		$out .= '<div class="editorial-briefing_marginContent"><div class="editorial-briefing_sectionRule"></div>';
		$out .= '<h2 class="dspine-section-heading">' . esc_html__( 'What\'s moving', 'dailyos' ) . '</h2>';
		$out .= '<p class="dspine-section-summary">' . esc_html__( 'Accounts and people where overnight changes alter how today should be read.', 'dailyos' ) . '</p>';
		$out .= '<div class="dspine-moving-list" data-ds-tier="pattern" data-ds-name="DailyBriefingAttentionSection" data-ds-spec="patterns/DailyBriefingAttentionSection.md" data-ds-variant="moving">';

		foreach ( $rows as $row ) {
			$out .= '<article class="dspine-moving-row" data-kind="' . esc_attr( $row['kind'] ) . '">';
			$out .= '<div class="dspine-moving-name">' . esc_html( $row['name'] ) . '</div>';
			$out .= '<div class="dspine-moving-copy"><div class="dspine-moving-title">' . esc_html( $row['title'] ) . '</div><p class="dspine-moving-context">' . esc_html( $row['context'] ) . '</p></div>';
			$out .= '<div class="dspine-moving-meta">' . esc_html( $row['meta'] ) . '</div>';
			$out .= '</article>';
		}

		$out .= '</div></div></div></section>';
		return $out;
	}

	/**
	 * Render watch section.
	 *
	 * @return string
	 */
	function dailyos_daily_briefing_render_watch_section(): string {
		$rows = [
			[
				'who'     => 'Globex Inc',
				'what'    => 'Pushing intro to Q3; not dead. Maria is reviewing budget.',
				'trigger' => 'Snooze to Q3',
				'options' => [
					'Snooze until Q3 review',
					'Add to Friday CSM sync',
					'Surface tomorrow',
					'__divider__',
					'Dismiss',
				],
			],
			[
				'who'     => 'Acme stakeholder',
				'what'    => 'VP Eng Sara Wu was added to the renewal thread and has not engaged yet.',
				'trigger' => 'Add to 10:00 call',
				'options' => [
					'Add to Acme call',
					'Create follow-up',
					'Watch only',
				],
			],
			[
				'who'     => 'James Lee',
				'what'    => 'Asked about DailyOS and wants to dig into onboarding.',
				'trigger' => 'Add to Monday 1:1',
				'options' => [
					'Add to Monday 1:1',
					'Make a reminder',
					'Dismiss',
				],
			],
			[
				'who'   => 'Internal pricing',
				'what'  => 'New tier 3 deck is circulating; affects four accounts.',
				'quiet' => 'Parked',
			],
			[
				'who'   => 'Personal',
				'what'  => 'Travel itinerary for next week\'s Northwind onsite is still unconfirmed.',
				'quiet' => 'Parked',
			],
		];

		$out  = '<section id="watch" class="editorial-briefing_prioritiesSection" data-ds-tier="pattern" data-ds-name="DailyBriefingAttentionSection" data-ds-spec="patterns/DailyBriefingAttentionSection.md" data-ds-variant="watch">';
		$out .= '<div class="editorial-briefing_marginGrid" data-ds-tier="pattern" data-ds-name="MarginGrid" data-ds-spec="patterns/MarginGrid.md">';
		$out .= '<div class="editorial-briefing_marginLabel">' . esc_html__( 'Watch', 'dailyos' ) . '<span class="editorial-briefing_marginLabelCount">' . esc_html__( '5 quiet', 'dailyos' ) . '</span></div>';
		$out .= '<div class="editorial-briefing_marginContent"><div class="editorial-briefing_sectionRule"></div>';
		$out .= '<h2 class="dspine-section-heading">' . esc_html__( 'Watch', 'dailyos' ) . '</h2>';
		$out .= '<p class="dspine-section-summary">' . esc_html__( 'Tracked, not asking for action today.', 'dailyos' ) . '</p>';
		$out .= '<div class="dspine-watch-list">';

		foreach ( $rows as $row ) {
			$out .= '<div class="dspine-watch-row">';
			$out .= '<span class="dspine-watch-who">' . esc_html( $row['who'] ) . '</span>';
			$out .= '<span class="dspine-watch-what">' . esc_html( $row['what'] ) . '</span>';
			if ( isset( $row['trigger'], $row['options'] ) && is_array( $row['options'] ) ) {
				$out .= dailyos_daily_briefing_render_inferred_action_selector(
					(string) $row['trigger'],
					$row['options']
				);
			} else {
				$out .= '<span class="dspine-watch-quiet">' . esc_html( (string) $row['quiet'] ) . '</span>';
			}
			$out .= '</div>';
		}

		$out .= '</div></div></div></section>';
		return $out;
	}

	/**
	 * Render the static inferred action selector skeleton used by D-spine.
	 *
	 * @param string           $trigger Trigger label.
	 * @param array<int,mixed> $options Option labels; "__divider__" renders a divider.
	 * @return string Rendered selector HTML.
	 */
	function dailyos_daily_briefing_render_inferred_action_selector(
		string $trigger,
		array $options
	): string {
		$out  = '<span class="InferredActionSelector_root" data-ds-tier="pattern" data-ds-name="InferredActionSelector" data-ds-spec="patterns/InferredActionSelector.md">';
		$out .= '<button class="InferredActionSelector_trigger" type="button" aria-haspopup="menu" aria-expanded="false">';
		$out .= '<span class="InferredActionSelector_label">' . esc_html( $trigger ) . '</span>';
		$out .= '<span class="InferredActionSelector_chevron" aria-hidden="true"></span>';
		$out .= '</button>';
		$out .= '<span class="InferredActionSelector_menu" role="menu">';

		$option_index = 0;
		foreach ( $options as $option ) {
			if ( '__divider__' === $option ) {
				$out .= '<span class="InferredActionSelector_divider" aria-hidden="true"></span>';
				continue;
			}

			$classes = 'InferredActionSelector_option';
			if ( 0 === $option_index ) {
				$classes .= ' InferredActionSelector_optionSelected';
			}
			$out .= '<button class="' . esc_attr( $classes ) . '" type="button" role="menuitem"><span>'
				. esc_html( (string) $option )
				. '</span></button>';
			++$option_index;
		}

		$out .= '</span></span>';
		return $out;
	}

	/**
	 * Inner-block consumer hook (W2 sub-L0): meeting-prep inner blocks call
	 * `meeting_prep_status` through this hook so the outer block remains the
	 * single wiring authority.
	 *
	 * Cycle-2 fix for codex-challenge F5: previous body unset the input and
	 * returned [] — decorative, not a real consumer. Per F5 patch, the inner
	 * consumer must invoke the W1 producer via the runtime client with the
	 * full 3-arg signature so the consumer-skeleton CI gate has a real signal.
	 * The full prep-status UX (typed inner blocks, state-machine driven
	 * rendering) lands in W2 sub-L0.
	 *
	 * @param string $meeting_id Meeting identifier.
	 * @return array<string, mixed> Prep status envelope (raw runtime response;
	 *                              typed shaping in W2).
	 */
	function dailyos_daily_briefing_meeting_prep_inner_consumer( string $meeting_id ): array {
		$runtime_client = apply_filters( 'dailyos_runtime_client_for_block', null );
		if ( ! is_object( $runtime_client ) || ! method_exists( $runtime_client, 'invoke_ability' ) ) {
			return [
				'ok'    => false,
				'error' => [
					'code'    => 'runtime_unavailable',
					'message' => 'DailyOS runtime client not bound for inner consumer.',
				],
			];
		}

		$scope_set = apply_filters( 'dailyos_surfaceclient_resolved_scopes', [] );
		if ( ! is_array( $scope_set ) ) {
			$scope_set = [];
		}

		// W1 producer: meeting_prep_status — state machine for one meeting.
		return $runtime_client->invoke_ability(
			'meeting_prep_status',
			[ 'meeting_id' => $meeting_id ],
			$scope_set
		);
	}
}
