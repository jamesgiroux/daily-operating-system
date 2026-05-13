<?php
/**
 * DailyOS settings admin page shell.
 *
 * @package DailyOS
 */

declare(strict_types=1);

namespace DailyOS\Admin;

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
	 * Render read-only placeholder settings.
	 */
	public static function render(): void {
		if ( ! current_user_can( 'manage_options' ) ) {
			wp_die( esc_html__( 'You do not have permission to manage DailyOS.', 'dailyos' ) );
		}

		echo '<div class="wrap">';
		echo '<h1>' . esc_html__( 'DailyOS Settings', 'dailyos' ) . '</h1>';
		echo '<h2>' . esc_html__( 'Pairing status', 'dailyos' ) . '</h2>';
		echo '<p>' . esc_html__( 'Needs pairing', 'dailyos' ) . '</p>';
		echo '<h2>' . esc_html__( 'Granted scopes', 'dailyos' ) . '</h2>';
		echo '<p>' . esc_html__( 'None', 'dailyos' ) . '</p>';
		echo '<h2>' . esc_html__( 'Last use', 'dailyos' ) . '</h2>';
		echo '<p>' . esc_html__( 'Never', 'dailyos' ) . '</p>';
		echo '</div>';
	}
}
