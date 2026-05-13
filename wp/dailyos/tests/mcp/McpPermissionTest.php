<?php
/**
 * DailyOS MCP permission tests.
 *
 * @package DailyOS
 */

declare(strict_types=1);

use DailyOS\DailyOS_Ability_Registry;
use DailyOS\Mcp\DailyOS_Mcp_Audit;
use DailyOS\Mcp\DailyOS_Mcp_Permission;
use DailyOS\Mcp\DailyOS_Mcp_Roles;
use DailyOS\Mcp\DailyOS_Mcp_Server;
use PHPUnit\Framework\TestCase;
use WP\MCP\Core\McpAdapter;

/**
 * Verifies MCP invocation requires both WordPress caps and resolved scopes.
 */
final class DailyOS_McpPermissionTest extends TestCase {
	/**
	 * Temporary inventory paths.
	 *
	 * @var array<int, string>
	 */
	private array $temporary_files = [];

	/**
	 * Reset WordPress test doubles.
	 */
	protected function setUp(): void {
		parent::setUp();
		dailyos_test_reset_globals();

		$GLOBALS['dailyos_test_user_can_callback'] = static function ( int $user_id, string $capability ): bool {
			return 42 === $user_id && 'dailyos_invoke_mcp_ability' === $capability;
		};
	}

	/**
	 * Remove temporary inventory files.
	 */
	protected function tearDown(): void {
		foreach ( $this->temporary_files as $temporary_file ) {
			if ( is_file( $temporary_file ) ) {
				// phpcs:ignore WordPress.WP.AlternativeFunctions.unlink_unlink -- Test fixture cleanup.
				unlink( $temporary_file );
			}
		}

		parent::tearDown();
	}

	/**
	 * Capability and scope together allow invocation.
	 */
	public function test_capability_and_scope_allow_invocation(): void {
		$permission = $this->create_permission(
			DailyOS_Mcp_Audit::EXPOSURE_INVOCABLE,
			static function (): array {
				return [ 'read.account_overview' ];
			}
		);
		$result     = $permission->check( 'dailyos/account-overview', 42 );

		$this->assertTrue( $result['allowed'] );
		$this->assertTrue( $result['wp_cap_ok'] );
		$this->assertTrue( $result['scope_ok'] );
		$this->assertSame( [], $result['missing_scopes'] );
	}

	/**
	 * Missing scopes reject invocation even with the WordPress cap.
	 */
	public function test_missing_scope_rejects_invocation(): void {
		$permission = $this->create_permission(
			DailyOS_Mcp_Audit::EXPOSURE_INVOCABLE,
			static function (): array {
				return [];
			}
		);
		$result     = $permission->check( 'dailyos/account-overview', 42 );

		$this->assertFalse( $result['allowed'] );
		$this->assertTrue( $result['wp_cap_ok'] );
		$this->assertFalse( $result['scope_ok'] );
		$this->assertSame( [ 'read.account_overview' ], $result['missing_scopes'] );
	}

	/**
	 * Missing WordPress capability rejects invocation even with scope.
	 */
	public function test_missing_capability_rejects_invocation(): void {
		$permission = $this->create_permission(
			DailyOS_Mcp_Audit::EXPOSURE_INVOCABLE,
			static function (): array {
				return [ 'read.account_overview' ];
			}
		);
		$result     = $permission->check( 'dailyos/account-overview', 99 );

		$this->assertFalse( $result['allowed'] );
		$this->assertFalse( $result['wp_cap_ok'] );
		$this->assertTrue( $result['scope_ok'] );
	}

	/**
	 * Metadata-only abilities cannot be invoked.
	 */
	public function test_metadata_only_rejects_even_when_capability_and_scope_pass(): void {
		$permission = $this->create_permission(
			DailyOS_Mcp_Audit::EXPOSURE_METADATA_ONLY,
			static function (): array {
				return [ 'read.account_overview' ];
			}
		);
		$result     = $permission->check( 'dailyos/account-overview', 42 );

		$this->assertFalse( $result['allowed'] );
		$this->assertTrue( $result['wp_cap_ok'] );
		$this->assertTrue( $result['scope_ok'] );
		$this->assertSame( DailyOS_Mcp_Audit::EXPOSURE_METADATA_ONLY, $result['mcp_exposure'] );
	}

