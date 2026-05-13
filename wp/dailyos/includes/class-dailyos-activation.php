<?php
/**
 * DailyOS lifecycle handlers.
 *
 * @package DailyOS
 */

declare(strict_types=1);

namespace DailyOS;

use DailyOS\Mcp\DailyOS_Mcp_Roles;
use DailyOS\Services\DailyOS_Namespace_Store;

/**
 * Activation, deactivation, and uninstall behavior.
 */
final class DailyOS_Activation {
	public const PAIRING_STATUS_OPTION = 'dailyos_pairing_status';
	public const PAIRING_MARKER_OPTION = 'dailyos_pairing_marker';
	public const REPAIR_MESSAGE        =
		'DailyOS detected pre-existing dailyos_* data without a valid pairing marker. ' .
		'Run: wp dailyos repair-namespace to inspect.';

	/**
	 * Activate the plugin with namespace-vacancy checks.
	 */
	public static function activate(): void {
		self::assert_environment();

		$store          = new DailyOS_Namespace_Store();
		$report         = $store->get_reserved_namespace_report();
		$marker         = get_option( self::PAIRING_MARKER_OPTION, false );
		$marker_present = false !== $marker;
		$namespace_used = self::namespace_is_dirty( $report );

		if ( ! $namespace_used && ! $marker_present ) {
			update_option( self::PAIRING_STATUS_OPTION, 'needs_pairing', false );
			self::complete_activation();
			return;
		}

		if ( ! $namespace_used && $marker_present ) {
			update_option( self::PAIRING_STATUS_OPTION, 'needs_pairing', false );
			self::complete_activation();
			return;
		}

		if ( $namespace_used && $marker_present && self::marker_matches_prior_pair( $marker ) ) {
			self::complete_activation();
			return;
		}

		self::refuse_activation( self::REPAIR_MESSAGE, 'dailyos_namespace_dirty' );
	}

	/**
	 * Deactivate the plugin while preserving pairing state.
	 */
	public static function deactivate(): void {
		if ( function_exists( 'wp_clear_scheduled_hook' ) ) {
			wp_clear_scheduled_hook( 'dailyos_nonce_sweep' );
		}

		$store = new DailyOS_Namespace_Store();
		$store->delete_dailyos_transients();
	}

	/**
	 * Delete all DailyOS-owned WordPress state.
	 */
	public static function uninstall(): void {
		DailyOS_Mcp_Roles::delete_user();

		$store = new DailyOS_Namespace_Store();
		$store->delete_reserved_namespace_data();

		DailyOS_Mcp_Roles::revoke();
	}

	/**
	 * Determine whether reserved namespace data exists beyond recoverable markers.
	 *
	 * @param array<string, array<int, string>> $report Namespace report.
	 */
	private static function namespace_is_dirty( array $report ): bool {
		foreach ( $report['options'] ?? [] as $option_name ) {
			if ( self::PAIRING_MARKER_OPTION === $option_name ) {
				continue;
			}

			if ( DailyOS_Mcp_Roles::USER_ID_OPTION === $option_name && self::has_recoverable_substrate_user() ) {
				continue;
			}

			if (
				self::PAIRING_STATUS_OPTION === $option_name
				&& 'needs_pairing' === get_option( self::PAIRING_STATUS_OPTION )
			) {
				continue;
			}

			return true;
		}

		return ! empty( $report['post_meta'] ) || ! empty( $report['transients'] ) || ! empty( $report['tables'] );
	}

	/**
	 * Check whether a marker belongs to the recorded prior pairing.
	 *
	 * @param mixed $marker Prior pairing marker.
	 */
	private static function marker_matches_prior_pair( mixed $marker ): bool {
		if ( ! self::is_valid_marker( $marker ) ) {
			return false;
		}

		if ( empty( $marker['instance_id'] ) ) {
			return false;
		}

		return hash_equals( (string) $marker['instance_id'], (string) $marker['runtime_instance_id'] );
	}

	/**
	 * Validate the prior-pairing marker shape.
	 *
	 * @param mixed $marker Prior pairing marker.
	 */
	private static function is_valid_marker( mixed $marker ): bool {
		if ( ! is_array( $marker ) || 1 !== (int) ( $marker['marker_version'] ?? 0 ) ) {
			return false;
		}

		foreach ( self::required_marker_fields() as $field ) {
			if ( empty( $marker[ $field ] ) || ! is_scalar( $marker[ $field ] ) ) {
				return false;
			}
		}

		return isset( $marker['granted_scopes'] ) && is_array( $marker['granted_scopes'] );
	}

	/**
	 * Return the unified marker fields that must be present and non-empty.
	 *
	 * @return array<int, string>
	 */
	private static function required_marker_fields(): array {
		return [
			'runtime_instance_id',
			'site_nonce_hash',
			'projection_version',
			'instance_id',
			'session_id',
			'endpoint_version',
			'paired_at_gmt',
			'last_use_gmt',
		];
	}

	/**
	 * Finish successful activation side effects.
	 */
	private static function complete_activation(): void {
		DailyOS_Mcp_Roles::ensure_user();
		self::schedule_nonce_sweep();
	}

	/**
	 * Determine whether the substrate user option points to the expected role.
	 */
	private static function has_recoverable_substrate_user(): bool {
		if ( ! function_exists( 'get_user_by' ) ) {
			return false;
		}

		$user_id = (int) get_option( DailyOS_Mcp_Roles::USER_ID_OPTION, 0 );

		if ( 0 >= $user_id ) {
			return false;
		}

		$user = get_user_by( 'id', $user_id );

		if ( ! is_object( $user ) || ! isset( $user->roles ) || ! is_array( $user->roles ) ) {
			return false;
		}

		return in_array( DailyOS_Mcp_Roles::ROLE_SLUG, array_map( 'strval', $user->roles ), true );
	}

	/**
	 * Schedule the nonce cleanup hook if WordPress cron is available.
	 */
	private static function schedule_nonce_sweep(): void {
		if ( ! function_exists( 'wp_next_scheduled' ) || ! function_exists( 'wp_schedule_event' ) ) {
			return;
		}

		if ( false === wp_next_scheduled( 'dailyos_nonce_sweep' ) ) {
			$offset = defined( 'HOUR_IN_SECONDS' ) ? HOUR_IN_SECONDS : 3600;
			wp_schedule_event( time() + $offset, 'hourly', 'dailyos_nonce_sweep' );
		}
	}

	/**
	 * Refuse activation through WordPress's standard fatal surface.
	 *
	 * @param string $message Activation failure message.
	 * @param string $code Activation failure code.
	 */
	private static function refuse_activation( string $message, string $code ): void {
		$error = new \WP_Error( $code, esc_html( $message ) );

		// phpcs:ignore WordPress.Security.EscapeOutput.OutputNotEscaped -- WP_Error message is escaped before wp_die().
		wp_die( $error );
	}

	/**
	 * Guard the minimum WordPress/PHP environment.
	 */
	private static function assert_environment(): void {
		if ( version_compare( PHP_VERSION, '8.1', '<' ) ) {
			self::refuse_activation( 'DailyOS requires PHP 8.1 or later.', 'dailyos_php_version' );
		}

		if ( function_exists( 'get_bloginfo' ) ) {
			$wp_version = (string) get_bloginfo( 'version' );

			if ( '' !== $wp_version && version_compare( $wp_version, '6.9', '<' ) ) {
				self::refuse_activation( 'DailyOS requires WordPress 6.9 or later.', 'dailyos_wp_version' );
			}
		}
	}
}
