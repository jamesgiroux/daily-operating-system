<?php
/**
 * Account overview dynamic block render entrypoint.
 *
 * Delegates to the renderer in render-functions.php so the same function
 * services both the block-registration render path and the editor preview
 * REST route.
 *
 * @package DailyOS
 *
 * @var array<string, mixed> $attributes Block attributes from core.
 */

declare(strict_types=1);

if ( ! defined( 'ABSPATH' ) ) {
	return '';
}

if ( ! function_exists( 'dailyos_account_overview_render' ) ) {
	require_once __DIR__ . '/render-functions.php';
}

$attributes = isset( $attributes ) && is_array( $attributes ) ? $attributes : [];
return dailyos_account_overview_render( $attributes );
