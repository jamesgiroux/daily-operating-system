<?php
/**
 * DailyOS custom MCP server registration.
 *
 * @package DailyOS
 */

declare(strict_types=1);

namespace DailyOS\Mcp;

use DailyOS\DailyOS_Ability_Registry;

/**
 * Registers the allowlisted DailyOS MCP server.
 */
final class DailyOS_Mcp_Server {
	private const SERVER_NAME        = 'DailyOS Substrate';
	private const DEFAULT_SERVER_ID  = 'dailyos-substrate';
	private const ALLOWED_CATEGORIES = [ 'Read', 'Transform' ];

	/**
	 * Ability registry.
	 *
	 * @var DailyOS_Ability_Registry
	 */
	private DailyOS_Ability_Registry $registry;

	/**
	 * Permission checker.
	 *
	 * @var DailyOS_Mcp_Permission
	 */
	private DailyOS_Mcp_Permission $permission;

	/**
	 * Optional scope resolver retained for later invocation hooks.
	 *
	 * @var \Closure|null
	 */
	private ?\Closure $scope_resolver;

	/**
	 * MCP server ID.
	 *
	 * @var string
	 */
	private string $server_id;

	/**
	 * Constructor.
	 *
	 * @param DailyOS_Ability_Registry $registry Ability registry.
	 * @param DailyOS_Mcp_Permission   $permission Permission checker.
	 * @param callable|null            $scope_resolver Optional scope resolver.
	 * @param string                   $server_id MCP server ID.
	 */
	public function __construct(
		DailyOS_Ability_Registry $registry,
		DailyOS_Mcp_Permission $permission,
		?callable $scope_resolver = null,
		string $server_id = self::DEFAULT_SERVER_ID
	) {
		$this->registry       = $registry;
		$this->permission     = $permission;
		$this->scope_resolver = null === $scope_resolver ? null : \Closure::fromCallable( $scope_resolver );
		$this->server_id      = $server_id;
	}

	/**
	 * Build and hook the DailyOS MCP server.
	 *
	 * @param DailyOS_Ability_Registry $registry Ability registry.
	 * @param callable                 $scope_resolver Scope resolver.
	 */
	public static function bootstrap( DailyOS_Ability_Registry $registry, callable $scope_resolver ): self {
		$permission = new DailyOS_Mcp_Permission( $registry, $scope_resolver );
		$server     = new self( $registry, $permission, $scope_resolver );

		if ( function_exists( 'add_action' ) ) {
			add_action( 'mcp_adapter_init', [ $server, 'register_with_adapter' ], 10, 1 );
		}

		return $server;
	}

	/**
	 * Register the server with the MCP adapter.
	 *
	 * @param object|null $adapter MCP adapter instance passed by the action.
	 */
	public function register_with_adapter( ?object $adapter = null ): void {
		if ( ! class_exists( \WP\MCP\Core\McpAdapter::class ) ) {
			return;
		}

		if ( null === $adapter ) {
			$adapter = \WP\MCP\Core\McpAdapter::instance();
		}

		if ( ! method_exists( $adapter, 'create_server' ) ) {
			return;
		}

		$adapter->create_server(
			$this->server_id,
			'dailyos/v1',
			'/mcp',
			self::SERVER_NAME,
			'DailyOS substrate-backed abilities (allowlisted)',
			'0.1.0',
			[ \WP\MCP\Transport\HttpTransport::class ],
			null,
			null,
			$this->build_allowlist(),
			[],
			[],
			static function (): bool {
				if ( ! function_exists( 'is_user_logged_in' ) || ! is_user_logged_in() ) {
					return false;
				}

				if ( ! function_exists( 'get_current_user_id' ) || ! function_exists( 'user_can' ) ) {
					return false;
				}

				// phpcs:ignore WordPress.WP.Capabilities.Unknown -- Custom capability registered by DailyOS_Mcp_Roles.
				return (bool) user_can( get_current_user_id(), 'dailyos_invoke_mcp_ability' );
			}
		);
	}

