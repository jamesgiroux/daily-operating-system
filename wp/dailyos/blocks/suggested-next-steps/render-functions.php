<?php
/**
 * Suggested Next Steps server-side render.
 *
 * @package DailyOS
 */

declare(strict_types=1);

if ( ! defined( 'ABSPATH' ) ) {
	return '';
}

if ( ! function_exists( 'dailyos_suggested_next_steps_feedback_kinds' ) ) {
	require_once __DIR__ . '/feedback-kinds.php';
}

if ( ! function_exists( 'dailyos_suggested_next_steps_render' ) ) {
	/**
	 * Render the suggested-next-steps inner block.
	 *
	 * @param array<string, mixed> $attributes Block attributes.
	 * @param mixed                $block      WP_Block instance carrying context.
	 * @return string Rendered HTML.
	 */
	function dailyos_suggested_next_steps_render( array $attributes, $block = null ): string {
		$subject = dailyos_suggested_next_steps_subject_from_block( $block );
		if ( null === $subject ) {
			return dailyos_suggested_next_steps_render_empty_shell(
				null,
				'',
				'missing_subject_context',
				__( 'No subject context.', 'dailyos' )
			);
		}

		$runtime_client = apply_filters( 'dailyos_runtime_client_for_block', null );
		if ( ! is_object( $runtime_client ) || ! method_exists( $runtime_client, 'invoke_ability' ) ) {
			return dailyos_suggested_next_steps_render_empty_shell(
				$subject,
				dailyos_suggested_next_steps_heading( $subject, $attributes, null, [] ),
				'runtime_unavailable',
				dailyos_suggested_next_steps_unavailable_copy()
			);
		}

		$scope_set = apply_filters( 'dailyos_surfaceclient_resolved_scopes', [] );
		if ( ! is_array( $scope_set ) ) {
			$scope_set = [];
		}

		$max_items = dailyos_suggested_next_steps_max_items( $attributes );
		$response  = $runtime_client->invoke_ability(
			'list_suggested_next_steps',
			[
				'schemaVersion' => 1,
				'subject'       => dailyos_suggested_next_steps_subject_ref( $subject ),
				'surface'       => dailyos_suggested_next_steps_surface_context( $subject['type'] ),
				'maxItems'      => $max_items,
			],
			$scope_set
		);

		if ( is_wp_error( $response ) || ! is_array( $response ) || ( isset( $response['ok'] ) && false === $response['ok'] ) ) {
			return dailyos_suggested_next_steps_render_empty_shell(
				$subject,
				dailyos_suggested_next_steps_heading( $subject, $attributes, null, [] ),
				'envelope_error',
				dailyos_suggested_next_steps_unavailable_copy()
			);
		}

		$items = dailyos_suggested_next_steps_extract_items( $response );
		if ( null === $items ) {
			return dailyos_suggested_next_steps_render_empty_shell(
				$subject,
				dailyos_suggested_next_steps_heading( $subject, $attributes, $response, [] ),
				'envelope_error',
				dailyos_suggested_next_steps_unavailable_copy()
			);
		}

		$items   = array_slice( $items, 0, $max_items );
		$heading = dailyos_suggested_next_steps_heading( $subject, $attributes, $response, $items );

		if ( [] === $items ) {
			return dailyos_suggested_next_steps_render_empty_shell(
				$subject,
				$heading,
				'no_recommendations',
				dailyos_suggested_next_steps_empty_copy( $subject['type'] )
			);
		}

		$feedback_enabled = (bool) apply_filters( 'dailyos_suggested_next_steps_feedback_enabled', false, $subject, $attributes );
		$rows             = '';
		foreach ( $items as $index => $item ) {
			if ( ! is_array( $item ) ) {
				continue;
			}
			$row = dailyos_suggested_next_steps_render_row( $item, $feedback_enabled, (int) $index );
			if ( '' !== $row ) {
				$rows .= $row;
			}
		}

		if ( '' === $rows ) {
			return dailyos_suggested_next_steps_render_empty_shell(
				$subject,
				$heading,
				'no_recommendations',
				dailyos_suggested_next_steps_empty_copy( $subject['type'] )
			);
		}

		$body = '';
		if ( ! $feedback_enabled ) {
			$body .= '<span class="dailyos-info-chip" aria-live="polite">' . esc_html__( 'Feedback opens on the next sync.', 'dailyos' ) . '</span>';
		}
		$body .= '<div class="suggested-next-steps_list">';
		$body .= $rows;
		$body .= '</div>';

		return dailyos_suggested_next_steps_render_shell( $subject, $heading, $body, count( $items ), false );
	}
}

