<?php
/**
 * DailyOS pairing admin page shell.
 *
 * @package DailyOS
 */

declare(strict_types=1);

namespace DailyOS\Admin;

/**
 * Registers and renders the pairing page shell.
 */
final class DailyOS_Pairing_Page {
	/**
	 * Register the top-level DailyOS menu page.
	 */
	public static function register(): void {
		add_menu_page(
			__( 'DailyOS', 'dailyos' ),
			__( 'DailyOS', 'dailyos' ),
			'manage_options',
			'dailyos-pairing',
			[ self::class, 'render' ],
			'dashicons-admin-site-alt3',
			58
		);
	}

	/**
	 * Render the pairing form shell. The runtime handshake lands later.
	 */
	public static function render(): void {
		if ( ! current_user_can( 'manage_options' ) ) {
			wp_die( esc_html__( 'You do not have permission to manage DailyOS.', 'dailyos' ) );
		}

		echo '<div class="wrap">';
		echo '<h1>' . esc_html__( 'DailyOS Pairing', 'dailyos' ) . '</h1>';
		settings_errors( 'dailyos_pairing' );
		echo '<div id="dailyos-pairing-status" role="status" aria-live="polite"></div>';
		echo '<div id="dailyos-pairing-error" role="alert"></div>';
		echo '<form method="post" action="">';
		wp_nonce_field( 'dailyos_pairing_submit', 'dailyos_pairing_nonce' );
		echo '<table class="form-table" role="presentation"><tbody><tr>';
		echo '<th scope="row"><label for="dailyos-pairing-code">' . esc_html__( 'Pairing code', 'dailyos' ) . '</label></th>';
		echo '<td>';
		echo '<input name="dailyos_pairing_code" id="dailyos-pairing-code" type="text" ';
		echo 'class="regular-text" autocomplete="one-time-code" />';
		echo '</td>';
		echo '</tr></tbody></table>';
		submit_button( __( 'Pair DailyOS', 'dailyos' ) );
		echo '</form>';
		echo '</div>';
	}
}
