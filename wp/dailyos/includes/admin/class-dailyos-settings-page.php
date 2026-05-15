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
	 * Action name used by the revoke-pairing admin-post handler.
	 *
	 * @var string
	 */
	public const REVOKE_ACTION = 'dailyos_revoke_pairing';

	/**
	 * Register the DailyOS settings submenu and revoke-action handler.
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

		add_action( 'admin_post_' . self::REVOKE_ACTION, [ self::class, 'handle_revoke' ] );
	}

	/**
	 * Process the revoke-pairing form submission.
	 *
	 * @return void
	 */
	public static function handle_revoke(): void {
		if ( ! current_user_can( 'manage_options' ) ) {
			wp_die( esc_html__( 'Insufficient permissions to revoke DailyOS pairing.', 'dailyos' ) );
		}

		check_admin_referer( self::REVOKE_ACTION );

		( new DailyOS_Credential_Store() )->clear();

		wp_safe_redirect(
			add_query_arg(
				'dailyos_revoked',
				'1',
				admin_url( 'admin.php?page=dailyos-settings' )
			)
		);
		exit;
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

		// Display-only flag set by handle_revoke() after a nonce-verified admin-post action.
		// phpcs:ignore WordPress.Security.NonceVerification.Recommended
		if ( isset( $_GET['dailyos_revoked'] ) && '1' === sanitize_text_field( wp_unslash( $_GET['dailyos_revoked'] ) ) ) {
			echo '<div class="notice notice-success is-dismissible"><p>'
				. esc_html__( 'DailyOS pairing revoked.', 'dailyos' )
				. '</p></div>';
		}

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
		self::render_row( __( 'Site nonce hash', 'dailyos' ), self::format_site_nonce_hash( (string) ( $marker['site_nonce_hash'] ?? '' ) ) );
		self::render_row( __( 'Granted scopes', 'dailyos' ), $granted_scopes );
		self::render_row( __( 'Endpoint version', 'dailyos' ), (string) ( $marker['endpoint_version'] ?? '' ) );
		self::render_row( __( 'Last use', 'dailyos' ), self::format_last_use( (string) ( $marker['last_use_gmt'] ?? '' ) ) );
		echo '</tbody></table>';

		echo '<form method="post" action="' . esc_url( admin_url( 'admin-post.php' ) ) . '">';
		wp_nonce_field( self::REVOKE_ACTION );
		echo '<input type="hidden" name="action" value="' . esc_attr( self::REVOKE_ACTION ) . '">';
		echo '<p class="submit">';
		echo '<button type="submit" class="button button-secondary">'
			. esc_html__( 'Revoke pairing', 'dailyos' )
			. '</button>';
		echo '</p>';
		echo '</form>';

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
	 * Format the non-secret site nonce hash for display.
	 *
	 * @param string $site_nonce_hash Stored site nonce hash.
	 * @return string Display hash.
	 */
	private static function format_site_nonce_hash( string $site_nonce_hash ): string {
		if ( '' === $site_nonce_hash ) {
			return '';
		}

		if ( 12 >= strlen( $site_nonce_hash ) ) {
			return $site_nonce_hash;
		}

		return substr( $site_nonce_hash, 0, 12 ) . '…';
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
