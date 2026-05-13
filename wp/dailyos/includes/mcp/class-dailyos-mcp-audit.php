<?php
/**
 * DailyOS MCP audit event sink.
 *
 * @package DailyOS
 */

declare(strict_types=1);

namespace DailyOS\Mcp;

/**
 * Emits normalized audit events for MCP exposure paths.
 */
final class DailyOS_Mcp_Audit {
	public const EXPOSURE_INVOCABLE     = 'Invocable';
	public const EXPOSURE_METADATA_ONLY = 'MetadataOnly';

	/**
	 * Required audit event keys.
	 *
	 * @var array<int, string>
	 */
	private const REQUIRED_KEYS = [
		'mcp_server_name',
		'mcp_exposure_path',
		'wp_user_id',
		'ability_name',
		'scope_check_result',
	];

	/**
	 * Emit an MCP audit event.
	 *
	 * @param array<string, mixed> $event Audit event.
	 *
	 * @throws \InvalidArgumentException When required keys are missing.
	 */
	public static function emit( array $event ): void {
		foreach ( self::REQUIRED_KEYS as $required_key ) {
			if ( ! array_key_exists( $required_key, $event ) ) {
				throw new \InvalidArgumentException( 'Missing MCP audit event key.' );
			}
		}

		if (
			defined( 'WP_DEBUG_LOG' )
			&& WP_DEBUG_LOG
			&& function_exists( 'has_action' )
			&& false === has_action( 'dailyos_mcp_audit' )
			&& function_exists( 'add_action' )
		) {
			add_action( 'dailyos_mcp_audit', [ self::class, 'fallback_log' ], PHP_INT_MAX, 1 );
		}

		if ( function_exists( 'do_action' ) ) {
			do_action( 'dailyos_mcp_audit', $event );
		}
	}

	/**
	 * Debug fallback for installs without an audit listener.
	 *
	 * @param array<string, mixed> $event Audit event.
	 */
	public static function fallback_log( array $event ): void {
		$message = function_exists( 'wp_json_encode' ) ? wp_json_encode( $event ) : '';

		if ( ! is_string( $message ) || '' === $message ) {
			$message = 'DailyOS MCP audit event';
		}

		// phpcs:ignore WordPress.PHP.DevelopmentFunctions.error_log_error_log -- Debug-only fallback when no audit listener is registered.
		error_log( $message );
	}
}
