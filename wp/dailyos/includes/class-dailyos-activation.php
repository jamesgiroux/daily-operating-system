<?php
/**
 * DailyOS lifecycle handlers.
 *
 * @package DailyOS
 */

declare(strict_types=1);

namespace DailyOS;

use DailyOS\Services\DailyOS_Namespace_Store;

/**
 * Activation, deactivation, and uninstall behavior.
 */
final class DailyOS_Activation {
	public const PAIRING_STATUS_OPTION = 'dailyos_pairing_status';
	public const PAIRING_MARKER_OPTION = 'dailyos_pairing_marker';
	public const PAIRING_RECORD_OPTION = 'dailyos_pairing_record';
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
			self::schedule_nonce_sweep();
			return;
		}

		if ( ! $namespace_used && $marker_present ) {
			update_option( self::PAIRING_STATUS_OPTION, 'needs_pairing', false );
			self::schedule_nonce_sweep();
			return;
		}

		if ( $namespace_used && $marker_present && self::marker_matches_prior_pair( $marker ) ) {
			self::schedule_nonce_sweep();
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
		$store = new DailyOS_Namespace_Store();
		$store->delete_reserved_namespace_data();
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
	 */
	private static function marker_matches_prior_pair( mixed $marker ): bool {
		if ( ! self::is_valid_marker( $marker ) ) {
			return false;
		}

		$record = get_option( self::PAIRING_RECORD_OPTION, [] );

		if ( ! is_array( $record ) || empty( $record['runtime_instance_id'] ) ) {
			return false;
		}

		return hash_equals( (string) $record['runtime_instance_id'], (string) $marker['runtime_instance_id'] );
	}

	/**
	 * Validate the prior-pairing marker shape.
	 */
	private static function is_valid_marker( mixed $marker ): bool {
		return is_array( $marker )
			&& 1 === (int) ( $marker['marker_version'] ?? 0 )
			&& isset( $marker['runtime_instance_id'], $marker['site_nonce_hash'], $marker['projection_version'] );
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
	 */
	private static function refuse_activation( string $message, string $code ): void {
		wp_die( new \WP_Error( $code, $message ) );
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
