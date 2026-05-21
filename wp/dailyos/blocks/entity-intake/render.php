<?php
/**
 * Entity Intake dynamic block render entrypoint.
 *
 * @package DailyOS
 *
 * @var array<string, mixed> $attributes Block attributes from core.
 */

declare(strict_types=1);

if ( ! defined( 'ABSPATH' ) ) {
	return '';
}

if ( ! function_exists( 'dailyos_entity_intake_render' ) ) {
	require_once __DIR__ . '/render-functions.php';
}

$attributes = isset( $attributes ) && is_array( $attributes ) ? $attributes : [];
// phpcs:ignore WordPress.Security.EscapeOutput.OutputNotEscaped -- render functions escape output internally.
echo dailyos_entity_intake_render( $attributes );
