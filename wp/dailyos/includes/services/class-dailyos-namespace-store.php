<?php
/**
 * Low-level namespace storage operations.
 *
 * @package DailyOS
 */

declare(strict_types=1);

namespace DailyOS\Services;

// phpcs:disable WordPress.DB.DirectDatabaseQuery.DirectQuery, WordPress.DB.DirectDatabaseQuery.NoCaching
// phpcs:disable WordPress.DB.DirectDatabaseQuery.SchemaChange, WordPress.DB.PreparedSQL.InterpolatedNotPrepared
// phpcs:disable WordPress.DB.PreparedSQL.NotPrepared

/**
 * Encapsulates direct WordPress database access for namespace repair/cleanup.
 */
final class DailyOS_Namespace_Store {
	/**
	 * Return every reserved DailyOS namespace item currently present.
	 *
	 * @return array<string, array<int, string>>
	 */
	public function get_reserved_namespace_report(): array {
		return [
			'options'    => $this->get_option_names_like( 'dailyos_' ),
			'post_meta'  => $this->get_post_meta_keys_like( '_dailyos_' ),
			'transients' => $this->get_dailyos_transient_option_names(),
			'tables'     => $this->get_dailyos_table_names(),
		];
	}

	/**
	 * Delete all DailyOS-owned options, post meta, transients, and tables.
	 */
	public function delete_reserved_namespace_data(): void {
		foreach ( $this->get_option_names_like( 'dailyos_' ) as $option_name ) {
			delete_option( $option_name );
		}

		$this->delete_dailyos_post_meta();
		$this->delete_dailyos_transients();
		$this->drop_consumed_nonces_table();
	}

	/**
	 * Delete all DailyOS-owned transients.
	 */
	public function delete_dailyos_transients(): void {
		global $wpdb;

		$transient_like = $wpdb->esc_like( '_transient_dailyos_' ) . '%';
		$timeout_like   = $wpdb->esc_like( '_transient_timeout_dailyos_' ) . '%';

		$wpdb->query(
			$wpdb->prepare(
				"DELETE FROM {$wpdb->options} WHERE option_name LIKE %s OR option_name LIKE %s",
				$transient_like,
				$timeout_like
			)
		);
	}

	/**
	 * Return option names with the given prefix.
	 *
	 * @param string $prefix Option name prefix.
	 * @return array<int, string>
	 */
	private function get_option_names_like( string $prefix ): array {
		global $wpdb;

		$like = $wpdb->esc_like( $prefix ) . '%';
		$sql  = $wpdb->prepare(
			"SELECT option_name FROM {$wpdb->options} WHERE option_name LIKE %s",
			$like
		);

		return array_map( 'strval', $wpdb->get_col( $sql ) );
	}

	/**
	 * Return post meta keys with the given prefix.
	 *
	 * @param string $prefix Post meta key prefix.
	 * @return array<int, string>
	 */
	private function get_post_meta_keys_like( string $prefix ): array {
		global $wpdb;

		$like = $wpdb->esc_like( $prefix ) . '%';
		$sql  = $wpdb->prepare(
			"SELECT DISTINCT meta_key FROM {$wpdb->postmeta} WHERE meta_key LIKE %s",
			$like
		);

		return array_map( 'strval', $wpdb->get_col( $sql ) );
	}

	/**
	 * Return DailyOS transient option names.
	 *
	 * @return array<int, string>
	 */
	private function get_dailyos_transient_option_names(): array {
		global $wpdb;

		$transient_like = $wpdb->esc_like( '_transient_dailyos_' ) . '%';
		$timeout_like   = $wpdb->esc_like( '_transient_timeout_dailyos_' ) . '%';
		$sql            = $wpdb->prepare(
			"SELECT option_name FROM {$wpdb->options} WHERE option_name LIKE %s OR option_name LIKE %s",
			$transient_like,
			$timeout_like
		);

		return array_map( 'strval', $wpdb->get_col( $sql ) );
	}

	/**
	 * Return DailyOS custom table names that currently exist.
	 *
	 * @return array<int, string>
	 */
	private function get_dailyos_table_names(): array {
		global $wpdb;

		$table = $this->get_consumed_nonces_table_name();
		$found = $wpdb->get_var( $wpdb->prepare( 'SHOW TABLES LIKE %s', $table ) );

		return $found === $table ? [ $table ] : [];
	}

	/**
	 * Delete all DailyOS post meta.
	 */
	private function delete_dailyos_post_meta(): void {
		global $wpdb;

		$like = $wpdb->esc_like( '_dailyos_' ) . '%';
		$wpdb->query(
			$wpdb->prepare(
				"DELETE FROM {$wpdb->postmeta} WHERE meta_key LIKE %s",
				$like
			)
		);
	}

	/**
	 * Drop the consumed nonce table if present.
	 */
	private function drop_consumed_nonces_table(): void {
		global $wpdb;

		$table = preg_replace( '/[^A-Za-z0-9_]/', '', $this->get_consumed_nonces_table_name() );
		$wpdb->query( "DROP TABLE IF EXISTS `{$table}`" );
	}

	/**
	 * Return the plugin-owned consumed nonce table name.
	 */
	private function get_consumed_nonces_table_name(): string {
		global $wpdb;

		return $wpdb->prefix . 'dailyos_consumed_nonces';
	}
}
