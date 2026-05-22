<?php
/**
 * MeetingClaimsForReview inner-block server-side render.
 *
 * Translation of the Risks section from
 * `.docs/design/reference/surfaces/meeting.html` lines 421-484.
 *
 * Reads `dailyos/entityId` from outer-block context (provided by
 * `dailyos/meeting-detail`), invokes `get_entity_intelligence`
 * (entity_type=meeting) via the runtime client, and projects the
 * envelope-level `risks` array.
 *
 * @package DailyOS
 *
 * @var array<string, mixed> $attributes Block attributes.
 */

declare(strict_types=1);

if ( ! defined( 'ABSPATH' ) ) {
	return '';
}

if ( ! function_exists( 'dailyos_meeting_claims_for_review_render' ) ) {
	/**
	 * Render the meeting-claims-for-review inner block.
	 *
	 * @param array<string, mixed> $attributes Block attributes.
	 * @param mixed                $block      WP_Block instance (provides context).
	 * @return string Rendered HTML.
	 */
	function dailyos_meeting_claims_for_review_render( array $attributes, $block = null ): string {
		$meeting_id = '';
		if ( is_object( $block ) && isset( $block->context ) && is_array( $block->context ) ) {
			$meeting_id = isset( $block->context['dailyos/entityId'] )
				? (string) $block->context['dailyos/entityId']
				: '';
		}

		if ( '' === $meeting_id ) {
			return dailyos_meeting_claims_for_review_empty_chip( 'missing_meeting_context', __( 'No meeting context.', 'dailyos' ) );
		}

		$runtime_client = apply_filters( 'dailyos_runtime_client_for_block', null );
		if ( ! is_object( $runtime_client ) || ! method_exists( $runtime_client, 'invoke_ability' ) ) {
			return dailyos_meeting_claims_for_review_empty_chip( 'runtime_unavailable', __( 'Risks unavailable.', 'dailyos' ) );
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

		if ( is_wp_error( $response ) || ! is_array( $response ) ) {
			return dailyos_meeting_claims_for_review_empty_chip( 'envelope_error', __( 'Risks unavailable.', 'dailyos' ) );
		}

		$risks = dailyos_meeting_claims_for_review_extract_risks( $response );
		if ( null === $risks ) {
			return dailyos_meeting_claims_for_review_empty_chip( 'no_risks', __( 'No risks surfaced.', 'dailyos' ) );
		}

		$out  = '<section id="risks" class="editorial-reveal meeting-intel_chapterSection">';
		$out .= '<div class="ChapterHeading_heading">';
		$out .= '<hr class="ChapterHeading_rule">';
		$out .= '<div class="ChapterHeading_titleRow">';
		$out .= '<h2 class="ChapterHeading_title">' . esc_html__( 'The Risks', 'dailyos' ) . '</h2>';
		$out .= '</div>';
		$out .= '</div>';
		$out .= '<div class="meeting-intel_risksContainer">';

		foreach ( $risks as $risk ) {
			if ( 'featured' !== $risk['rank'] ) {
				continue;
			}
			$out .= '<blockquote class="meeting-intel_featuredRisk">';
			$out .= '<div class="meeting-intel_intelRowBody">';
			$out .= '<p class="EditableText_editable meeting-intel_featuredRiskText" data-editable-text data-claim-id="' . esc_attr( $risk['claim_id'] ) . '">'
				. esc_html( $risk['text'] )
				. '</p>';
			$out .= '<!-- // TODO(IntelligenceFeedback): wire helpful/not-helpful buttons via record_claim_feedback ability -->';
			$out .= '</div>';
			$out .= '</blockquote>';
		}

		foreach ( $risks as $risk ) {
			if ( 'subordinate' !== $risk['rank'] ) {
				continue;
			}
			$class_name = 'meeting-intel_subordinateRisk';
			if ( 'high' === $risk['urgency'] ) {
				$class_name .= ' meeting-intel_subordinateRiskHighUrgency';
			} elseif ( 'low' === $risk['urgency'] ) {
				$class_name .= ' meeting-intel_subordinateRiskLowUrgency';
			}
			$out .= '<div class="' . esc_attr( $class_name ) . '">';
			$out .= '<div class="meeting-intel_intelRowBody">';
			$out .= '<p class="EditableText_editable meeting-intel_subordinateRiskText" data-editable-text data-claim-id="' . esc_attr( $risk['claim_id'] ) . '">'
				. esc_html( $risk['text'] )
				. '</p>';
			$out .= '<!-- // TODO(IntelligenceFeedback): wire helpful/not-helpful buttons via record_claim_feedback ability -->';
			$out .= '</div>';
			$out .= '</div>';
		}

		$out .= '</div>';
		$out .= '</section>';

		return $out;
	}
}

if ( ! function_exists( 'dailyos_meeting_claims_for_review_extract_risks' ) ) {
	/**
	 * Pull the top-level risks array from an EntityIntelligenceEnvelope
	 * response envelope.
	 *
	 * @param array<string, mixed> $response Raw runtime client response.
	 * @return array<int, array{rank:string,urgency:string,text:string,claim_id:string}>|null
	 */
	function dailyos_meeting_claims_for_review_extract_risks( array $response ): ?array {
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

		$risks_section = $envelope['risks'] ?? null;
		if ( ! is_array( $risks_section ) || empty( $risks_section ) ) {
			return null;
		}

		$risks = [];
		foreach ( $risks_section as $item ) {
			if ( ! is_array( $item ) ) {
				continue;
			}
			$rank = isset( $item['rank'] ) ? (string) $item['rank'] : '';
			if ( ! in_array( $rank, [ 'featured', 'subordinate' ], true ) ) {
				continue;
			}

			$text = isset( $item['text'] ) ? trim( (string) $item['text'] ) : '';
			if ( '' === $text ) {
				continue;
			}

			$urgency = isset( $item['urgency'] ) ? (string) $item['urgency'] : 'medium';
			if ( ! in_array( $urgency, [ 'high', 'medium', 'low' ], true ) ) {
				$urgency = 'medium';
			}

			$risks[] = [
				'rank'     => $rank,
				'urgency'  => $urgency,
				'text'     => $text,
				'claim_id' => isset( $item['claim_id'] ) ? (string) $item['claim_id'] : '',
			];
		}

		if ( empty( $risks ) ) {
			return null;
		}

		return $risks;
	}
}

if ( ! function_exists( 'dailyos_meeting_claims_for_review_empty_chip' ) ) {
	/**
	 * Visible empty-state chip per §10 invariant - never silent-hidden.
	 *
	 * @param string $reason Machine-readable reason for diagnostics.
	 * @param string $label  Human-readable label.
	 * @return string Rendered HTML chip.
	 */
	function dailyos_meeting_claims_for_review_empty_chip( string $reason, string $label ): string {
		return sprintf(
			'<section class="wp-block-dailyos-meeting-claims-for-review is-empty" data-empty-reason="%s"><p class="meeting-intel_recordOverline">%s</p></section>',
			esc_attr( $reason ),
			esc_html( $label )
		);
	}
}
