<?php
/**
 * MeetingContextBundle inner-block server-side render.
 *
 * Translation of the PostMeetingIntelligence section from
 * `.docs/design/reference/surfaces/meeting.html` lines 59-160 — the
 * post-meeting summary, thread list, and predictions vs reality groups.
 *
 * Reads `dailyos/entityId` from outer-block context (provided by
 * `dailyos/meeting-detail`), invokes `get_entity_intelligence`
 * (entity_type=meeting) via the runtime client, and projects
 * envelope-level `post_meeting_intelligence`.
 *
 * @package DailyOS
 *
 * @var array<string, mixed> $attributes Block attributes.
 */

declare(strict_types=1);

if ( ! defined( 'ABSPATH' ) ) {
	return '';
}

if ( ! function_exists( 'dailyos_meeting_context_bundle_render' ) ) {
	/**
	 * Render the meeting-context-bundle inner block.
	 *
	 * @param array<string, mixed> $attributes Block attributes.
	 * @param mixed                $block      WP_Block instance (provides context).
	 * @return string Rendered HTML.
	 */
	function dailyos_meeting_context_bundle_render( array $attributes, $block = null ): string {
		$meeting_id = '';
		if ( is_object( $block ) && isset( $block->context ) && is_array( $block->context ) ) {
			$meeting_id = isset( $block->context['dailyos/entityId'] )
				? (string) $block->context['dailyos/entityId']
				: '';
		}

		if ( '' === $meeting_id ) {
			return dailyos_meeting_context_bundle_empty_chip(
				'missing_meeting_context',
				__( 'No meeting context.', 'dailyos' )
			);
		}

		$runtime_client = apply_filters( 'dailyos_runtime_client_for_block', null );
		if ( ! is_object( $runtime_client ) || ! method_exists( $runtime_client, 'invoke_ability' ) ) {
			return dailyos_meeting_context_bundle_empty_chip(
				'runtime_unavailable',
				__( 'Post-meeting intelligence is not generated yet.', 'dailyos' )
			);
		}

		$scope_set = apply_filters( 'dailyos_surfaceclient_resolved_scopes', [] );
		if ( ! is_array( $scope_set ) ) {
			$scope_set = [];
		}

		// W1 producer: get_entity_intelligence (entity_type=meeting).
		$response = $runtime_client->invoke_ability(
			'get_entity_intelligence',
			[
				'entity_type' => 'meeting',
				'entity_id'   => $meeting_id,
			],
			$scope_set
		);

		if ( is_wp_error( $response ) ) {
			return dailyos_meeting_context_bundle_empty_chip(
				'envelope_error',
				__( 'Post-meeting intelligence is not generated yet.', 'dailyos' )
			);
		}

		$extraction = dailyos_meeting_context_bundle_extract_post_meeting_intelligence( $response );
		$status     = isset( $extraction['status'] ) ? (string) $extraction['status'] : 'envelope_error';
		if ( 'ok' !== $status ) {
			return dailyos_meeting_context_bundle_empty_chip(
				$status,
				__( 'Post-meeting intelligence is not generated yet.', 'dailyos' )
			);
		}

		$post_meeting_intelligence = isset( $extraction['post_meeting_intelligence'] ) && is_array( $extraction['post_meeting_intelligence'] )
			? $extraction['post_meeting_intelligence']
			: [];
		$summary                   = isset( $post_meeting_intelligence['summary'] )
			? trim( (string) $post_meeting_intelligence['summary'] )
			: '';
		$thread_items              = isset( $post_meeting_intelligence['thread_items'] ) && is_array( $post_meeting_intelligence['thread_items'] )
			? $post_meeting_intelligence['thread_items']
			: [];
		$predictions               = isset( $post_meeting_intelligence['predictions'] ) && is_array( $post_meeting_intelligence['predictions'] )
			? $post_meeting_intelligence['predictions']
			: [];

		$wrapper_attrs = function_exists( 'get_block_wrapper_attributes' )
			? get_block_wrapper_attributes(
				[
					'class'        => 'wp-block-dailyos-meeting-context-bundle PostMeetingIntelligence_container',
					'data-ds-tier' => 'pattern',
					'data-ds-name' => 'PostMeetingIntelligence',
					'data-ds-spec' => 'patterns/PostMeetingIntelligence.md',
				]
			)
			: 'class="wp-block-dailyos-meeting-context-bundle PostMeetingIntelligence_container" data-ds-tier="pattern" data-ds-name="PostMeetingIntelligence"';

		$thread_icons               = [
			'confirmed' => '<svg class="PostMeetingIntelligence_threadIcon PostMeetingIntelligence_threadIconConfirmed" xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" width="14" height="14" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><path d="M20 6 9 17l-5-5"/></svg>',
			'open'      => '<svg class="PostMeetingIntelligence_threadIcon PostMeetingIntelligence_threadIconOpen" xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" width="14" height="14" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><circle cx="12" cy="12" r="10"/></svg>',
			'neutral'   => '<span class="PostMeetingIntelligence_threadIcon PostMeetingIntelligence_threadIconNeutral"><svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" width="12" height="12" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><circle cx="12" cy="12" r="10"/><circle cx="12" cy="12" r="1"/></svg></span>',
			'new_face'  => '<svg class="PostMeetingIntelligence_threadIcon PostMeetingIntelligence_threadIconNewFace" xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" width="14" height="14" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><path d="M16 21v-2a4 4 0 0 0-4-4H6a4 4 0 0 0-4 4v2"/><circle cx="9" cy="7" r="4"/><line x1="19" y1="8" x2="19" y2="14"/><line x1="22" y1="11" x2="16" y2="11"/></svg>',
		];
		$prediction_icon_confirmed  = '<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" width="14" height="14" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><path d="M20 6 9 17l-5-5"/></svg>';
		$prediction_icon_not_raised = '<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" width="14" height="14" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><path d="M18 6 6 18"/><path d="m6 6 12 12"/></svg>';
		$prediction_groups          = [
			'risks'         => [
				'label' => __( 'Risks', 'dailyos' ),
				'tone'  => 'PostMeetingIntelligence_monoLabelTerracotta',
			],
			'opportunities' => [
				'label' => __( 'Opportunities', 'dailyos' ),
				'tone'  => 'PostMeetingIntelligence_monoLabelSage',
			],
		];

		$out  = '<div ' . $wrapper_attrs . '>';
		$out .= '<div class="PostMeetingIntelligence_summaryBlock">';
		$out .= '<p class="PostMeetingIntelligence_summaryText">' . esc_html( $summary ) . '</p>';
		$out .= '</div>';

		$out .= '<section class="PostMeetingIntelligence_chapter">';
		$out .= '<div class="ChapterHeading_heading"><hr class="ChapterHeading_rule"><div class="ChapterHeading_titleRow"><h2 class="ChapterHeading_title">'
			. esc_html__( 'The Thread', 'dailyos' )
			. '</h2></div></div>';
		$out .= '<p class="PostMeetingIntelligence_threadIntro">'
			. esc_html__( "Since last touchpoint, here's what changed.", 'dailyos' )
			. '</p>';
		$out .= '<ul class="PostMeetingIntelligence_threadList">';
		foreach ( $thread_items as $thread_item ) {
			if ( ! is_array( $thread_item ) ) {
				continue;
			}
			$kind     = isset( $thread_item['kind'] ) ? (string) $thread_item['kind'] : '';
			$headline = isset( $thread_item['headline'] ) ? trim( (string) $thread_item['headline'] ) : '';
			$detail   = isset( $thread_item['detail'] ) ? trim( (string) $thread_item['detail'] ) : '';
			if ( '' === $headline || ! isset( $thread_icons[ $kind ] ) ) {
				continue;
			}
			$out .= '<li class="PostMeetingIntelligence_threadItem">';
			$out .= $thread_icons[ $kind ];
			$out .= '<span>' . esc_html( $headline );
			if ( '' !== $detail ) {
				$out .= ' <span class="PostMeetingIntelligence_threadDetail">' . esc_html( $detail ) . '</span>';
			}
			$out .= '</span></li>';
		}
		$out .= '</ul></section>';

		$out .= '<section class="PostMeetingIntelligence_chapter">';
		$out .= '<div class="ChapterHeading_heading"><hr class="ChapterHeading_rule"><div class="ChapterHeading_titleRow"><h2 class="ChapterHeading_title">'
			. esc_html__( 'What We Predicted vs What Happened', 'dailyos' )
			. '</h2></div></div>';
		foreach ( $prediction_groups as $group_key => $group ) {
			$items = isset( $predictions[ $group_key ] ) && is_array( $predictions[ $group_key ] )
				? $predictions[ $group_key ]
				: [];

			$out .= '<div class="PostMeetingIntelligence_predictionGroup">';
			$out .= '<p class="PostMeetingIntelligence_monoLabel ' . esc_attr( $group['tone'] ) . '">'
				. esc_html( $group['label'] )
				. '</p>';

			foreach ( $items as $prediction_item ) {
				if ( ! is_array( $prediction_item ) || ! array_key_exists( 'matched', $prediction_item ) || ! is_bool( $prediction_item['matched'] ) ) {
					continue;
				}
				$prediction = isset( $prediction_item['prediction'] ) ? trim( (string) $prediction_item['prediction'] ) : '';
				$reality    = isset( $prediction_item['reality'] ) ? trim( (string) $prediction_item['reality'] ) : '';
				if ( '' === $prediction ) {
					continue;
				}

				$item_class = $prediction_item['matched']
					? 'PostMeetingIntelligence_predictionItemConfirmed'
					: 'PostMeetingIntelligence_predictionItemNotRaised';
				$icon_class = $prediction_item['matched']
					? 'PostMeetingIntelligence_predictionIconConfirmed'
					: 'PostMeetingIntelligence_predictionIconNotRaised';
				$icon       = $prediction_item['matched']
					? $prediction_icon_confirmed
					: $prediction_icon_not_raised;

				$out .= '<div class="PostMeetingIntelligence_predictionItem ' . esc_attr( $item_class ) . '">';
				$out .= '<span class="PostMeetingIntelligence_predictionIcon ' . esc_attr( $icon_class ) . '">' . $icon . '</span>';
				$out .= '<div><p>' . esc_html( $prediction ) . '</p>';
				if ( '' !== $reality ) {
					$out .= '<p class="PostMeetingIntelligence_predictionMatch">' . esc_html( $reality ) . '</p>';
				}
				$out .= '</div></div>';
			}

			$out .= '</div>';
		}
		$out .= '</section></div>';

		return $out;
	}
}

