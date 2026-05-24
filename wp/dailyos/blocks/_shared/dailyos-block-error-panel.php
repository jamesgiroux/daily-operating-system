<?php
/**
 * Shared DailyOS block error panel.
 *
 * @package DailyOS
 */

declare(strict_types=1);

if ( ! defined( 'ABSPATH' ) ) {
	return '';
}

if ( ! function_exists( 'dailyos_block_error_panel' ) ) {
	/**
	 * Render a compact block-scoped error state.
	 *
	 * @param string $block_name Block name.
	 * @param string $error_code Typed error code.
	 * @return string
	 */
	function dailyos_block_error_panel( string $block_name, string $error_code ): string {
		$labels  = [
			'InvalidEntityType'    => __( 'Choose a supported entity type.', 'dailyos' ),
			'InvalidEntityId'      => __( 'Choose a valid entity.', 'dailyos' ),
			'EntityNotFound'       => __( 'This entity is unavailable.', 'dailyos' ),
			'InvalidCategorySlug'  => __( 'Choose a valid workspace category.', 'dailyos' ),
			'CategoryNotAllowed'   => __( 'This category is not available for the entity.', 'dailyos' ),
			'FileNotFound'         => __( 'Choose an available workspace document.', 'dailyos' ),
			'PathTraversalAttempt' => __( 'Choose a workspace-relative document.', 'dailyos' ),
			'RuntimeNotPaired'     => __( 'Reconnect this surface to refresh content.', 'dailyos' ),
			'IngestionFailed'      => __( 'Intake is temporarily unavailable.', 'dailyos' ),
		];
		$message = $labels[ $error_code ] ?? __( 'Content is temporarily unavailable.', 'dailyos' );

		return sprintf(
			'<div class="dailyos-block-error-panel" data-block="%s" data-error-code="%s" role="status">%s</div>',
			esc_attr( $block_name ),
			esc_attr( $error_code ),
			esc_html( $message )
		);
	}
}
