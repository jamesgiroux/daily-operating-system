<?php
/**
 * DailyOS custom MCP server registration.
 *
 * @package DailyOS
 */

declare(strict_types=1);

namespace DailyOS\Mcp;

use DailyOS\DailyOS_Ability_Registry;
use DailyOS\Transport\DailyOS_Credential_Store;
use DailyOS\Transport\DailyOS_Hmac_Signer;
use DailyOS\Transport\DailyOS_Runtime_Client;

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

		if ( function_exists( 'add_filter' ) ) {
			add_filter( 'mcp_adapter_tools_list', [ $server, 'filter_tools_list' ], 10, 2 );
			add_filter( 'mcp_adapter_pre_tool_call', [ $server, 'prepare_tool_call' ], 10, 4 );
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
			$this->build_tool_registrations(),
			[],
			[],
			function (): bool {
				if ( ! function_exists( 'is_user_logged_in' ) || ! is_user_logged_in() ) {
					return false;
				}

				if ( ! function_exists( 'get_current_user_id' ) || ! function_exists( 'user_can' ) ) {
					return false;
				}

				$substrate_user_id = DailyOS_Mcp_Roles::substrate_user_id();
				$current_user_id   = get_current_user_id();

				if ( 0 >= $substrate_user_id || $current_user_id !== $substrate_user_id ) {
					return false;
				}

				// phpcs:ignore WordPress.WP.Capabilities.Unknown -- Custom capability registered by DailyOS_Mcp_Roles.
				if ( ! user_can( $current_user_id, 'dailyos_invoke_mcp_ability' ) ) {
					return false;
				}

				$this->switch_to_substrate_user();

				return true;
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
	 * Filter MCP tools before list responses are exposed.
	 *
	 * @param array<int, mixed> $tools Tool DTOs.
	 * @param object            $server MCP server instance.
	 * @return array<int, mixed>
	 */
	public function filter_tools_list( array $tools, object $server ): array {
		if ( ! $this->is_dailyos_server( $server ) ) {
			return array_values(
				array_filter(
					$tools,
					function ( mixed $tool ): bool {
						return ! $this->is_dailyos_tool( $tool );
					}
				)
			);
		}

		$this->switch_to_substrate_user();

		return array_values(
			array_filter(
				$tools,
				function ( mixed $tool ): bool {
					$ability_name = $this->ability_name_from_tool( $tool );

					if ( null === $ability_name ) {
						return true;
					}

					$wp_user_id = function_exists( 'get_current_user_id' ) ? get_current_user_id() : 0;
					$result     = $this->permission->check( $ability_name, $wp_user_id, $this->resolved_scopes() );

					return $result['allowed'];
				}
			)
		);
	}

	/**
	 * Ensure DailyOS tool calls execute as the substrate user.
	 *
	 * @param mixed  $args Tool arguments.
	 * @param string $tool_name MCP tool name.
	 * @param object $mcp_tool MCP tool wrapper.
	 * @param object $server MCP server instance.
	 * @return mixed
	 */
	public function prepare_tool_call( mixed $args, string $tool_name, object $mcp_tool, object $server ): mixed {
		unset( $tool_name, $mcp_tool );

		if ( $this->is_dailyos_server( $server ) ) {
			$this->switch_to_substrate_user();
		}

		return $args;
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
				'mcp_exposure_path'  => $exposure_path,
				'actor_instance'     => $this->actor_instance(),
				'wp_user_id'         => $wp_user_id,
				'ability_name'       => $ability_name,
				'scope_check_result' => $scope_check_result,
			]
		);
	}

	/**
	 * Build adapter tool registrations with per-tool DailyOS permission callbacks.
	 *
	 * @return array<int, object|string>
	 */
	private function build_tool_registrations(): array {
		$tools          = [];
		$allowlist      = $this->build_allowlist();
		$allowlist_keys = array_fill_keys( $allowlist, true );

		foreach ( $this->load_ability_descriptors() as $ability ) {
			if ( ! isset( $allowlist_keys[ $this->ability_full_name( $ability ) ] ) ) {
				continue;
			}

			$tools[] = $this->build_tool_registration( $ability );
		}

		return array_values(
			array_filter(
				$tools,
				static function ( mixed $tool ): bool {
					return is_object( $tool ) || is_string( $tool );
				}
			)
		);
	}

	/**
	 * Build one adapter tool registration for an inventory ability.
	 *
	 * @param array<string, mixed> $ability Ability descriptor.
	 * @return object|string|null
	 */
	private function build_tool_registration( array $ability ): object|string|null {
		$ability_name = $this->ability_full_name( $ability );

		if ( ! class_exists( \WP\MCP\Domain\Tools\McpTool::class ) ) {
			return $ability_name;
		}

		$tool = \WP\MCP\Domain\Tools\McpTool::fromArray(
			[
				'name'         => $this->mcp_tool_name( $ability_name ),
				'description'  => isset( $ability['description'] ) ? (string) $ability['description'] : '',
				'inputSchema'  => isset( $ability['input_schema'] ) && is_array( $ability['input_schema'] )
					? $ability['input_schema']
					: [ 'type' => 'object' ],
				'outputSchema' => isset( $ability['output_schema'] ) && is_array( $ability['output_schema'] )
					? $ability['output_schema']
					: [ 'type' => 'object' ],
				'handler'      => function ( mixed $args ) use ( $ability_name ): array {
					return $this->invoke_runtime_ability( $ability_name, $args );
				},
				'permission'   => function ( mixed $args = [] ) use ( $ability_name ): bool|\WP_Error {
					unset( $args );

					$result = $this->authorize_ability_invocation( $ability_name );

					if ( $result['allowed'] ) {
						return true;
					}

					return new \WP_Error(
						'dailyos_mcp_permission_denied',
						__( 'DailyOS MCP permission denied.', 'dailyos' ),
						$result
					);
				},
			]
		);

		return is_wp_error( $tool ) ? null : $tool;
	}

	/**
	 * Invoke an ability through the runtime client.
	 *
	 * @param string $ability_name Full DailyOS ability name.
	 * @param mixed  $args Tool arguments.
	 * @return array<string, mixed>
	 */
	private function invoke_runtime_ability( string $ability_name, mixed $args ): array {
		$this->switch_to_substrate_user();

		$payload = is_array( $args ) ? $args : [ 'input' => $args ];
		$client  = new DailyOS_Runtime_Client( new DailyOS_Credential_Store(), new DailyOS_Hmac_Signer() );

		return $client->invoke_ability( $ability_name, $payload, $this->resolved_scopes() );
	}

	/**
	 * Authorize an MCP ability invocation.
	 *
	 * @param string $ability_name Full DailyOS ability name.
	 * @return array{allowed: bool, wp_cap_ok: bool, scope_ok: bool, missing_scopes: array<int, string>, mcp_exposure: string}
	 */
	private function authorize_ability_invocation( string $ability_name ): array {
		$this->switch_to_substrate_user();

		$wp_user_id = function_exists( 'get_current_user_id' ) ? get_current_user_id() : 0;
		$result     = $this->permission->check( $ability_name, $wp_user_id, $this->resolved_scopes() );

		$this->public_log_invocation(
			$ability_name,
			$wp_user_id,
			$result['mcp_exposure'],
			$this->scope_check_result( $result )
		);

		return $result;
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
	 * Resolve SurfaceClient scopes for the current request.
	 *
	 * @return array<int, string>
	 */
	private function resolved_scopes(): array {
		$scopes = null === $this->scope_resolver ? [] : ( $this->scope_resolver )();

		if ( ! is_array( $scopes ) ) {
			return [];
		}

		$normalized = [];

		foreach ( $scopes as $scope ) {
			if ( is_string( $scope ) && '' !== $scope ) {
				$normalized[] = $scope;
			}
		}

		return array_values( array_unique( $normalized ) );
	}

	/**
	 * Switch the current WordPress user to the dedicated substrate user.
	 */
	private function switch_to_substrate_user(): void {
		$user_id = DailyOS_Mcp_Roles::substrate_user_id();

		if ( 0 < $user_id && function_exists( 'wp_set_current_user' ) ) {
			wp_set_current_user( $user_id );
		}
	}

	/**
	 * Return whether a server is DailyOS-owned.
	 *
	 * @param object $server MCP server instance.
	 */
	private function is_dailyos_server( object $server ): bool {
		if ( ! method_exists( $server, 'get_server_route_namespace' ) ) {
			return false;
		}

		$namespace = (string) $server->get_server_route_namespace();

		return 'dailyos' === $namespace || str_starts_with( $namespace, 'dailyos/' );
	}

	/**
	 * Return whether a tool DTO looks like a DailyOS ability.
	 *
	 * @param mixed $tool Tool DTO.
	 */
	private function is_dailyos_tool( mixed $tool ): bool {
		if ( ! is_object( $tool ) || ! method_exists( $tool, 'getName' ) ) {
			return false;
		}

		$name = (string) $tool->getName();

		return str_starts_with( $name, 'dailyos/' )
			|| str_starts_with( $name, 'dailyos-' )
			|| str_starts_with( $name, 'dailyos_' );
	}

	/**
	 * Convert a DailyOS MCP tool DTO to its full ability name.
	 *
	 * @param mixed $tool Tool DTO.
	 */
	private function ability_name_from_tool( mixed $tool ): ?string {
		if ( ! is_object( $tool ) || ! method_exists( $tool, 'getName' ) ) {
			return null;
		}

		$name = (string) $tool->getName();

		if ( str_starts_with( $name, 'dailyos/' ) ) {
			return $name;
		}

		if ( str_starts_with( $name, 'dailyos-' ) ) {
			return 'dailyos/' . substr( $name, strlen( 'dailyos-' ) );
		}

		if ( str_starts_with( $name, 'dailyos_' ) ) {
			return 'dailyos/' . substr( $name, strlen( 'dailyos_' ) );
		}

		return null;
	}

	/**
	 * Return the audit actor instance from the pairing marker.
	 */
	private function actor_instance(): string {
		$marker = ( new DailyOS_Credential_Store() )->get_marker();

		if ( null === $marker || empty( $marker['plugin_instance_uuid'] ) ) {
			return '';
		}

		return (string) $marker['plugin_instance_uuid'];
	}

	/**
	 * Convert a DailyOS ability name to an MCP-safe tool name.
	 *
	 * @param string $ability_name Full DailyOS ability name.
	 */
	private function mcp_tool_name( string $ability_name ): string {
		return str_replace( '/', '-', $ability_name );
	}

	/**
	 * Convert a permission result to a compact audit result.
	 *
	 * @param array{allowed: bool, wp_cap_ok: bool, scope_ok: bool, missing_scopes: array<int, string>, mcp_exposure: string} $result Permission result.
	 */
	private function scope_check_result( array $result ): string {
		if ( $result['allowed'] ) {
			return 'allowed';
		}

		if ( ! $result['wp_cap_ok'] ) {
			return 'denied_wp_cap';
		}

		if ( ! $result['scope_ok'] ) {
			return 'denied_scope';
		}

		return 'denied_exposure';
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
