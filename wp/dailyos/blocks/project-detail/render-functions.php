<?php
/**
 * Project detail outer-block server-side render (W2 sub-L0 SKELETON).
 *
 * W2 consumer-skeleton wiring per AC-W1.2 / AC-W1.9: the outer
 * project-detail block invokes the W1 producer `get_entity_intelligence`
 * (entity_type=project) via the paired DailyOS runtime, then emits the
 * outer wrapper and an <InnerBlocks /> placeholder so inner blocks can
 * project named slices of the composed envelope (ADR-0130 §4, V1.1 §13
 * "Entity-detail composites are 1 outer + N inner blocks"). Full inner
 * shape lands in W2 sub-L0; this stub is the lint-anchor for the
 * consumer-skeleton CI gate.
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

if ( ! function_exists( 'dailyos_project_detail_render' ) ) {
	/**
	 * Render the project-detail outer block. Invokes `get_entity_intelligence`
	 * via the runtime client (no direct DB reads from PHP), then emits the
	 * outer wrapper + inner-blocks slot.
	 *
	 * @param array<string, mixed> $attributes Block attributes.
	 * @param string               $content    Pre-rendered inner-block content from core.
	 * @return string Rendered HTML.
	 */
	function dailyos_project_detail_render( array $attributes, string $content = '' ): string {
		$project_id = isset( $attributes['project_id'] ) ? (string) $attributes['project_id'] : '';

		if ( '' === $project_id ) {
			return '<div class="wp-block-dailyos-project-detail is-empty">'
				. esc_html__( 'No project to show here.', 'dailyos' )
				. '</div>';
		}

		$runtime_client = apply_filters( 'dailyos_runtime_client_for_block', null );
		if ( ! is_object( $runtime_client ) || ! method_exists( $runtime_client, 'invoke_ability' ) ) {
			return '<div class="wp-block-dailyos-project-detail is-unavailable">'
				. esc_html__( 'Runtime unavailable.', 'dailyos' )
				. '</div>';
		}

		// W1 producer: get_entity_intelligence (entity_type=project).
		$response = $runtime_client->invoke_ability(
			'get_entity_intelligence',
			[
				'entity_type' => 'project',
				'entity_id'   => $project_id,
			]
		);

		if ( is_wp_error( $response ) ) {
			return '<div class="wp-block-dailyos-project-detail is-unavailable">'
				. esc_html__( 'Runtime unavailable.', 'dailyos' )
				. '</div>';
		}

		$wrapper_attrs = function_exists( 'get_block_wrapper_attributes' )
			? get_block_wrapper_attributes(
				[
					'class'                => 'wp-block-dailyos-project-detail',
					'data-ds-tier'         => 'pattern',
					'data-ds-name'         => 'ProjectDetail',
					'data-dailyos-surface' => 'project_detail',
				]
			)
			: 'class="wp-block-dailyos-project-detail" data-dailyos-surface="project_detail"';

		$out  = '<section ' . $wrapper_attrs . '>';
		$out .= '<div class="dailyos-inner-blocks-slot">' . $content . '</div>';
		$out .= '</section>';

		return $out;
	}
}