if ( ! function_exists( 'dailyos_suggested_next_steps_subject_from_block' ) ) {
	/**
	 * Resolve the entity subject from block context.
	 *
	 * @param mixed $block WP_Block-like instance.
	 * @return array{type: string, id: string, label: string}|null
	 */
	function dailyos_suggested_next_steps_subject_from_block( $block ): ?array {
		if ( ! is_object( $block ) || ! isset( $block->context ) || ! is_array( $block->context ) ) {
			return null;
		}

		$type = isset( $block->context['dailyos/entityType'] ) ? (string) $block->context['dailyos/entityType'] : '';
		$id   = isset( $block->context['dailyos/entityId'] ) ? (string) $block->context['dailyos/entityId'] : '';
		if ( ! in_array( $type, [ 'account', 'project', 'person', 'meeting' ], true ) || '' === $id ) {
			return null;
		}

		$label = '';
		foreach ( [ 'dailyos/entityLabel', 'dailyos/entityName' ] as $key ) {
			if ( isset( $block->context[ $key ] ) && is_string( $block->context[ $key ] ) && '' !== trim( $block->context[ $key ] ) ) {
				$label = trim( $block->context[ $key ] );
				break;
			}
		}

		return [
			'type'  => $type,
			'id'    => $id,
			'label' => $label,
		];
	}
}

if ( ! function_exists( 'dailyos_suggested_next_steps_subject_ref' ) ) {
	/**
	 * Build the runtime SubjectRef serde shape.
	 *
	 * @param array{type: string, id: string, label: string} $subject Subject.
	 * @return array<string, string>
	 */
	function dailyos_suggested_next_steps_subject_ref( array $subject ): array {
		return [ $subject['type'] => $subject['id'] ];
	}
}

if ( ! function_exists( 'dailyos_suggested_next_steps_surface_context' ) ) {
	/**
	 * Map entity type to claim receipt surface context.
	 *
	 * @param string $entity_type Entity type.
	 * @return string
	 */
	function dailyos_suggested_next_steps_surface_context( string $entity_type ): string {
		return 'meeting' === $entity_type ? 'meeting_detail' : 'entity_detail';
	}
}

if ( ! function_exists( 'dailyos_suggested_next_steps_max_items' ) ) {
	/**
	 * Resolve maxItems with the W3-A default and server ceiling.
	 *
	 * @param array<string, mixed> $attributes Block attributes.
	 * @return int
	 */
	function dailyos_suggested_next_steps_max_items( array $attributes ): int {
		$max_items = isset( $attributes['maxItems'] ) && is_numeric( $attributes['maxItems'] )
			? (int) $attributes['maxItems']
			: 5;

		if ( $max_items < 1 ) {
			return 1;
		}

		return min( 8, $max_items );
	}
}

if ( ! function_exists( 'dailyos_suggested_next_steps_extract_items' ) ) {
	/**
	 * Extract the items list from supported runtime envelope shapes.
	 *
	 * @param array<string, mixed> $response Raw runtime client response.
	 * @return array<int, mixed>|null
	 */
	function dailyos_suggested_next_steps_extract_items( array $response ): ?array {
		if (
			isset( $response['ability'] )
			&& is_array( $response['ability'] )
			&& isset( $response['ability']['data'] )
			&& is_array( $response['ability']['data'] )
			&& isset( $response['ability']['data']['items'] )
			&& is_array( $response['ability']['data']['items'] )
		) {
			return array_values( $response['ability']['data']['items'] );
		}

		if (
			isset( $response['data'] )
			&& is_array( $response['data'] )
			&& isset( $response['data']['items'] )
			&& is_array( $response['data']['items'] )
		) {
			return array_values( $response['data']['items'] );
		}

		if ( isset( $response['items'] ) && is_array( $response['items'] ) ) {
			return array_values( $response['items'] );
		}

		return null;
	}
}