	/**
	 * Build the invocable MCP tool allowlist.
	 *
	 * @return array<int, string>
	 */
	public function build_allowlist(): array {
		$allowlist = [];

		foreach ( $this->load_ability_descriptors() as $ability ) {
			$mcp_exposure = isset( $ability['mcp_exposure'] ) && is_string( $ability['mcp_exposure'] )
				? $ability['mcp_exposure']
				: 'None';
			$category     = isset( $ability['category'] ) && is_string( $ability['category'] )
				? $ability['category']
				: '';

			if ( DailyOS_Mcp_Audit::EXPOSURE_INVOCABLE !== $mcp_exposure || ! in_array( $category, self::ALLOWED_CATEGORIES, true ) ) {
				continue;
			}

			$allowlist[] = $this->ability_full_name( $ability );
		}

		return $allowlist;
	}

	/**
	 * Return abilities that may be described to tests or metadata flows.
	 *
	 * @return array<int, array{name: string, mcp_exposure: string, category: string}>
	 */
	public function enumerable_abilities(): array {
		$enumerable_abilities = [];

		foreach ( $this->load_ability_descriptors() as $ability ) {
			$mcp_exposure = isset( $ability['mcp_exposure'] ) && is_string( $ability['mcp_exposure'] )
				? $ability['mcp_exposure']
				: 'None';
			$category     = isset( $ability['category'] ) && is_string( $ability['category'] )
				? $ability['category']
				: '';

			if ( ! in_array( $mcp_exposure, [ DailyOS_Mcp_Audit::EXPOSURE_INVOCABLE, DailyOS_Mcp_Audit::EXPOSURE_METADATA_ONLY ], true ) ) {
				continue;
			}

			$enumerable_abilities[] = [
				'name'         => $this->ability_full_name( $ability ),
				'mcp_exposure' => $mcp_exposure,
				'category'     => $category,
			];
		}

		return $enumerable_abilities;
	}

	/**
	 * Test helper for exercising the audit sink before invocation hooks land.
	 *
	 * @param string $ability_name Ability name.
	 * @param int    $wp_user_id WordPress user ID.
	 * @param string $exposure_path MCP exposure path.
	 * @param string $scope_check_result Scope check result.
	 */
	public function public_log_invocation( string $ability_name, int $wp_user_id, string $exposure_path, string $scope_check_result ): void {
		DailyOS_Mcp_Audit::emit(
			[
				'mcp_server_name'    => self::SERVER_NAME,
				'mcp_exposure_path'  => $exposure_path,
				'wp_user_id'         => $wp_user_id,
				'ability_name'       => $ability_name,
				'scope_check_result' => $scope_check_result,
			]
		);
	}

	/**
	 * Load valid ability descriptors from the registry inventory.
	 *
	 * @return array<int, array<string, mixed>>
	 */
	private function load_ability_descriptors(): array {
		$inventory = $this->registry->load_inventory();
		$abilities = isset( $inventory['abilities'] ) && is_array( $inventory['abilities'] ) ? $inventory['abilities'] : [];
		$filtered  = [];

		foreach ( $abilities as $ability ) {
			if ( is_array( $ability ) && ! empty( $ability['name'] ) ) {
				$filtered[] = $ability;
			}
		}

		return $filtered;
	}

	/**
	 * Normalize an inventory descriptor to a full DailyOS ability name.
	 *
	 * This mirrors DailyOS_Ability_Registry::normalize_ability_name via the
	 * public normalize_name wrapper.
	 *
	 * @param array<string, mixed> $ability Ability descriptor.
	 */
	private function ability_full_name( array $ability ): string {
		$name   = isset( $ability['name'] ) ? (string) $ability['name'] : '';
		$prefix = 'dailyos/';
		$suffix = str_starts_with( $name, $prefix ) ? substr( $name, strlen( $prefix ) ) : $name;

		return $prefix . $this->registry->normalize_name( $suffix );
	}
}
