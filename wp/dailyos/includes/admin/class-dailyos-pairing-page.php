<?php
/**
 * DailyOS pairing admin page shell.
 *
 * @package DailyOS
 */

declare(strict_types=1);

namespace DailyOS\Admin;

use DailyOS\Transport\DailyOS_Credential_Store;
use DailyOS\Transport\DailyOS_Hmac_Signer;
use DailyOS\Transport\DailyOS_Runtime_Client;

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
	 * Handle pairing form submission.
	 *
	 * @return void
	 */
	public static function handle_submit(): void {
		if ( false === check_admin_referer( 'dailyos_pairing_submit', 'dailyos_pairing_nonce' ) ) {
			add_settings_error(
				'dailyos_pairing',
				'dailyos_pairing_nonce_failure',
				__( 'WP nonce failure — refresh page and retry', 'dailyos' ),
				'error'
			);
			return;
		}

		$pairing_code = isset( $_POST['dailyos_pairing_code'] )
			? sanitize_text_field( wp_unslash( $_POST['dailyos_pairing_code'] ) )
			: '';

		if ( '' === $pairing_code ) {
			add_settings_error(
				'dailyos_pairing',
				'dailyos_pairing_missing_code',
				__( 'Enter a pairing code before connecting DailyOS.', 'dailyos' ),
				'error'
			);
			return;
		}

		$current_user     = wp_get_current_user();
		$current_user_log = is_object( $current_user ) && isset( $current_user->user_login )
			? (string) $current_user->user_login
			: '';
		$wp_context       = [
			'site_url'         => get_site_url(),
			'admin_user_id'    => get_current_user_id(),
			'admin_user_login' => $current_user_log,
		];
		$credential_store = new DailyOS_Credential_Store();
		$runtime_client   = new DailyOS_Runtime_Client( $credential_store, new DailyOS_Hmac_Signer() );
		$result           = $runtime_client->handshake( $pairing_code, $wp_context );

		if ( true === ( $result['ok'] ?? false ) ) {
			$now_gmt = gmdate( 'Y-m-d H:i:s', time() );

			$credential_store->save_marker(
				[
					'instance_id'      => $result['instance_id'] ?? '',
					'session_id'       => $result['session_id'] ?? '',
					'granted_scopes'   => $result['scopes'] ?? [],
					'endpoint_version' => $result['endpoint_version'] ?? '',
					'paired_at_gmt'    => $now_gmt,
					'last_use_gmt'     => $now_gmt,
				]
			);

			update_option( 'dailyos_pairing_status', 'paired', false );
			add_settings_error(
				'dailyos_pairing',
				'dailyos_pairing_success',
				__( 'DailyOS pairing completed.', 'dailyos' ),
				'success'
			);
			return;
		}

		$error      = isset( $result['error'] ) && is_array( $result['error'] ) ? $result['error'] : [];
		$error_code = isset( $error['code'] ) ? (string) $error['code'] : '';

		add_settings_error(
			'dailyos_pairing',
			'dailyos_pairing_failed',
			self::message_for_error_code( $error_code ),
			'error'
		);
	}

	/**
	 * Render the pairing form shell.
	 *
	 * @return void
	 */
	public static function render(): void {
		if ( ! current_user_can( 'manage_options' ) ) {
			wp_die( esc_html__( 'You do not have permission to manage DailyOS.', 'dailyos' ) );
		}

		$request_method = isset( $_SERVER['REQUEST_METHOD'] )
			? sanitize_text_field( wp_unslash( $_SERVER['REQUEST_METHOD'] ) )
			: 'GET';

		if ( 'POST' === strtoupper( $request_method ) ) {
			self::handle_submit();
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

	/**
	 * Map runtime pairing errors to typed admin messages.
	 *
	 * @param string $error_code Runtime error code.
	 * @return string Admin-facing message.
	 */
	private static function message_for_error_code( string $error_code ): string {
		return match ( $error_code ) {
			DailyOS_Credential_Store::ERROR_TAMPERED_PAIRING_CODE => __( 'The pairing code could not be verified. Generate a new code and retry.', 'dailyos' ),
			DailyOS_Credential_Store::ERROR_RUNTIME_RESTART => __( 'The DailyOS runtime restarted during pairing. Generate a new code and retry.', 'dailyos' ),
			DailyOS_Credential_Store::ERROR_STALE_PAIRING_CODE => __( 'The pairing code has expired. Generate a fresh code and retry.', 'dailyos' ),
			DailyOS_Credential_Store::ERROR_CONCURRENT_ADMIN_PAIRING => __( 'Another administrator completed a pairing attempt first. Refresh this page before retrying.', 'dailyos' ),
			default => __( 'DailyOS pairing failed. Generate a new code and retry.', 'dailyos' ),
		};
	}
}