if ( ! function_exists( 'dailyos_suggested_next_steps_heading' ) ) {
	/**
	 * Resolve heading text from override or surface defaults.
	 *
	 * @param array{type: string, id: string, label: string} $subject    Subject.
	 * @param array<string, mixed>                          $attributes Block attributes.
	 * @param array<string, mixed>|null                     $response   Runtime response.
	 * @param array<int, mixed>                             $items      Items.
	 * @return string
	 */
	function dailyos_suggested_next_steps_heading( array $subject, array $attributes, ?array $response, array $items ): string {
		if ( isset( $attributes['headingLabel'] ) && is_string( $attributes['headingLabel'] ) && '' !== trim( $attributes['headingLabel'] ) ) {
			return trim( $attributes['headingLabel'] );
		}

		if ( 'meeting' === $subject['type'] ) {
			return __( 'What to cover', 'dailyos' );
		}

		$label = dailyos_suggested_next_steps_subject_label( $subject, $response, $items );
		switch ( $subject['type'] ) {
			case 'project':
				return sprintf(
					/* translators: %s: project label */
					__( "What's next on %s", 'dailyos' ),
					$label
				);
			case 'person':
				return sprintf(
					/* translators: %s: person label */
					__( 'Open threads with %s', 'dailyos' ),
					$label
				);
			case 'account':
			default:
				return sprintf(
					/* translators: %s: account label */
					__( "What's next with %s", 'dailyos' ),
					$label
				);
		}
	}
}

if ( ! function_exists( 'dailyos_suggested_next_steps_subject_label' ) ) {
	/**
	 * Resolve a redacted display label when present in context or response.
	 *
	 * @param array{type: string, id: string, label: string} $subject  Subject.
	 * @param array<string, mixed>|null                     $response Runtime response.
	 * @param array<int, mixed>                             $items    Items.
	 * @return string
	 */
	function dailyos_suggested_next_steps_subject_label( array $subject, ?array $response, array $items ): string {
		if ( '' !== $subject['label'] ) {
			return dailyos_suggested_next_steps_truncate_label( $subject['label'] );
		}

		$roots = [];
		if ( is_array( $response ) ) {
			$roots[] = $response;
			if ( isset( $response['data'] ) && is_array( $response['data'] ) ) {
				$roots[] = $response['data'];
			}
			if ( isset( $response['ability'] ) && is_array( $response['ability'] ) && isset( $response['ability']['data'] ) && is_array( $response['ability']['data'] ) ) {
				$roots[] = $response['ability']['data'];
			}
		}

		foreach ( $roots as $root ) {
			foreach ( [ 'subjectLabel', 'entityLabel', 'displayLabel' ] as $key ) {
				if ( isset( $root[ $key ] ) && is_string( $root[ $key ] ) && '' !== trim( $root[ $key ] ) ) {
					return dailyos_suggested_next_steps_truncate_label( trim( $root[ $key ] ) );
				}
			}
			if ( isset( $root['subject'] ) && is_array( $root['subject'] ) ) {
				foreach ( [ 'label', 'displayLabel', 'name' ] as $key ) {
					if ( isset( $root['subject'][ $key ] ) && is_string( $root['subject'][ $key ] ) && '' !== trim( $root['subject'][ $key ] ) ) {
						return dailyos_suggested_next_steps_truncate_label( trim( $root['subject'][ $key ] ) );
					}
				}
			}
		}

		foreach ( $items as $item ) {
			if ( ! is_array( $item ) ) {
				continue;
			}
			if ( isset( $item['subjectLabel'] ) && is_string( $item['subjectLabel'] ) && '' !== trim( $item['subjectLabel'] ) ) {
				return dailyos_suggested_next_steps_truncate_label( trim( $item['subjectLabel'] ) );
			}
		}

		$fallbacks = [
			'account' => __( 'this account', 'dailyos' ),
			'project' => __( 'this project', 'dailyos' ),
			'person'  => __( 'this person', 'dailyos' ),
		];

		return $fallbacks[ $subject['type'] ] ?? $subject['id'];
	}
}

if ( ! function_exists( 'dailyos_suggested_next_steps_truncate_label' ) ) {
	/**
	 * Truncate a subject label at 40 characters, preferring word boundary.
	 *
	 * @param string $label Subject label.
	 * @return string
	 */
	function dailyos_suggested_next_steps_truncate_label( string $label ): string {
		$label = trim( $label );
		if ( function_exists( 'mb_strlen' ) && function_exists( 'mb_substr' ) ) {
			if ( mb_strlen( $label ) <= 40 ) {
				return $label;
			}
			$prefix = mb_substr( $label, 0, 40 );
		} else {
			if ( strlen( $label ) <= 40 ) {
				return $label;
			}
			$prefix = substr( $label, 0, 40 );
		}

		$space = strrpos( $prefix, ' ' );
		if ( false !== $space && $space >= 24 ) {
			$prefix = substr( $prefix, 0, $space );
		}

		return rtrim( $prefix ) . '...';
	}
}

