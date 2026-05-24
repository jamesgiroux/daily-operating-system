<?php
/**
 * Account Watch List (account-detail/inner/watch-list) — W2 L1 inner block.
 *
 * Distinct from the person-detail-bound `dailyos/watch-list` at
 * wp/dailyos/blocks/watch-list/ (which renders an OpenLoops subset for
 * the person surface). This block registers as
 * `dailyos/account-detail-watch-list` and emits the canonical Watch List
 * chapter DOM from .docs/design/reference/surfaces/account.html lines
 * 519-580 (section id `watch-list`). Verify with:
 *   python3 wp/dailyos/dev-tools/parity-check.py watch-list
 *
 * @package DailyOS
 */

declare(strict_types=1);

if ( ! defined( 'ABSPATH' ) ) {
	return '';
}

if ( ! function_exists( 'dailyos_resolve_envelope' ) ) {
	require_once dirname( __DIR__, 3 ) . '/_shared/envelope/envelope-resolver.php';
}

if ( ! function_exists( 'dailyos_account_detail_watch_list_render' ) ) {
	/**
	 * Render the account detail watch list.
	 *
	 * @param array<string, mixed> $attributes Block attributes.
	 * @param string               $content    Inner content.
	 * @param \WP_Block|null       $block      Parsed block.
	 * @return string
	 */
	function dailyos_account_detail_watch_list_render( array $attributes, string $content = '', $block = null ): string {
		unset( $attributes, $content, $block );

		$wins     = [
			'Support workflow rehearsal completed with the admin team and produced a forwardable proof point.',
		];
		$unknowns = [
			'Finance timing is still unclear once the legal owner is named.',
		];
		$programs = [
			[
				'name'   => 'Support workflow expansion',
				'status' => 'active',
				'label'  => 'Active',
				'notes'  => 'Pilot is live with the admin group; next checkpoint is support leadership adoption.',
			],
			[
				'name'   => 'Executive renewal readout',
				'status' => 'planned',
				'label'  => 'Planned',
				'notes'  => 'Waiting on the legal owner before Jen forwards the packet to finance.',
			],
		];

		$out  = '<div id="watch-list" class="editorial-reveal entity-detail_marginLabelSection">';
		$out .= '<div class="entity-detail_marginLabel">' . wp_kses_post( __( 'Watch<br/>List', 'dailyos' ) ) . '</div>';
		$out .= '<div class="entity-detail_marginContent">';
		$out .= '<section class="WatchList_section">';

		$out .= '<div class="ChapterHeading_heading">';
		$out .= '<hr class="ChapterHeading_rule" />';
		$out .= '<div class="ChapterHeading_titleRow">';
		$out .= '<h2 class="ChapterHeading_title">' . esc_html__( 'Watch List', 'dailyos' ) . '</h2>';
		$out .= '</div>';
		$out .= '</div>';

		$out .= dailyos_account_detail_watch_list_section_card( 'win', __( 'Wins', 'dailyos' ), $wins );
		$out .= dailyos_account_detail_watch_list_section_card( 'unknown', __( 'Unknowns', 'dailyos' ), $unknowns );

		$out .= '<div class="WatchListPrograms_section">';
		$out .= '<div class="WatchListPrograms_heading">' . esc_html__( 'Active Initiatives', 'dailyos' ) . '</div>';
		$out .= '<div class="WatchListPrograms_programList">';
		foreach ( $programs as $program ) {
			$out .= '<div class="WatchListPrograms_program">';
			$out .= '<div class="WatchListPrograms_programHeader">';
			$out .= '<span class="WatchListPrograms_programName">' . esc_html( $program['name'] ) . '</span>';
			$out .= '<span class="WatchListPrograms_statusBadge" data-status="' . esc_attr( $program['status'] ) . '">' . esc_html( $program['label'] ) . '</span>';
			$out .= '<button type="button" class="WatchListPrograms_deleteButton">x</button>';
			$out .= '</div>';
			$out .= '<p class="WatchListPrograms_notes">' . esc_html( $program['notes'] ) . '</p>';
			$out .= '</div>';
		}
		$out .= '</div>';
		$out .= '<button type="button" class="WatchListPrograms_addButton">' . esc_html__( '+ Add Initiative', 'dailyos' ) . '</button>';
		$out .= '</div>';

		$out .= '</section>';
		$out .= '</div>'; // .entity-detail_marginContent
		$out .= '</div>'; // .entity-detail_marginLabelSection
		return $out;
	}
}

if ( ! function_exists( 'dailyos_account_detail_watch_list_section_card' ) ) {
	/**
	 * Render the account detail watch list section card.
	 *
	 * @param string            $type  Section type.
	 * @param string            $label Section label.
	 * @param array<int,string> $items Section items.
	 * @return string
	 */
	function dailyos_account_detail_watch_list_section_card( string $type, string $label, array $items ): string {
		$out  = '<div class="WatchList_sectionCard" data-type="' . esc_attr( $type ) . '">';
		$out .= '<div class="WatchList_sectionLabel">' . esc_html( $label ) . '</div>';
		foreach ( $items as $text ) {
			$out .= '<div class="WatchList_itemRow">';
			$out .= '<div class="WatchList_itemText">';
			$out .= '<p class="EditableText_editable WatchList_itemTextContent" title="' . esc_attr__( 'Click to edit', 'dailyos' ) . '" data-editable-text>' . esc_html( $text ) . '</p>';
			$out .= '</div>';
			$out .= '<div class="WatchList_itemActions">';
			$out .= '<span class="IntelligenceFeedback_wrapper"><span class="IntelligenceFeedback_container">';
			$out .= '<button type="button" class="IntelligenceFeedback_button" aria-label="' . esc_attr__( 'This was helpful', 'dailyos' ) . '" aria-pressed="false" title="' . esc_attr__( 'Helpful', 'dailyos' ) . '"></button>';
			$out .= '<button type="button" class="IntelligenceFeedback_button" aria-label="' . esc_attr__( 'This was not helpful', 'dailyos' ) . '" aria-pressed="false" title="' . esc_attr__( 'Not helpful', 'dailyos' ) . '"></button>';
			$out .= '</span></span>';
			$out .= '<button type="button" title="' . esc_attr__( 'Remove', 'dailyos' ) . '" class="WatchList_dismissButton"></button>';
			$out .= '</div>';
			$out .= '</div>';
		}
		$out .= '</div>';
		return $out;
	}
}
