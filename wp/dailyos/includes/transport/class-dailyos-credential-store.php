<?php
/**
 * DailyOS pairing marker and session material retrieval.
 *
 * @package DailyOS
 */

declare(strict_types=1);

namespace DailyOS\Transport;

use InvalidArgumentException;

/**
 * Stores non-secret pairing markers and retrieves process-local session material.
 */
final class DailyOS_Credential_Store {
	public const ERROR_TAMPERED_PAIRING_CODE    = 'tampered_pairing_code';
	public const ERROR_RUNTIME_RESTART          = 'runtime_restart';
	public const ERROR_STALE_PAIRING_CODE       = 'stale_pairing_code';
	public const ERROR_CONCURRENT_ADMIN_PAIRING = 'concurrent_admin_pairing';

	private const PAIRING_MARKER_OPTION = 'dailyos_pairing_marker';

	/**
	 * Cached session material for the current request.
	 *
	 * @var array{credential: DailyOS_Session_Credential, hmac_key: DailyOS_Hmac_Key}|null
	 */
	private ?array $session_material = null;

	/**
	 * Whether the session key filter has already been queried for this store.
	 *
	 * @var bool
	 */
	private bool $session_material_loaded = false;

	/**
	 * Determine whether a non-secret pairing marker exists.
	 *
	 * @return bool True when paired.
	 */
	public function is_paired(): bool {
		return null !== $this->get_marker();
	}

	/**
	 * Return the stored non-secret pairing marker.
	 *
	 * @return array<string, mixed>|null Pairing marker, or null.
	 */
	public function get_marker(): ?array {
		$marker = get_option( self::PAIRING_MARKER_OPTION, null );

		return is_array( $marker ) ? $marker : null;
	}

	/**
	 * Persist the non-secret pairing marker.
	 *
	 * @param array<string, mixed> $marker Marker fields.
	 * @return void
	 */
	public function save_marker( array $marker ): void {
		$runtime_instance_id = isset( $marker['runtime_instance_id'] ) ? (string) $marker['runtime_instance_id'] : '';
		$instance_id         = isset( $marker['instance_id'] ) ? (string) $marker['instance_id'] : $runtime_instance_id;

		$site_nonce_full = isset( $marker['site_nonce_full'] )
			? (string) $marker['site_nonce_full']
			: (string) ( $marker['site_nonce'] ?? '' );

		$normalized_marker = [
			'marker_version'       => 1,
			'runtime_instance_id'  => $runtime_instance_id,
			'surface_client_id'    => isset( $marker['surface_client_id'] ) ? (string) $marker['surface_client_id'] : $runtime_instance_id,
			'runtime_url'          => isset( $marker['runtime_url'] ) ? (string) $marker['runtime_url'] : '',
			'site_nonce_hash'      => isset( $marker['site_nonce_hash'] ) ? (string) $marker['site_nonce_hash'] : '',
			'site_nonce_full'      => $site_nonce_full,
			'site_binding_digest'  => isset( $marker['site_binding_digest'] ) ? (string) $marker['site_binding_digest'] : '',
			'wp_site_id'           => isset( $marker['wp_site_id'] ) ? (string) $marker['wp_site_id'] : '',
			'wp_install_uuid'      => isset( $marker['wp_install_uuid'] ) ? (string) $marker['wp_install_uuid'] : '',
			'plugin_instance_uuid' => isset( $marker['plugin_instance_uuid'] ) ? (string) $marker['plugin_instance_uuid'] : '',
			'projection_version'   => isset( $marker['projection_version'] ) ? (string) $marker['projection_version'] : '',
			'instance_id'          => $instance_id,
			'session_id'           => isset( $marker['session_id'] ) ? (string) $marker['session_id'] : '',
			'granted_scopes'       => $this->normalize_scopes( $marker['granted_scopes'] ?? [] ),
			'endpoint_version'     => isset( $marker['endpoint_version'] ) ? (string) $marker['endpoint_version'] : '',
			'paired_at_gmt'        => isset( $marker['paired_at_gmt'] ) ? (string) $marker['paired_at_gmt'] : '',
			'last_use_gmt'         => isset( $marker['last_use_gmt'] ) ? (string) $marker['last_use_gmt'] : '',
		];

		update_option( self::PAIRING_MARKER_OPTION, $normalized_marker, false );
	}