if ( ! function_exists( 'dailyos_suggested_next_steps_render_empty_shell' ) ) {
	/**
	 * Render a visible empty-state chip.
	 *
	 * @param array{type: string, id: string, label: string}|null $subject Subject.
	 * @param string                                             $heading Heading.
	 * @param string                                             $reason  Empty reason.
	 * @param string                                             $copy    Empty copy.
	 * @return string
	 */
	function dailyos_suggested_next_steps_render_empty_shell( ?array $subject, string $heading, string $reason, string $copy ): string {
		$body = '<span class="dailyos-empty-chip" data-empty-reason="' . esc_attr( $reason ) . '">' . esc_html( $copy ) . '</span>';
		return dailyos_suggested_next_steps_render_shell( $subject, $heading, $body, 0, true );
	}
}

if ( ! function_exists( 'dailyos_suggested_next_steps_render_shell' ) ) {
	/**
	 * Render the reference shell for the current surface.
	 *
	 * @param array{type: string, id: string, label: string}|null $subject  Subject.
	 * @param string                                             $heading  Heading.
	 * @param string                                             $body     Inner HTML.
	 * @param int                                                $count    Item count.
	 * @param bool                                               $is_empty Empty state.
	 * @return string
	 */
	function dailyos_suggested_next_steps_render_shell( ?array $subject, string $heading, string $body, int $count, bool $is_empty ): string {
		$type          = is_array( $subject ) ? $subject['type'] : 'unknown';
		$empty_class   = $is_empty ? ' SuggestedNextSteps_section--empty is-empty' : '';
		$surface_attr  = ' data-surface="' . esc_attr( $type ) . '"';
		$section_attrs = ' class="SuggestedNextSteps_section' . esc_attr( $empty_class ) . '"' . $surface_attr . ' data-dailyos-block="suggested-next-steps"';
		$chapter       = dailyos_suggested_next_steps_render_chapter_heading( $heading, $type, $count, $is_empty );

		if ( 'account' === $type ) {
			$out  = '<div id="whats-next" class="editorial-reveal entity-detail_marginLabelSection' . esc_attr( $empty_class ) . '">';
			$out .= '<div class="entity-detail_marginLabel">What\'s<br/>Next</div>';
			$out .= '<div class="entity-detail_marginContent">';
			$out .= '<section' . $section_attrs . '>';
			$out .= $chapter . $body;
			$out .= '</section></div></div>';
			return $out;
		}

		if ( 'project' === $type ) {
			$out  = '<div id="whats-next" class="editorial-reveal entity-detail_chapterSection' . esc_attr( $empty_class ) . '">';
			$out .= '<section' . $section_attrs . '>';
			$out .= $chapter . $body;
			$out .= '</section></div>';
			return $out;
		}

		if ( 'person' === $type ) {
			$out  = '<div id="open-threads" class="editorial-reveal entity-detail_chapterSectionWithPadding' . esc_attr( $empty_class ) . '">';
			$out .= '<section' . $section_attrs . '>';
			$out .= $chapter . $body;
			$out .= '</section></div>';
			return $out;
		}

		if ( 'meeting' === $type ) {
			$out  = '<section id="what-to-cover" class="editorial-reveal meeting-intel_chapterSection SuggestedNextSteps_section' . esc_attr( $empty_class ) . '"' . $surface_attr . ' data-dailyos-block="suggested-next-steps">';
			$out .= $chapter . $body;
			$out .= '</section>';
			return $out;
		}

		$out  = '<section class="editorial-reveal entity-detail_chapterSection SuggestedNextSteps_section' . esc_attr( $empty_class ) . '"' . $surface_attr . ' data-dailyos-block="suggested-next-steps">';
		$out .= $chapter . $body;
		$out .= '</section>';
		return $out;
	}
}

if ( ! function_exists( 'dailyos_suggested_next_steps_render_chapter_heading' ) ) {
	/**
	 * Render the shared ChapterHeading markup.
	 *
	 * @param string $heading  Heading text.
	 * @param string $type     Surface type.
	 * @param int    $count    Item count.
	 * @param bool   $is_empty Whether the body is an empty state.
	 * @return string
	 */
	function dailyos_suggested_next_steps_render_chapter_heading( string $heading, string $type, int $count, bool $is_empty ): string {
		if ( '' === $heading ) {
			return '';
		}

		$out  = '<div class="ChapterHeading_heading">';
		$out .= '<hr class="ChapterHeading_rule">';
		$out .= '<div class="ChapterHeading_titleRow">';
		$out .= '<h2 class="ChapterHeading_title">' . esc_html( $heading ) . '</h2>';
		$out .= '</div>';

		if ( ! $is_empty ) {
			$epigraph = dailyos_suggested_next_steps_epigraph( $type, $count );
			if ( '' !== $epigraph ) {
				$out .= '<p class="ChapterHeading_epigraph">' . esc_html( $epigraph ) . '</p>';
			}
		}

		$out .= '</div>';
		return $out;
	}
}

