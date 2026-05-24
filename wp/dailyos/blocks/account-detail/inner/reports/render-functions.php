<?php
/**
 * Account Reports (account-detail/inner/reports) — W2 L1 inner block.
 *
 * Registered as `dailyos/account-detail-reports`. Emits the canonical
 * Reports chapter DOM from .docs/design/reference/surfaces/account.html
 * lines 826-882 (section id `reports`). Verify with:
 *   python3 wp/dailyos/dev-tools/parity-check.py reports
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

if ( ! function_exists( 'dailyos_account_detail_reports_render' ) ) {
	/**
	 * Render the account detail reports.
	 *
	 * @param array<string, mixed> $attributes Block attributes.
	 * @param string               $content    Inner content.
	 * @param \WP_Block|null       $block      Parsed block.
	 * @return string
	 */
	function dailyos_account_detail_reports_render( array $attributes, string $content = '', $block = null ): string {
		unset( $attributes, $content, $block );

		$reports = [
			[
				'type'  => __( 'Health', 'dailyos' ),
				'title' => __( 'Account health report', 'dailyos' ),
			],
			[
				'type'  => __( 'Risk', 'dailyos' ),
				'title' => __( 'Risk briefing', 'dailyos' ),
			],
		];

		$out  = '<div id="outputs" class="entity-detail_marginLabelSection" data-ds-name="MarginGrid" data-ds-tier="pattern" data-ds-spec="patterns/MarginGrid.md">';
		$out .= '<div class="entity-detail_marginLabel">' . wp_kses_post( __( 'Out-<br/>puts', 'dailyos' ) ) . '</div>';
		$out .= '<div class="entity-detail_marginContent">';
		$out .= '<div class="ChapterHeading_heading" data-ds-name="ChapterHeading" data-ds-tier="pattern" data-ds-spec="patterns/ChapterHeading.md">';
		$out .= '<hr class="ChapterHeading_rule" />';
		$out .= '<div class="ChapterHeading_titleRow">';
		$out .= '<h2 class="ChapterHeading_title">' . esc_html__( 'Outputs', 'dailyos' ) . '</h2>';
		$out .= '</div>';
		$out .= '<div class="FreshnessIndicator_root FreshnessIndicator_strip">';
		$out .= '<span class="FreshnessIndicator_part"><span class="FreshnessIndicator_text">' . esc_html__( '2 reports available', 'dailyos' ) . '</span></span>';
		$out .= '<span class="FreshnessIndicator_part"><span class="FreshnessIndicator_separator">' . esc_html__( '·', 'dailyos' ) . '</span><span class="FreshnessIndicator_text FreshnessIndicator_timeText">' . esc_html__( 'Current intelligence', 'dailyos' ) . '</span></span>';
		$out .= '</div>';
		$out .= '<p class="ChapterHeading_epigraph">' . esc_html__( 'Generated reports · open to regenerate', 'dailyos' ) . '</p>';
		$out .= '</div>';
		$out .= '<div class="WorkSurface_reportGrid" data-dailyos-projection="account-detail-reports" data-ds-tier="pattern" data-ds-name="ReportGrid" data-ds-spec="patterns/WorkSurface.md">';
		foreach ( $reports as $report ) {
			$out .= '<article class="WorkSurface_reportCard">';
			$out .= '<div class="WorkSurface_reportType">' . esc_html( $report['type'] ) . '</div>';
			$out .= '<h3 class="WorkSurface_reportTitle">' . esc_html( $report['title'] ) . '</h3>';
			$out .= '<div class="WorkSurface_reportGen">' . esc_html__( 'Generated recently', 'dailyos' ) . '<br />' . esc_html__( 'Trigger: on-demand', 'dailyos' ) . '</div>';
			$out .= '<div class="WorkSurface_reportActions">';
			$out .= '<button type="button" class="WorkSurface_workBtn WorkSurface_workBtnPrimary">' . esc_html__( 'Open', 'dailyos' ) . '</button>';
			$out .= '<button type="button" class="WorkSurface_workBtn">' . esc_html__( 'Refresh', 'dailyos' ) . '</button>';
			$out .= '</div>';
			$out .= '</article>';
		}
		$out .= '</div>';
		$out .= '<p class="WorkSurface_reportFooterNote">' . esc_html__( 'Full-plan synthesis and export lives in the report engine.', 'dailyos' ) . '</p>';
		$out .= '</div>'; // .entity-detail_marginContent
		$out .= '</div>'; // .entity-detail_marginLabelSection
		return $out;
	}
}