if ( ! function_exists( 'dailyos_meeting_context_bundle_extract_post_meeting_intelligence' ) ) {
	/**
	 * Pull post-meeting intelligence from an EntityIntelligenceEnvelope response.
	 *
	 * Returns a status array so callers can distinguish a malformed envelope
	 * from the expected pre-meeting state where post-meeting intelligence is
	 * present as null.
	 *
	 * @param array<string, mixed> $response Raw runtime client response.
	 * @return array<string, mixed>
	 */
	function dailyos_meeting_context_bundle_extract_post_meeting_intelligence( array $response ): array {
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
			return [ 'status' => 'envelope_error' ];
		}

		if ( ! array_key_exists( 'post_meeting_intelligence', $envelope ) || null === $envelope['post_meeting_intelligence'] ) {
			return [ 'status' => 'no_post_intel' ];
		}

		if ( ! is_array( $envelope['post_meeting_intelligence'] ) ) {
			return [ 'status' => 'envelope_error' ];
		}

		return [
			'status'                    => 'ok',
			'post_meeting_intelligence' => $envelope['post_meeting_intelligence'],
		];
	}
}

if ( ! function_exists( 'dailyos_meeting_context_bundle_empty_chip' ) ) {
	/**
	 * Visible empty-state chip per §10 invariant — never silent-hidden.
	 *
	 * @param string $reason Machine-readable reason for diagnostics.
	 * @param string $label  Human-readable label.
	 * @return string Rendered HTML chip.
	 */
	function dailyos_meeting_context_bundle_empty_chip( string $reason, string $label ): string {
		return sprintf(
			'<span class="dailyos-empty-chip wp-block-dailyos-meeting-context-bundle is-empty" data-empty-reason="%s">%s</span>',
			esc_attr( $reason ),
			esc_html( $label )
		);
	}
}
