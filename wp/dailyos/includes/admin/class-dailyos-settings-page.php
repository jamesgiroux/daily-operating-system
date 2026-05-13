<?php
/**
 * DailyOS settings admin page shell.
 *
 * @package DailyOS
 */

declare(strict_types=1);

namespace DailyOS\Admin;

use DailyOS\Transport\DailyOS_Credential_Store;

/**
 * Registers and renders read-only settings diagnostics.
 */
final class DailyOS_Settings_Page {
	/**
	 * Register the DailyOS settings submenu.
	 */
	public static function register(): void {
		add_submenu_page(
			'dailyos-pairing',
			__( 'DailyOS Settings', 'dailyos' ),
			__( 'Settings', 'dailyos' ),
			'manage_options',
			'dailyos-settings',
			[ self::class, 'render' ]
		);
	}

	/**
	 * Render read-only pairing settings.
	 *
	 * @return void
	 */
	public static function render(): void {
		if ( ! current_user_can( 'manage_options' ) ) {
			wp_die( esc_html__( 'You do not have permission to manage DailyOS.', 'dailyos' ) );
		}

		$credential_store = new DailyOS_Credential_Store();
		$marker           = $credential_store->get_marker();

		echo '<div class="wrap">';
		echo '<h1>' . esc_html__( 'DailyOS Settings', 'dailyos' ) . '</h1>';

		if ( null === $marker ) {
			echo '<p>' . esc_html__( 'Not paired', 'dailyos' ) . '</p>';
			echo '</div>';
			return;
		}

		$granted_scopes = isset( $marker['granted_scopes'] ) && is_array( $marker['granted_scopes'] )
			? implode( ', ', array_map( 'strval', $marker['granted_scopes'] ) )
			: '';

		if ( '' === $granted_scopes ) {
			$granted_scopes = __( 'None', 'dailyos' );
		}

		echo '<table class="widefat striped"><tbody>';
		self::render_row( __( 'Instance ID', 'dailyos' ), (string) ( $marker['instance_id'] ?? '' ) );
		self::render_row( __( 'Granted scopes', 'dailyos' ), $granted_scopes );
		self::render_row( __( 'Endpoint version', 'dailyos' ), (string) ( $marker['endpoint_version'] ?? '' ) );
		self::render_row( __( 'Last use', 'dailyos' ), self::format_last_use( (string) ( $marker['last_use_gmt'] ?? '' ) ) );
		echo '</tbody></table>';
		echo '</div>';
	}

	/**
	 * Render a read-only settings row.
	 *
	 * @param string $label Row label.
	 * @param string $value Row value.
	 * @return void
	 */
	private static function render_row( string $label, string $value ): void {
		echo '<tr>';
		echo '<th scope="row">' . esc_html( $label ) . '</th>';
		echo '<td>' . esc_html( $value ) . '</td>';
		echo '</tr>';
	}

	/**
	 * Format the last-use timestamp for display.
	 *
	 * @param string $last_use_gmt Last-use timestamp in GMT.
	 * @return string Display timestamp.
	 */
	private static function format_last_use( string $last_use_gmt ): string {
		if ( '' === $last_use_gmt ) {
			return __( 'Never', 'dailyos' );
		}

		$timestamp = strtotime( $last_use_gmt . ' GMT' );

		if ( false === $timestamp ) {
			return $last_use_gmt;
		}

		$age_seconds = time() - $timestamp;

		if ( 0 <= $age_seconds && ( 7 * DAY_IN_SECONDS ) > $age_seconds && function_exists( 'human_time_diff' ) ) {
			return sprintf(
				/* translators: %s: Human-readable time difference. */
				__( '%s ago', 'dailyos' ),
				human_time_diff( $timestamp, time() )
			);
		}

		return $last_use_gmt;
	}
}
