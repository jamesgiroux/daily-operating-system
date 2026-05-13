<?php
/**
 * DailyOS MCP ability invocation permission checks.
 *
 * @package DailyOS
 */

declare(strict_types=1);

namespace DailyOS\Mcp;

use DailyOS\DailyOS_Ability_Registry;

/**
 * Combines WordPress capabilities with SurfaceClient scope checks.
 */
final class DailyOS_Mcp_Permission {
	/**
	 * Ability registry.
	 *
	 * @var DailyOS_Ability_Registry
	 */
	private DailyOS_Ability_Registry $registry;

	/**
	 * Resolved SurfaceClient scope callback.
	 *
	 * @var \Closure
	 */
	private \Closure $scope_resolver;

	/**
	 * Constructor.
	 *
	 * @param DailyOS_Ability_Registry $registry Ability registry.
	 * @param callable                 $scope_resolver Scope resolver returning strings.
	 */
	public function __construct( DailyOS_Ability_Registry $registry, callable $scope_resolver ) {
		$this->registry       = $registry;
		$this->scope_resolver = \Closure::fromCallable( $scope_resolver );
	}

	/**
	 * Check whether a WordPress user may invoke an ability through MCP.
	 *
	 * @param string $ability_full_name Full ability name, such as dailyos/account-overview.
	 * @param int    $wp_user_id WordPress user ID.
	 * @return array{allowed: bool, wp_cap_ok: bool, scope_ok: bool, missing_scopes: array<int, string>, mcp_exposure: string}
	 */
	public function check( string $ability_full_name, int $wp_user_id ): array {
		$wp_cap_ok = false;

		if ( function_exists( 'user_can' ) ) {
			// phpcs:ignore WordPress.WP.Capabilities.Unknown -- Custom capability registered by DailyOS_Mcp_Roles.
			$wp_cap_ok = (bool) user_can( $wp_user_id, 'dailyos_invoke_mcp_ability' );
		}

		$ability = $this->find_ability( $ability_full_name );

		if ( null === $ability ) {
			return [
				'allowed'        => false,
				'wp_cap_ok'      => $wp_cap_ok,
				'scope_ok'       => false,
				'missing_scopes' => [],
				'mcp_exposure'   => 'None',
			];
		}

		$required_scopes = $this->normalize_scope_list( $ability['required_scopes'] ?? [] );
		$resolved_scopes = $this->normalize_scope_list( ( $this->scope_resolver )() );
		$scope_lookup    = array_fill_keys( $resolved_scopes, true );
		$missing_scopes  = [];

		foreach ( $required_scopes as $required_scope ) {
			if ( ! isset( $scope_lookup[ $required_scope ] ) ) {
				$missing_scopes[] = $required_scope;
			}
		}

		$mcp_exposure = isset( $ability['mcp_exposure'] ) && is_string( $ability['mcp_exposure'] )
			? $ability['mcp_exposure']
			: 'None';
		$scope_ok     = [] === $missing_scopes;
		$allowed      = $wp_cap_ok && $scope_ok && DailyOS_Mcp_Audit::EXPOSURE_INVOCABLE === $mcp_exposure;

		return [
			'allowed'        => $allowed,
			'wp_cap_ok'      => $wp_cap_ok,
			'scope_ok'       => $scope_ok,
			'missing_scopes' => $missing_scopes,
			'mcp_exposure'   => $mcp_exposure,
		];
	}

	/**
	 * Find an ability descriptor by full ability name.
	 *
	 * @param string $ability_full_name Full ability name.
	 * @return array<string, mixed>|null
	 */
	private function find_ability( string $ability_full_name ): ?array {
		$inventory      = $this->registry->load_inventory();
		$abilities      = isset( $inventory['abilities'] ) && is_array( $inventory['abilities'] ) ? $inventory['abilities'] : [];
		$requested_name = $this->normalize_full_name( $ability_full_name );

		foreach ( $abilities as $ability ) {
			if ( ! is_array( $ability ) || empty( $ability['name'] ) ) {
				continue;
			}

			if ( $requested_name === $this->normalize_full_name( (string) $ability['name'] ) ) {
				return $ability;
			}
		}

		return null;
	}

	/**
	 * Normalize an inventory or full ability name to dailyos/<slug>.
	 *
	 * @param string $name Ability name.
	 */
	private function normalize_full_name( string $name ): string {
		$prefix = 'dailyos/';
		$suffix = str_starts_with( $name, $prefix ) ? substr( $name, strlen( $prefix ) ) : $name;

		return $prefix . $this->registry->normalize_name( $suffix );
	}

	/**
	 * Keep only string scopes.
	 *
	 * @param mixed $scopes Scope list.
	 * @return array<int, string>
	 */
	private function normalize_scope_list( mixed $scopes ): array {
		if ( ! is_array( $scopes ) ) {
			return [];
		}

		$normalized_scopes = [];

		foreach ( $scopes as $scope ) {
			if ( is_string( $scope ) && '' !== $scope ) {
				$normalized_scopes[] = $scope;
			}
		}

		return array_values( array_unique( $normalized_scopes ) );
	}
}