if ( ! function_exists( 'dailyos_suggested_next_steps_epigraph' ) ) {
	/**
	 * Render surface-specific epigraph copy matching the reference surfaces.
	 *
	 * @param string $type  Surface type.
	 * @param int    $count Item count.
	 * @return string
	 */
	function dailyos_suggested_next_steps_epigraph( string $type, int $count ): string {
		$count = max( 0, $count );
		switch ( $type ) {
			case 'account':
				return sprintf(
					/* translators: %d: recommendation count */
					_n( '%d recommendation to consider this week.', '%d recommendations to consider this week.', $count, 'dailyos' ),
					$count
				);
			case 'project':
				return sprintf(
					/* translators: %d: move count */
					_n( '%d move to keep the project on its arc.', '%d moves to keep the project on its arc.', $count, 'dailyos' ),
					$count
				);
			case 'person':
				return sprintf(
					/* translators: %d: thread count */
					_n( '%d thread to carry into the next 1:1.', '%d threads to carry into the next 1:1.', $count, 'dailyos' ),
					$count
				);
			case 'meeting':
				return sprintf(
					/* translators: %d: prep topic count */
					_n( '%d prep topic worth landing.', '%d prep topics worth landing.', $count, 'dailyos' ),
					$count
				);
			default:
				return '';
		}
	}
}

if ( ! function_exists( 'dailyos_suggested_next_steps_render_row' ) ) {
	/**
	 * Render one recommendation row.
	 *
	 * @param array<string, mixed> $item             Runtime item.
	 * @param bool                 $feedback_enabled Whether feedback is enabled.
	 * @param int                  $index            Row index.
	 * @return string
	 */
	function dailyos_suggested_next_steps_render_row( array $item, bool $feedback_enabled, int $index ): string {
		$claim_id = isset( $item['claimId'] ) ? (string) $item['claimId'] : '';
		$headline = isset( $item['headline'] ) ? trim( (string) $item['headline'] ) : '';
		if ( '' === $claim_id || '' === $headline ) {
			return '';
		}

		$trust_band     = dailyos_suggested_next_steps_trust_band( $item );
		$terminal_state = dailyos_suggested_next_steps_terminal_state( $item );
		$row_classes    = [ 'suggested-next-steps_row' ];
		if ( 'needs_verification' === $trust_band ) {
			$row_classes[] = 'suggested-next-steps_row--needsVerification';
		}
		if ( 'in_flight' === $terminal_state ) {
			$row_classes[] = 'suggested-next-steps_row--inFlight';
		}
		if ( 'decided_dismissed' === $terminal_state ) {
			$row_classes[] = 'suggested-next-steps_row--collapsing';
		}

		$busy_attr = 'in_flight' === $terminal_state ? ' aria-busy="true"' : '';
		$more_id   = dailyos_suggested_next_steps_more_id( $claim_id, $index );
		$disabled  = ! $feedback_enabled || 'in_flight' === $terminal_state;

		$out  = '<article class="' . esc_attr( implode( ' ', $row_classes ) ) . '" data-claim-id="' . esc_attr( $claim_id ) . '" data-feedback-state="' . esc_attr( $terminal_state ) . '"' . $busy_attr . '>';
		$out .= '<header class="suggested-next-steps_rowHeader">';
		$out .= '<h3 class="suggested-next-steps_headline">' . esc_html( $headline ) . dailyos_suggested_next_steps_render_trust_band_indicator( $trust_band ) . '</h3>';
		$out .= '</header>';

		$why = isset( $item['whyThisNowSurfaceText'] ) ? trim( (string) $item['whyThisNowSurfaceText'] ) : '';
		if ( '' !== $why ) {
			$out .= '<p class="suggested-next-steps_whyThisNow">' . esc_html( $why ) . '</p>';
		}

		$action = isset( $item['recommendedAction'] ) && is_array( $item['recommendedAction'] )
			? dailyos_suggested_next_steps_render_recommended_action( $item['recommendedAction'] )
			: '';
		if ( '' !== $action ) {
			$out .= '<p class="suggested-next-steps_action">' . $action . '</p>';
		}

		if ( 'decided_converted' === $terminal_state ) {
			$out .= dailyos_suggested_next_steps_render_converted_chip( $item );
			$out .= '</article>';
			return $out;
		}

		if ( 'error' === $terminal_state ) {
			$out .= '<span class="dailyos-info-chip" data-feedback-state="error">' . esc_html__( 'Feedback did not save. Try again.', 'dailyos' ) . '</span>';
		}

		$out .= '<div class="suggested-next-steps_affordances" role="group" aria-label="' . esc_attr( __( 'Feedback on this recommendation', 'dailyos' ) ) . '">';
		$out .= dailyos_suggested_next_steps_render_button( 'convert', __( 'Convert to action', 'dailyos' ), __( 'Convert this recommendation to an action', 'dailyos' ), $disabled, '' );
		$out .= dailyos_suggested_next_steps_render_button( 'dismiss', __( 'Dismiss', 'dailyos' ), __( 'Dismiss this recommendation', 'dailyos' ), $disabled, '' );
		$out .= dailyos_suggested_next_steps_render_more_button( $more_id, $disabled );
		$out .= '</div>';

		$out .= '<div id="' . esc_attr( $more_id ) . '" class="suggested-next-steps_subAffordances" aria-hidden="true">';
		$out .= dailyos_suggested_next_steps_render_button( 'notUseful', __( 'Mark as not useful', 'dailyos' ), __( 'Mark this recommendation as not useful', 'dailyos' ), $disabled, 'suggested-next-steps_subButton' );
		$out .= dailyos_suggested_next_steps_render_button( 'tooNoisy', __( 'Mark as too noisy', 'dailyos' ), __( 'Mark this recommendation as too noisy', 'dailyos' ), $disabled, 'suggested-next-steps_subButton' );
		$out .= dailyos_suggested_next_steps_render_button( 'dismissWithReason', __( 'Dismiss with reason', 'dailyos' ), __( 'Dismiss this recommendation with a reason', 'dailyos' ), $disabled, 'suggested-next-steps_subButton' );
		$out .= '</div>';
		$out .= '</article>';

		return $out;
	}
}

