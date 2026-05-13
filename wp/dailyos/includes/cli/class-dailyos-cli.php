<?php
/**
 * WP-CLI commands for the DailyOS plugin.
 *
 * @package DailyOS
 */

declare(strict_types=1);

namespace DailyOS\CLI;

use DailyOS\DailyOS_Activation;
use DailyOS\Services\DailyOS_Namespace_Store;

/**
 * Registers the wp dailyos command namespace.
 */
final class DailyOS_CLI {
	/**
	 * Register WP-CLI commands when WP-CLI is available.
	 */
	public static function register(): void {
		if ( ! defined( 'WP_CLI' ) || ! WP_CLI || ! class_exists( '\WP_CLI' ) ) {
			return;
		}

		\WP_CLI::add_command( 'dailyos', self::class );
	}

	/**
	 * Print plugin status.
	 *
	 * @param array<int, string>        $args Positional arguments.
	 * @param array<string, string|int> $assoc_args Associative arguments.
	 */
	public function status( array $args, array $assoc_args ): void {
		unset( $args, $assoc_args );

		$status = get_option( DailyOS_Activation::PAIRING_STATUS_OPTION, 'unknown' );
		$marker = get_option( DailyOS_Activation::PAIRING_MARKER_OPTION, false );

		\WP_CLI::line( 'pairing_status: ' . (string) $status );
		\WP_CLI::line( 'pairing_marker: ' . ( false === $marker ? 'absent' : 'present' ) );
	}

	/**
	 * Inspect or normalize the DailyOS namespace.
	 *
	 * @param array<int, string>        $args Positional arguments.
	 * @param array<string, string|int> $assoc_args Associative arguments.
	 */
	public function repair_namespace( array $args, array $assoc_args ): void {
		unset( $args );

		$execute = self::flag_is_set( $assoc_args, 'execute' );
		$store   = new DailyOS_Namespace_Store();
		$report  = $store->get_reserved_namespace_report();

		self::print_report( $report );

		if ( ! $execute ) {
			\WP_CLI::line( 'Dry run only. Re-run with --execute to write safe defaults.' );
			return;
		}

		update_option( DailyOS_Activation::PAIRING_STATUS_OPTION, 'needs_pairing', false );
		\WP_CLI::success( 'Namespace repair executed. Pairing status is set to needs_pairing.' );
	}

	/**
	 * Inspect or repair projection envelope rows.
	 *
	 * @param array<int, string>        $args Positional arguments.
	 * @param array<string, string|int> $assoc_args Associative arguments.
	 */
	public function repair_projection( array $args, array $assoc_args ): void {
		unset( $args );

		if ( ! self::flag_is_set( $assoc_args, 'execute' ) ) {
			\WP_CLI::line( 'Dry run only. No projection storage exists in this scaffold.' );
			return;
		}

		\WP_CLI::success( 'Projection repair executed. No projection rows exist in this scaffold.' );
	}

	/**
	 * Return whether a flag was provided.
	 *
	 * @param array<string, string|int> $assoc_args Associative arguments.
	 */
	private static function flag_is_set( array $assoc_args, string $name ): bool {
		return array_key_exists( $name, $assoc_args ) && false !== $assoc_args[ $name ];
	}

	/**
	 * Print namespace report counts.
	 *
	 * @param array<string, array<int, string>> $report Namespace report.
	 */
	private static function print_report( array $report ): void {
		foreach ( [ 'options', 'post_meta', 'transients', 'tables' ] as $key ) {
			$count = isset( $report[ $key ] ) ? count( $report[ $key ] ) : 0;
			\WP_CLI::line( $key . ': ' . (string) $count );
		}
	}
}