	/**
	 * Unknown abilities are never allowed.
	 */
	public function test_unknown_ability_rejects_invocation(): void {
		$permission = $this->create_permission(
			DailyOS_Mcp_Audit::EXPOSURE_INVOCABLE,
			static function (): array {
				return [ 'read.account_overview' ];
			}
		);
		$result     = $permission->check( 'dailyos/not-found', 42 );

		$this->assertFalse( $result['allowed'] );
	}

	/**
	 * Wired MCP tool permission rejects capability-only, scope-only, and allows both.
	 */
	public function test_registered_tool_permission_uses_capability_and_scope(): void {
		$registry = new DailyOS_Ability_Registry(
			$this->create_inventory_file(
				[
					[
						'name'            => 'account-overview',
						'category'        => 'Read',
						'description'     => 'Account overview.',
						'mcp_exposure'    => DailyOS_Mcp_Audit::EXPOSURE_INVOCABLE,
						'required_scopes' => [ 'read.account_overview' ],
						'input_schema'    => [ 'type' => 'object' ],
						'output_schema'   => [ 'type' => 'object' ],
					],
				]
			)
		);
		$scopes   = [ 'read.account_overview' ];

		DailyOS_Mcp_Roles::ensure_user();
		DailyOS_Mcp_Server::bootstrap(
			$registry,
			static function () use ( &$scopes ): array {
				return $scopes;
			}
		);
		do_action( 'mcp_adapter_init', McpAdapter::instance() );

		$tool = $GLOBALS['dailyos_test_mcp_server_calls'][0]['tools'][0] ?? null;

		$this->assertIsObject( $tool );
		$this->assertTrue( method_exists( $tool, 'check_permission' ) );

		$substrate_user_id = (int) get_option( DailyOS_Mcp_Roles::USER_ID_OPTION );

		$GLOBALS['dailyos_test_user_can_callback'] = static function ( int $user_id, string $capability ) use ( $substrate_user_id ): bool {
			return $substrate_user_id === $user_id && 'dailyos_invoke_mcp_ability' === $capability;
		};
		$scopes                                    = [];

		$this->assertTrue( is_wp_error( $tool->check_permission( [] ) ) );

		$GLOBALS['dailyos_test_user_can_callback'] = static function (): bool {
			return false;
		};
		$scopes                                    = [ 'read.account_overview' ];

		$this->assertTrue( is_wp_error( $tool->check_permission( [] ) ) );

		$GLOBALS['dailyos_test_user_can_callback'] = static function ( int $user_id, string $capability ) use ( $substrate_user_id ): bool {
			return $substrate_user_id === $user_id && 'dailyos_invoke_mcp_ability' === $capability;
		};

		$this->assertTrue( $tool->check_permission( [] ) );
	}

	/**
	 * Create a permission checker with one ability fixture.
	 *
	 * @param string   $mcp_exposure MCP exposure value.
	 * @param callable $scope_resolver Scope resolver.
	 */
	private function create_permission( string $mcp_exposure, callable $scope_resolver ): DailyOS_Mcp_Permission {
		$registry = new DailyOS_Ability_Registry(
			$this->create_inventory_file(
				[
					[
						'name'            => 'account-overview',
						'category'        => 'Read',
						'mcp_exposure'    => $mcp_exposure,
						'required_scopes' => [ 'read.account_overview' ],
					],
				]
			)
		);

		return new DailyOS_Mcp_Permission( $registry, $scope_resolver );
	}

	/**
	 * Create a temporary ability inventory.
	 *
	 * @param array<int, array<string, mixed>> $abilities Ability descriptors.
	 */
	private function create_inventory_file( array $abilities ): string {
		$path = tempnam( sys_get_temp_dir(), 'dailyos-abilities-' );

		$this->assertIsString( $path );
		$this->temporary_files[] = $path;

		$inventory = [
			'schema_version' => '1.0',
			'abilities'      => $abilities,
		];
		$encoded   = wp_json_encode( $inventory );

		$this->assertIsString( $encoded );

		// phpcs:ignore WordPress.WP.AlternativeFunctions.file_system_operations_copy -- Test fixture writes a temporary inventory file.
		copy( 'data://text/plain,' . rawurlencode( $encoded ), $path );

		return $path;
	}
}
