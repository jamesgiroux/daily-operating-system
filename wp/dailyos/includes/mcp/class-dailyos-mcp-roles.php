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
	public const ROLE_SLUG      = 'dailyos_substrate';
	public const ROLE_NAME      = 'DailyOS Substrate';
	public const USERNAME       = 'dailyos_substrate';
	public const USER_EMAIL     = 'dailyos-substrate@localhost.invalid';
	public const USER_ID_OPTION = 'dailyos_substrate_user_id';
	public const CAPABILITIES   = [
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
	 * Ensure the dedicated substrate user exists and has the substrate role.
	 */
	public static function ensure_user(): int {
		self::register();

		if ( ! function_exists( 'get_user_by' ) ) {
			return 0;
		}

		$user = get_user_by( 'login', self::USERNAME );

		if ( false === $user && function_exists( 'wp_create_user' ) ) {
			$password = function_exists( 'wp_generate_password' )
				? wp_generate_password( 64, true, true )
				: self::fallback_password();
			$user_id  = wp_create_user( self::USERNAME, $password, self::USER_EMAIL );

			if ( function_exists( 'is_wp_error' ) && is_wp_error( $user_id ) ) {
				return 0;
			}

			$user = get_user_by( 'id', (int) $user_id );
		}

		if ( false === $user || ! is_object( $user ) || empty( $user->ID ) ) {
			return 0;
		}

		self::assign_user_role( $user );

		$user_id = (int) $user->ID;
		update_option( self::USER_ID_OPTION, $user_id, false );

		return $user_id;
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
	 * Delete the dedicated substrate user.
	 */
	public static function delete_user(): void {
		$user_id = self::substrate_user_id();

		if ( 0 < $user_id && ! function_exists( 'wp_delete_user' ) ) {
			$user_functions = ABSPATH . 'wp-admin/includes/user.php';

			if ( is_readable( $user_functions ) ) {
				require_once $user_functions;
			}
		}

		if ( 0 < $user_id && function_exists( 'wp_delete_user' ) ) {
			wp_delete_user( $user_id );
		}

		delete_option( self::USER_ID_OPTION );
	}

	/**
	 * Return the dedicated substrate user ID.
	 */
	public static function substrate_user_id(): int {
		$user_id = (int) get_option( self::USER_ID_OPTION, 0 );

		if ( 0 < $user_id && function_exists( 'get_user_by' ) && is_object( get_user_by( 'id', $user_id ) ) ) {
			return $user_id;
		}

		if ( ! function_exists( 'get_user_by' ) ) {
			return 0;
		}

		$user = get_user_by( 'login', self::USERNAME );

		return is_object( $user ) && ! empty( $user->ID ) ? (int) $user->ID : 0;
	}

	/**
	 * Return the pinned capability map.
	 *
	 * @return array<string, bool>
	 */
	public static function capabilities(): array {
		return self::CAPABILITIES;
	}

	/**
	 * Assign the substrate role to the dedicated user.
	 *
	 * @param object $user WordPress user object.
	 */
	private static function assign_user_role( object $user ): void {
		$roles = isset( $user->roles ) && is_array( $user->roles )
			? array_map( 'strval', $user->roles )
			: [];

		if ( in_array( self::ROLE_SLUG, $roles, true ) ) {
			return;
		}

		if ( method_exists( $user, 'set_role' ) ) {
			$user->set_role( self::ROLE_SLUG );
			return;
		}

		if ( method_exists( $user, 'add_role' ) ) {
			$user->add_role( self::ROLE_SLUG );
		}
	}

	/**
	 * Generate a password when the WordPress helper is not loaded.
	 */
	private static function fallback_password(): string {
		try {
			return bin2hex( random_bytes( 32 ) );
		} catch ( \Exception ) {
			return sha1( uniqid( self::USERNAME, true ) );
		}
	}
}
