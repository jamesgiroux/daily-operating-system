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
	public const PAIRING_STATUS_OPTION       = 'dailyos_pairing_status';
	public const PAIRING_MARKER_OPTION       = 'dailyos_pairing_marker';
	public const PLUGIN_INSTANCE_UUID_OPTION = 'dailyos_plugin_instance_uuid';
	public const WP_INSTALL_UUID_OPTION      = 'dailyos_wp_install_uuid';
	public const REPAIR_MESSAGE              =
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

			if ( in_array( $option_name, self::recoverable_identity_options(), true ) ) {
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

		foreach ( $report['roles'] ?? [] as $role_slug ) {
			if ( DailyOS_Mcp_Roles::ROLE_SLUG === $role_slug && self::has_recoverable_substrate_user() ) {
				continue;
			}

			return true;
		}

		return ! empty( $report['post_meta'] )
			|| ! empty( $report['transients'] )
			|| ! empty( $report['tables'] )
			|| ! empty( $report['post_types'] )
			|| ! empty( $report['user_meta'] );
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

		if ( ! isset( $marker['granted_scopes'] ) || ! is_array( $marker['granted_scopes'] ) ) {
			return false;
		}

		// Per L0 V4: the marker is a namespace-vacancy heuristic, not authoritative.
		// Tightening the well-formedness gate raises the cost of trivial forgery
		// without claiming the marker proves runtime ownership. Runtime-state
		// attestation remains load-bearing at first signed request.
		if ( ! self::is_runtime_id_like( (string) $marker['runtime_instance_id'] ) ) {
			return false;
		}

		if ( ! self::is_runtime_id_like( (string) $marker['instance_id'] ) ) {
			return false;
		}

		if ( ! self::is_well_formed_gmt( (string) $marker['paired_at_gmt'] ) ) {
			return false;
		}

		if ( ! self::is_well_formed_gmt( (string) $marker['last_use_gmt'] ) ) {
			return false;
		}

		if ( ! self::is_well_formed_endpoint_version( (string) $marker['endpoint_version'] ) ) {
			return false;
		}

		// projection_version is L0-listed but the W2 runtime does not yet emit it
		// (tracked separately as a substrate follow-up). Validate the shape only
		// when present so legitimate paired-reactivation does not quarantine.
		if (
			isset( $marker['projection_version'] )
			&& '' !== (string) $marker['projection_version']
			&& ! self::is_well_formed_projection_version( (string) $marker['projection_version'] )
		) {
			return false;
		}

		return true;
	}

	/**
	 * Accept either canonical RFC 4122 UUID form or the substrate's
	 * `sc_<32 hex>` surface-client-id shape (used as runtime_instance_id
	 * fallback when the runtime does not emit a separate runtime_instance_id).
	 *
	 * @param string $value Candidate runtime/instance id.
	 */
	private static function is_runtime_id_like( string $value ): bool {
		if ( 1 === preg_match( '/^[0-9a-fA-F]{8}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{12}$/', $value ) ) {
			return true;
		}

		return 1 === preg_match( '/^sc_[0-9a-fA-F]{32}$/', $value );
	}

	/**
	 * Accept ISO-8601 UTC timestamps either as RFC3339-Z or `Y-m-d H:i:s` GMT.
	 *
	 * @param string $value Candidate timestamp string.
	 */
	private static function is_well_formed_gmt( string $value ): bool {
		if ( 1 === preg_match( '/^\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}Z$/', $value ) ) {
			return false !== strtotime( $value );
		}

		if ( 1 === preg_match( '/^\d{4}-\d{2}-\d{2} \d{2}:\d{2}:\d{2}$/', $value ) ) {
			return false !== strtotime( $value . ' UTC' );
		}

		return false;
	}

	/**
	 * Endpoint versions follow the `v<N>` shape produced by the runtime pairing handshake.
	 *
	 * @param string $value Candidate endpoint-version string.
	 */
	private static function is_well_formed_endpoint_version( string $value ): bool {
		return 1 === preg_match( '/^v\d+$/', $value );
	}

	/**
	 * Accept calendar-version style `YYYY.MM.DD` or semver-style `vN.N.N`
	 * for projection_version when the runtime emits it.
	 *
	 * @param string $value Candidate projection-version string.
	 */
	private static function is_well_formed_projection_version( string $value ): bool {
		if ( 1 === preg_match( '/^\d{4}\.\d{2}\.\d{2}$/', $value ) ) {
			return true;
		}

		return 1 === preg_match( '/^v\d+(?:\.\d+){0,2}$/', $value );
	}

	/**
	 * Return the unified marker fields that must be present and non-empty.
	 *
	 * @return array<int, string>
	 */
	private static function required_marker_fields(): array {
		// projection_version is L0-listed but the W2 runtime does not yet emit it
		// (substrate-side follow-up tracked separately). Optional with a shape
		// gate in is_valid_marker(); promoted to required once the runtime emits.
		return [
			'runtime_instance_id',
			'site_nonce_hash',
			'instance_id',
			'session_id',
			'endpoint_version',
			'paired_at_gmt',
			'last_use_gmt',
		];
	}

	/**
	 * Return plugin-owned identity options that are safe to keep across reactivation.
	 *
	 * @return array<int, string>
	 */
	private static function recoverable_identity_options(): array {
		return [
			self::PLUGIN_INSTANCE_UUID_OPTION,
			self::WP_INSTALL_UUID_OPTION,
		];
	}

	/**
	 * Finish successful activation side effects.
	 *
	 * Fails closed if the dedicated substrate WordPress user cannot be created or
	 * recovered: every MCP request runs as `dailyos_substrate`, so a missing user
	 * must surface at activation rather than silently masking later request denials.
	 */
	private static function complete_activation(): void {
		self::ensure_identity_options();

		$substrate_user_id = DailyOS_Mcp_Roles::ensure_user();

		if ( 0 === $substrate_user_id ) {
			self::refuse_activation(
				'DailyOS could not create or recover the dailyos_substrate WordPress user required by the MCP server. Check user-creation permissions and reactivate.',
				'dailyos_substrate_user_missing'
			);
		}

		self::schedule_nonce_sweep();
	}

	/**
	 * Return this plugin install's stable instance UUID, creating it if absent.
	 */
	public static function plugin_instance_uuid(): string {
		return self::ensure_uuid_option( self::PLUGIN_INSTANCE_UUID_OPTION );
	}

	/**
	 * Return this WordPress install's stable DailyOS UUID, creating it if absent.
	 */
	public static function wp_install_uuid(): string {
		return self::ensure_uuid_option( self::WP_INSTALL_UUID_OPTION );
	}

	/**
	 * Ensure stable identity UUID options exist.
	 */
	private static function ensure_identity_options(): void {
		self::plugin_instance_uuid();
		self::wp_install_uuid();
	}

	/**
	 * Create an option UUID once and preserve it across reactivation.
	 *
	 * @param string $option Option name.
	 */
	private static function ensure_uuid_option( string $option ): string {
		$current = get_option( $option, '' );

		if ( is_string( $current ) && '' !== $current ) {
			return $current;
		}

		$uuid = function_exists( 'wp_generate_uuid4' )
			? wp_generate_uuid4()
			: sprintf( '%04x%04x-%04x-4%03x-8%03x-%04x%04x%04x', random_int( 0, 0xffff ), random_int( 0, 0xffff ), random_int( 0, 0xffff ), random_int( 0, 0xfff ), random_int( 0, 0xfff ), random_int( 0, 0xffff ), random_int( 0, 0xffff ), random_int( 0, 0xffff ) );

		update_option( $option, $uuid, false );

		return $uuid;
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