if ( ! function_exists( 'dailyos_suggested_next_steps_trust_band' ) ) {
	/**
	 * Resolve trust band from item receipt or top-level field.
	 *
	 * @param array<string, mixed> $item Runtime item.
	 * @return string
	 */
	function dailyos_suggested_next_steps_trust_band( array $item ): string {
		$band = '';
		if (
			isset( $item['receipt'] )
			&& is_array( $item['receipt'] )
			&& isset( $item['receipt']['trust'] )
			&& is_array( $item['receipt']['trust'] )
			&& isset( $item['receipt']['trust']['band'] )
		) {
			$band = (string) $item['receipt']['trust']['band'];
		} elseif ( isset( $item['trustBand'] ) ) {
			$band = (string) $item['trustBand'];
		}

		if ( ! in_array( $band, [ 'likely_current', 'use_with_caution', 'needs_verification', 'unscored' ], true ) ) {
			return 'use_with_caution';
		}

		return $band;
	}
}

if ( ! function_exists( 'dailyos_suggested_next_steps_terminal_state' ) ) {
	/**
	 * Normalize item lifecycle to the row data-feedback-state value.
	 *
	 * @param array<string, mixed> $item Runtime item.
	 * @return string
	 */
	function dailyos_suggested_next_steps_terminal_state( array $item ): string {
		$feedback = $item['feedbackState'] ?? 'pending';
		if ( is_string( $feedback ) && in_array( $feedback, [ 'pending', 'in_flight', 'decided_dismissed', 'decided_converted', 'error' ], true ) ) {
			return $feedback;
		}

		$conversion = isset( $item['conversionState'] ) && is_array( $item['conversionState'] ) ? $item['conversionState'] : [];
		if ( isset( $conversion['kind'] ) && is_string( $conversion['kind'] ) && 'convertedToAction' === $conversion['kind'] ) {
			return 'decided_converted';
		}

		if ( is_array( $feedback ) && isset( $feedback['decided'] ) && is_array( $feedback['decided'] ) ) {
			$kind = isset( $feedback['decided']['kind'] ) ? (string) $feedback['decided']['kind'] : '';
			if ( 'convert' === $kind ) {
				return 'decided_converted';
			}
			if ( in_array( $kind, [ 'dismiss', 'notUseful', 'tooNoisy' ], true ) ) {
				return 'decided_dismissed';
			}
		}

		return 'pending';
	}
}

