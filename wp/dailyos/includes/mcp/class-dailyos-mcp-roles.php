<?php
/**
 * DailyOS MCP role and capability registration.
 *
 * @package DailyOS
 */

declare(strict_types=1);

namespace DailyOS\Mcp;

/**
 * Registers the substrate role used for authenticated MCP invocation.
 */
final class DailyOS_Mcp_Roles {
	public const ROLE_SLUG    = 'dailyos_substrate';
	public const ROLE_NAME    = 'DailyOS Substrate';
	public const CAPABILITIES = [
		'read'                       => true,
		'dailyos_invoke_mcp_ability' => true,
	];

	/**
	 * Register or repair the DailyOS substrate role.
	 */
	public static function register(): void {
		if ( ! function_exists( 'get_role' ) || ! function_exists( 'add_role' ) ) {
			return;
		}

		$role = get_role( self::ROLE_SLUG );

		if ( null === $role ) {
			add_role( self::ROLE_SLUG, self::ROLE_NAME, self::CAPABILITIES );
			return;
		}

		$existing_capabilities = isset( $role->capabilities ) && is_array( $role->capabilities )
			? $role->capabilities
			: [];

		foreach ( self::CAPABILITIES as $capability => $grant ) {
			if ( array_key_exists( $capability, $existing_capabilities ) && $grant === $existing_capabilities[ $capability ] ) {
				continue;
			}

			if ( method_exists( $role, 'add_cap' ) ) {
				$role->add_cap( $capability, $grant );
				continue;
			}

			if ( function_exists( 'add_cap' ) ) {
				add_cap( $role, $capability, $grant );
			}
		}
	}

	/**
	 * Revoke the DailyOS substrate role.
	 */
	public static function revoke(): void {
		if ( ! function_exists( 'get_role' ) || ! function_exists( 'remove_role' ) ) {
			return;
		}

		if ( null !== get_role( self::ROLE_SLUG ) ) {
			remove_role( self::ROLE_SLUG );
		}
	}

	/**
	 * Return the pinned capability map.
	 *
	 * @return array<string, bool>
	 */
	public static function capabilities(): array {
		return self::CAPABILITIES;
	}
}