	/**
	 * Update the marker's last-use timestamp.
	 *
	 * @return void
	 */
	public function update_last_use(): void {
		$marker = $this->get_marker();

		if ( null === $marker ) {
			return;
		}

		$marker['last_use_gmt'] = gmdate( 'Y-m-d H:i:s', time() );

		update_option( self::PAIRING_MARKER_OPTION, $marker, false );
	}

	/**
	 * Clear the stored non-secret pairing marker.
	 *
	 * @return void
	 */
	public function clear(): void {
		delete_option( self::PAIRING_MARKER_OPTION );
	}

	/**
	 * Retrieve the process-local session credential from the session key filter.
	 *
	 * @return DailyOS_Session_Credential|null Session credential, or null.
	 */
	public function retrieve_session_key(): ?DailyOS_Session_Credential {
		$material = $this->retrieve_session_material();

		return null === $material ? null : $material['credential'];
	}

	/**
	 * Retrieve the process-local HMAC key from the session key filter.
	 *
	 * @internal
	 *
	 * @return DailyOS_Hmac_Key|null HMAC key, or null.
	 */
	public function retrieve_hmac_key(): ?DailyOS_Hmac_Key {
		$material = $this->retrieve_session_material();

		return null === $material ? null : $material['hmac_key'];
	}

	/**
	 * Register a high-priority guard for malformed session key filter results.
	 *
	 * @return void
	 */
	public function register_session_key_filter_safeguard(): void {
		add_filter(
			'dailyos_wp_bridge_session_key',
			static function ( mixed $candidate ): ?array {
				return self::normalize_session_key_result( $candidate );
			},
			PHP_INT_MAX,
			1
		);
	}

	/**
	 * Retrieve and cache both process-local transport secret wrappers.
	 *
	 * @return array{credential: DailyOS_Session_Credential, hmac_key: DailyOS_Hmac_Key}|null Session material.
	 */
	private function retrieve_session_material(): ?array {
		if ( $this->session_material_loaded ) {
			return $this->session_material;
		}

		$this->session_material_loaded = true;

		// DOS 599: MCP runtime runs as substrate user with invoke capability, not manage_options.
		if (
			function_exists( 'current_user_can' )
			&& ! (
				current_user_can( 'manage_options' )
				|| user_can( get_current_user_id(), 'dailyos_invoke_mcp_ability' )
			)
		) {
			return null;
		}

		$candidate = apply_filters( 'dailyos_wp_bridge_session_key', null );
		$material  = self::normalize_session_key_result( $candidate );

		if ( null === $material ) {
			return null;
		}

		try {
			$this->session_material = [
				'credential' => new DailyOS_Session_Credential( $material['bearer'], $material['session_id'] ),
				'hmac_key'   => new DailyOS_Hmac_Key( $material['hmac_key'] ),
			];
		} catch ( InvalidArgumentException ) {
			$this->session_material = null;
		}

		return $this->session_material;
	}

	/**
	 * Normalize granted scopes to a list of strings.
	 *
	 * @param mixed $scopes Scope candidate.
	 * @return array<int, string> Normalized scopes.
	 */
	private function normalize_scopes( mixed $scopes ): array {
		if ( ! is_array( $scopes ) ) {
			return [];
		}

		return array_values(
			array_map(
				static fn( mixed $scope ): string => (string) $scope,
				array_filter( $scopes, 'is_scalar' )
			)
		);
	}

	/**
	 * Validate and normalize the session key filter result.
	 *
	 * @param mixed $candidate Filter result.
	 * @return array{bearer: string, hmac_key: string, session_id: string}|null Normalized result.
	 */
	private static function normalize_session_key_result( mixed $candidate ): ?array {
		if ( null === $candidate ) {
			return null;
		}

		if ( ! is_array( $candidate ) ) {
			return null;
		}

		$bearer     = $candidate['bearer'] ?? null;
		$hmac_key   = $candidate['hmac_key'] ?? null;
		$session_id = $candidate['session_id'] ?? null;

		if ( ! is_string( $bearer ) || '' === $bearer ) {
			return null;
		}

		if ( ! is_string( $hmac_key ) || 32 !== strlen( $hmac_key ) ) {
			return null;
		}

		if ( ! is_string( $session_id ) || '' === $session_id ) {
			return null;
		}

		return [
			'bearer'     => $bearer,
			'hmac_key'   => $hmac_key,
			'session_id' => $session_id,
		];
	}
}