if ( ! function_exists( 'dailyos_suggested_next_steps_render_trust_band_indicator' ) ) {
	/**
	 * Render the TrustBandIndicator finis marker.
	 *
	 * @param string $band Trust band.
	 * @return string
	 */
	function dailyos_suggested_next_steps_render_trust_band_indicator( string $band ): string {
		$labels = [
			'likely_current'     => __( 'Likely current', 'dailyos' ),
			'use_with_caution'   => __( 'Use with caution', 'dailyos' ),
			'needs_verification' => __( 'Needs verification', 'dailyos' ),
			'unscored'           => __( 'Unscored', 'dailyos' ),
		];
		$classes = [
			'likely_current'     => 'TrustBandIndicator_likelyCurrent',
			'use_with_caution'   => 'TrustBandIndicator_useWithCaution',
			'needs_verification' => 'TrustBandIndicator_needsVerification',
			'unscored'           => 'TrustBandIndicator_unscored',
		];
		$label   = $labels[ $band ] ?? $labels['use_with_caution'];
		$class   = $classes[ $band ] ?? $classes['use_with_caution'];

		return '<span class="TrustBandIndicator TrustBandIndicator_indicator ' . esc_attr( $class ) . '" title="' . esc_attr( $label ) . '" aria-label="' . esc_attr( $label ) . '">&#9679;</span>';
	}
}

if ( ! function_exists( 'dailyos_suggested_next_steps_render_recommended_action' ) ) {
	/**
	 * Render the redacted RecommendedActionView label.
	 *
	 * @param array<string, mixed> $action Recommended action DTO.
	 * @return string Escaped HTML.
	 */
	function dailyos_suggested_next_steps_render_recommended_action( array $action ): string {
		$kind  = isset( $action['kind'] ) ? (string) $action['kind'] : '';
		$parts = [];

		switch ( $kind ) {
			case 'scheduleMeeting':
				$parts[] = __( 'Schedule meeting', 'dailyos' );
				foreach ( [ 'entityLabel', 'whenWindow' ] as $key ) {
					if ( isset( $action[ $key ] ) && '' !== trim( (string) $action[ $key ] ) ) {
						$parts[] = trim( (string) $action[ $key ] );
					}
				}
				break;
			case 'sendMessage':
				$parts[] = __( 'Send message', 'dailyos' );
				foreach ( [ 'channel', 'entityLabel' ] as $key ) {
					if ( isset( $action[ $key ] ) && '' !== trim( (string) $action[ $key ] ) ) {
						$parts[] = trim( (string) $action[ $key ] );
					}
				}
				break;
			case 'reviewClaim':
				$parts[] = __( 'Review claim', 'dailyos' );
				if ( isset( $action['claimLabel'] ) && '' !== trim( (string) $action['claimLabel'] ) ) {
					$parts[] = trim( (string) $action['claimLabel'] );
				}
				break;
			case 'updateRecord':
				$parts[] = __( 'Update record', 'dailyos' );
				foreach ( [ 'entityLabel', 'fieldLabel' ] as $key ) {
					if ( isset( $action[ $key ] ) && '' !== trim( (string) $action[ $key ] ) ) {
						$parts[] = trim( (string) $action[ $key ] );
					}
				}
				break;
			case 'investigateChange':
				$parts[] = __( 'Investigate change', 'dailyos' );
				foreach ( [ 'entityLabel', 'changeLabel' ] as $key ) {
					if ( isset( $action[ $key ] ) && '' !== trim( (string) $action[ $key ] ) ) {
						$parts[] = trim( (string) $action[ $key ] );
					}
				}
				break;
			case 'custom':
				if ( isset( $action['actionLabel'] ) && '' !== trim( (string) $action['actionLabel'] ) ) {
					$parts[] = trim( (string) $action['actionLabel'] );
				}
				break;
		}

		$parts = array_values(
			array_filter(
				$parts,
				static function ( $part ): bool {
					return is_string( $part ) && '' !== trim( $part );
				}
			)
		);

		if ( [] === $parts ) {
			return '';
		}

		return implode(
			' &middot; ',
			array_map(
				static function ( string $part ): string {
					return esc_html( $part );
				},
				$parts
			)
		);
	}
}

