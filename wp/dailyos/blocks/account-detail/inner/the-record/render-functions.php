<?php
/**
 * The Record (the-record) — W2 L1 inner block render-functions.
 *
 * DOM verbatim copy of .docs/design/reference/surfaces/account.html lines
 * 693-748 (section id `the-record`). Verify with:
 *   python3 wp/dailyos/dev-tools/parity-check.py the-record
 *
 * Structure: marginLabelSection chrome → unclassed <section> with
 * ChapterHeading + AddToRecord_addButton + TimelineEntry_timeline of
 * TimelineEntry rows (meeting / email / note / value dot variants).
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

if ( ! function_exists( 'dailyos_account_detail_the_record_render' ) ) {
	/**
	 * Render the account detail record.
	 *
	 * @param array<string, mixed> $attributes Block attributes.
	 * @param string               $content    Inner content.
	 * @param \WP_Block|null       $block      Parsed block.
	 * @return string
	 */
	function dailyos_account_detail_the_record_render( array $attributes, string $content = '', $block = null ): string {
		unset( $attributes, $content, $block );

		$entries = array_merge(
			[
				[
					'kind'   => 'email',
					'date'   => 'May',
					'title'  => __( 'Customer follow-up received', 'dailyos' ),
					'detail' => __( 'Email', 'dailyos' ),
					'link'   => false,
				],
				[
					'kind'   => 'email',
					'date'   => 'May',
					'title'  => __( 'Internal update captured', 'dailyos' ),
					'detail' => __( 'Email', 'dailyos' ),
					'link'   => false,
				],
			],
			array_fill(
				0,
				8,
				[
					'kind'   => 'meeting',
					'date'   => 'Apr',
					'title'  => __( 'Account meeting captured', 'dailyos' ),
					'detail' => __( 'Meeting', 'dailyos' ),
					'link'   => true,
				]
			)
		);

		$out  = '<div id="the-record" class="entity-detail_marginLabelSection" data-ds-name="MarginGrid" data-ds-tier="pattern" data-ds-spec="patterns/MarginGrid.md">';
		$out .= '<div class="entity-detail_marginLabel">' . wp_kses_post( __( 'The<br/>Record', 'dailyos' ) ) . '</div>';
		$out .= '<div class="entity-detail_marginContent">';
		$out .= '<section>';

		$out .= '<div class="ChapterHeading_heading" data-ds-name="ChapterHeading" data-ds-tier="pattern" data-ds-spec="patterns/ChapterHeading.md">';
		$out .= '<hr class="ChapterHeading_rule" />';
		$out .= '<div class="ChapterHeading_titleRow">';
		$out .= '<h2 class="ChapterHeading_title">' . esc_html__( 'The Record', 'dailyos' ) . '</h2>';
		$out .= '</div>';
		$out .= '</div>';

		$out .= '<button type="button" class="AddToRecord_addButton">' . esc_html__( '+ Add Note', 'dailyos' ) . '</button>';
		$out .= '<div class="TimelineEntry_timeline">';
		foreach ( $entries as $entry ) {
			$out .= dailyos_account_detail_the_record_entry( $entry );
		}
		$out .= '</div>';
		$out .= '<button type="button" class="TimelineEntry_toggleButton">';
		$out .= '<svg class="TimelineEntry_chevron" viewBox="0 0 24 24" aria-hidden="true"><polyline points="6 9 12 15 18 9"></polyline></svg>';
		$out .= '</button>';

		$out .= '</section>';
		$out .= '</div>'; // .entity-detail_marginContent
		$out .= '</div>'; // .entity-detail_marginLabelSection
		return $out;
	}
}

if ( ! function_exists( 'dailyos_account_detail_the_record_resolve_entries' ) ) {
	/**
	 * Resolve the account detail record entries.
	 *
	 * @param array<string,mixed>|null $envelope Envelope payload.
	 * @return array<int,array<string,mixed>>
	 */
	function dailyos_account_detail_the_record_resolve_entries( ?array $envelope ): array {
		$default = [
			[
				'kind'   => 'meeting',
				'date'   => 'May 4',
				'title'  => 'Account renewal checkpoint',
				'detail' => 'Customer',
				'link'   => true,
			],
			[
				'kind'   => 'email',
				'date'   => 'May 2',
				'title'  => 'A stakeholder asked for a single owner before forwarding the package.',
				'detail' => 'customer@example.com',
				'link'   => false,
			],
			[
				'kind'   => 'context',
				'date'   => 'Apr 29',
				'title'  => 'Renewal packet framing',
				'detail' => 'Anchor the next update around owner, date, and decision requested. · Added by you',
				'link'   => false,
			],
			[
				'kind'   => 'value',
				'date'   => 'Apr 25',
				'title'  => 'Milestone completed: Admin workflow rehearsal',
				'detail' => 'Auto-completed by meeting evidence',
				'link'   => false,
			],
		];
		// When the producer emits typed record_entries claims, swap defaults here.
		if ( ! is_array( $envelope ) ) {
			return $default;
		}
		$record_entries = $envelope['record_entries']['items'] ?? null;
		if ( ! is_array( $record_entries ) || empty( $record_entries ) ) {
			return $default;
		}
		$resolved = [];
		foreach ( $record_entries as $item ) {
			if ( ! is_array( $item ) ) {
				continue;
			}
			$kind = (string) ( $item['entryKind'] ?? $item['entry_kind'] ?? 'context' );
			// Normalize: reference uses 'context' for user-added notes;
			// envelope may carry 'note' as the semantic label.
			if ( 'note' === $kind ) {
				$kind = 'context';
			}
			$resolved[] = [
				'kind'   => $kind,
				'date'   => (string) ( $item['entryDate'] ?? $item['entry_date'] ?? '' ),
				'title'  => (string) ( $item['renderedText'] ?? $item['rendered_text'] ?? '' ),
				'detail' => (string) ( $item['dataSource'] ?? $item['data_source'] ?? '' ),
				// Meeting entries always wrap in a TimelineEntry_entryLink per reference.
				'link'   => 'meeting' === $kind,
			];
		}
		return empty( $resolved ) ? $default : $resolved;
	}
}

if ( ! function_exists( 'dailyos_account_detail_the_record_entry' ) ) {
	/**
	 * Render the account detail record entry.
	 *
	 * @param array<string,mixed> $entry Record entry.
	 * @return string
	 */
	function dailyos_account_detail_the_record_entry( array $entry ): string {
		$kind       = $entry['kind'];
		$dot_class  = 'TimelineEntry_dot TimelineEntry_dot' . ucfirst( $kind );
		$type_class = 'TimelineEntry_typeBadge TimelineEntry_type' . ucfirst( $kind );
		$type_label = ucfirst( 'context' === $kind ? 'note' : $kind );

		$inner  = '<div class="TimelineEntry_entry">';
		$inner .= '<div class="' . esc_attr( $dot_class ) . '"></div>';
		$inner .= '<div class="TimelineEntry_dateLine">';
		$inner .= '<span class="TimelineEntry_date">' . esc_html( $entry['date'] ) . '</span>';
		$inner .= '<span class="' . esc_attr( $type_class ) . '">' . esc_html( $type_label ) . '</span>';
		$inner .= '</div>';
		$inner .= '<div class="TimelineEntry_title">' . esc_html( $entry['title'] ) . '</div>';
		$inner .= '<div class="TimelineEntry_detail">' . esc_html( $entry['detail'] ) . '</div>';
		$inner .= '</div>';

		if ( ! empty( $entry['link'] ) ) {
			return '<a href="#" class="TimelineEntry_entryLink">' . $inner . '</a>';
		}
		return $inner;
	}
}
