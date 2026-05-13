<?php
/**
 * DailyOS MCP exposure allowlist tests.
 *
 * @package DailyOS
 */

declare(strict_types=1);

use DailyOS\DailyOS_Ability_Registry;
use DailyOS\Mcp\DailyOS_Mcp_Audit;
use DailyOS\Mcp\DailyOS_Mcp_Permission;
use DailyOS\Mcp\DailyOS_Mcp_Server;
use PHPUnit\Framework\TestCase;
use WP\MCP\Core\McpAdapter;

/**
 * Verifies non-invocable abilities never reach tool enumeration.
 */
final class DailyOS_McpExposureNoneTest extends TestCase {
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
	 * Allowlist excludes None exposure and disallowed categories.
	 */
	public function test_build_allowlist_excludes_none_and_disallowed_categories(): void {
		$registry = new DailyOS_Ability_Registry(
			$this->create_inventory_file(
				[
					[
						'name'         => 'account-overview',
						'category'     => 'Read',
						'mcp_exposure' => 'Invocable',
					],
					[
						'name'         => 'hidden-report',
						'category'     => 'Read',
						'mcp_exposure' => 'None',
					],
					[
						'name'         => 'publish-note',
						'category'     => 'Publish',
						'mcp_exposure' => 'Invocable',
					],
				]
			)
		);
		$server   = $this->create_server( $registry );

		$this->assertSame( [ 'dailyos/account-overview' ], $server->build_allowlist() );
	}

	/**
	 * All-None inventories do not register DailyOS tools with the adapter.
	 */
	public function test_all_none_inventory_registers_no_dailyos_tools_with_adapter(): void {
		$registry = new DailyOS_Ability_Registry(
			$this->create_inventory_file(
				[
					[
						'name'         => 'account-overview',
						'category'     => 'Read',
						'mcp_exposure' => 'None',
					],
					[
						'name'         => 'transform-preview',
						'category'     => 'Transform',
						'mcp_exposure' => 'None',
					],
				]
			)
		);
		$server   = DailyOS_Mcp_Server::bootstrap(
			$registry,
			static function (): array {
				return [];
			}
		);

		$this->assertSame( [], $server->build_allowlist() );

		do_action( 'mcp_adapter_init', McpAdapter::instance() );

		$server_calls = $GLOBALS['dailyos_test_mcp_server_calls'];

		$this->assertCount( 1, $server_calls );
		$this->assertSame( [], $server_calls[0]['tools'] );
		$this->assertSame(
			[],
			array_values(
				array_filter(
					$server_calls[0]['tools'],
					static function ( string $tool_name ): bool {
						return str_starts_with( $tool_name, 'dailyos/' );
					}
				)
			)
		);
	}

	/**
	 * Generic MCP server listings filter DailyOS tools even when invocable abilities exist.
	 */
	public function test_generic_mcp_server_enumerates_zero_dailyos_tools(): void {
		$registry = new DailyOS_Ability_Registry(
			$this->create_inventory_file(
				[
					[
						'name'         => 'account-overview',
						'category'     => 'Read',
						'mcp_exposure' => 'Invocable',
					],
				]
			)
		);

		DailyOS_Mcp_Server::bootstrap(
			$registry,
			static function (): array {
				return [];
			}
		);

		McpAdapter::instance()->create_server(
			'generic-server',
			'wp/v2',
			'/mcp',
			'Generic MCP',
			'Generic WordPress MCP server',
			'0.1.0',
			[ \WP\MCP\Transport\HttpTransport::class ],
			null,
			null,
			[ 'dailyos/account-overview' ],
			[],
			[],
			static function (): bool {
				return true;
			}
		);

		$server_calls = $GLOBALS['dailyos_test_mcp_server_calls'];
		$generic_call = end( $server_calls );

		$this->assertSame( [], $generic_call['enumerated_tools'] );
		$this->assertSame(
			[],
			array_values(
				array_filter(
					$generic_call['enumerated_tools'],
					static function ( string $tool_name ): bool {
						return str_starts_with( $tool_name, 'dailyos-' ) || str_starts_with( $tool_name, 'dailyos/' );
					}
				)
			)
		);
	}

	/**
	 * Public invocation logger emits an MCP audit event.
	 */
	public function test_public_log_invocation_emits_audit_event(): void {
		$registry = new DailyOS_Ability_Registry( $this->create_inventory_file( [] ) );
		$server   = $this->create_server( $registry );

		$server->public_log_invocation( 'dailyos/account-overview', 42, DailyOS_Mcp_Audit::EXPOSURE_INVOCABLE, 'allowed' );

		$this->assertCount( 1, $GLOBALS['dailyos_test_audit_events'] );
		$this->assertSame(
			[
				'mcp_exposure_path'  => DailyOS_Mcp_Audit::EXPOSURE_INVOCABLE,
				'wp_user_id'         => 42,
				'ability_name'       => 'dailyos/account-overview',
				'scope_check_result' => 'allowed',
			],
			$GLOBALS['dailyos_test_audit_events'][0]
		);
	}

	/**
	 * Create a server wrapper for a registry.
	 *
	 * @param DailyOS_Ability_Registry $registry Ability registry.
	 */
	private function create_server( DailyOS_Ability_Registry $registry ): DailyOS_Mcp_Server {
		$permission = new DailyOS_Mcp_Permission(
			$registry,
			static function (): array {
				return [];
			}
		);

		return new DailyOS_Mcp_Server( $registry, $permission );
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