if ( ! function_exists( 'dailyos_suggested_next_steps_render_button' ) ) {
	/**
	 * Render a feedback button.
	 *
	 * @param string $kind       Feedback kind.
	 * @param string $label      Visible label.
	 * @param string $aria_label ARIA label.
	 * @param bool   $disabled   Disabled affordance state.
	 * @param string $extra      Extra class.
	 * @return string
	 */
	function dailyos_suggested_next_steps_render_button( string $kind, string $label, string $aria_label, bool $disabled, string $extra ): string {
		$classes = 'suggested-next-steps_button';
		if ( '' !== $extra ) {
			$classes .= ' ' . $extra;
		}
		if ( $disabled ) {
			$classes .= ' disabled-affordance';
		}

		$disabled_attrs = $disabled ? ' tabindex="-1" aria-disabled="true"' : '';

		$visible_label = esc_html( $label );
		if ( 'dismissWithReason' === $kind ) {
			$visible_label .= '&hellip;';
		}

		return '<button type="button" class="' . esc_attr( $classes ) . '" data-feedback-kind="' . esc_attr( $kind ) . '" aria-label="' . esc_attr( $aria_label ) . '"' . $disabled_attrs . '>' . $visible_label . '</button>';
	}
}

if ( ! function_exists( 'dailyos_suggested_next_steps_render_more_button' ) ) {
	/**
	 * Render the More feedback disclosure button.
	 *
	 * @param string $more_id  Controlled row ID.
	 * @param bool   $disabled Disabled affordance state.
	 * @return string
	 */
	function dailyos_suggested_next_steps_render_more_button( string $more_id, bool $disabled ): string {
		$classes = 'suggested-next-steps_button suggested-next-steps_buttonExpand';
		if ( $disabled ) {
			$classes .= ' disabled-affordance';
		}
		$disabled_attrs = $disabled ? ' tabindex="-1" aria-disabled="true"' : '';

		return '<button type="button" class="' . esc_attr( $classes ) . '" aria-expanded="false" aria-controls="' . esc_attr( $more_id ) . '" aria-label="' . esc_attr( __( 'More feedback options', 'dailyos' ) ) . '"' . $disabled_attrs . '>' . esc_html__( 'More feedback', 'dailyos' ) . '&hellip;</button>';
	}
}

if ( ! function_exists( 'dailyos_suggested_next_steps_render_converted_chip' ) ) {
	/**
	 * Render converted terminal state.
	 *
	 * @param array<string, mixed> $item Runtime item.
	 * @return string
	 */
	function dailyos_suggested_next_steps_render_converted_chip( array $item ): string {
		$action_id = '';
		if (
			isset( $item['conversionState'] )
			&& is_array( $item['conversionState'] )
			&& isset( $item['conversionState']['actionId'] )
		) {
			$action_id = (string) $item['conversionState']['actionId'];
		}

		$label = __( 'Converted to action', 'dailyos' );
		if ( '' !== $action_id ) {
			$label .= ' - ' . $action_id;
		}

		return '<span class="dailyos-info-chip" data-feedback-state="decided_converted">' . esc_html( $label ) . '</span>';
	}
}

if ( ! function_exists( 'dailyos_suggested_next_steps_more_id' ) ) {
	/**
	 * Build a stable aria-controls target ID.
	 *
	 * @param string $claim_id Claim ID.
	 * @param int    $index    Row index.
	 * @return string
	 */
	function dailyos_suggested_next_steps_more_id( string $claim_id, int $index ): string {
		$base = preg_replace( '/[^A-Za-z0-9_-]+/', '-', $claim_id );
		$base = is_string( $base ) && '' !== trim( $base ) ? trim( $base, '-' ) : 'rec-claim-' . ( $index + 1 );
		return $base . '-more';
	}
}

if ( ! function_exists( 'dailyos_suggested_next_steps_empty_copy' ) ) {
	/**
	 * Surface-specific empty-state copy.
	 *
	 * @param string $entity_type Entity type.
	 * @return string
	 */
	function dailyos_suggested_next_steps_empty_copy( string $entity_type ): string {
		switch ( $entity_type ) {
			case 'person':
				return __( 'No open threads.', 'dailyos' );
			case 'meeting':
				return __( 'Nothing to cover yet.', 'dailyos' );
			case 'account':
			case 'project':
			default:
				return __( 'Nothing flagged right now.', 'dailyos' );
		}
	}
}

if ( ! function_exists( 'dailyos_suggested_next_steps_unavailable_copy' ) ) {
	/**
	 * Runtime unavailable copy.
	 *
	 * @return string
	 */
	function dailyos_suggested_next_steps_unavailable_copy(): string {
		return __( 'Recommendations unavailable.', 'dailyos' );
	}
}
