<?php
/**
 * DailyOS activation smoke tests.
 *
 * @package DailyOS
 */

declare(strict_types=1);

use DailyOS\DailyOS_Activation;
use DailyOS\DailyOS_Plugin;
use DailyOS\Mcp\DailyOS_Mcp_Roles;
use PHPUnit\Framework\TestCase;

/**
 * Smoke coverage for plugin bootstrap.
 */
final class DailyOS_ActivationTest extends TestCase {
	/**
	 * Reset WordPress test doubles.
	 */
	protected function setUp(): void {
		parent::setUp();
		dailyos_test_reset_globals();
	}

	/**
	 * Singleton returns the same instance.
	 */
	public function test_plugin_singleton_returns_same_instance(): void {
		$this->assertSame( DailyOS_Plugin::instance(), DailyOS_Plugin::instance() );
	}

	/**
	 * Plugin constants are defined.
	 */
	public function test_plugin_constants_are_defined(): void {
		$this->assertTrue( defined( 'DAILYOS_PLUGIN_FILE' ) );
		$this->assertTrue( defined( 'DAILYOS_PLUGIN_DIR' ) );
		$this->assertTrue( defined( 'DAILYOS_PLUGIN_URL' ) );
		$this->assertTrue( defined( 'DAILYOS_VERSION' ) );
		$this->assertSame( '0.1.0', DAILYOS_VERSION );
	}

	/**
	 * Clean namespace without a marker activates and seeds needs-pairing state.
	 */
	public function test_activation_branch_clean_namespace_without_marker_seeds_needs_pairing(): void {
		DailyOS_Activation::activate();

		$this->assertSame( 'needs_pairing', get_option( DailyOS_Activation::PAIRING_STATUS_OPTION ) );
		$this->assertGreaterThan( 0, (int) get_option( DailyOS_Mcp_Roles::USER_ID_OPTION ) );
		$this->assertNotFalse( get_user_by( 'login', DailyOS_Mcp_Roles::USERNAME ) );
		$this->assertNotSame( '', get_option( DailyOS_Activation::PLUGIN_INSTANCE_UUID_OPTION, '' ) );
	}

	/**
	 * Clean namespace with an orphan marker activates through the recovery path.
	 */
	public function test_activation_branch_clean_namespace_with_orphan_marker_activates(): void {
		update_option(
			DailyOS_Activation::PAIRING_MARKER_OPTION,
			[
				'marker_version' => 1,
				'instance_id'    => 'orphaned-runtime',
			],
			false
		);

		DailyOS_Activation::activate();

		$this->assertSame( 'needs_pairing', get_option( DailyOS_Activation::PAIRING_STATUS_OPTION ) );
		$this->assertGreaterThan( 0, (int) get_option( DailyOS_Mcp_Roles::USER_ID_OPTION ) );
	}

	/**
	 * Dirty namespace with a matching unified marker proceeds.
	 */
	public function test_activation_branch_dirty_namespace_with_matching_marker_proceeds(): void {
		update_option( 'dailyos_projection_cache', 'present', false );
		update_option( DailyOS_Activation::PAIRING_MARKER_OPTION, $this->valid_marker(), false );

		DailyOS_Activation::activate();

		$this->assertGreaterThan( 0, (int) get_option( DailyOS_Mcp_Roles::USER_ID_OPTION ) );
	}

	/**
	 * Dirty namespace without a marker refuses activation.
	 */
	public function test_activation_branch_dirty_namespace_without_marker_refuses(): void {
		update_option( 'dailyos_projection_cache', 'present', false );

		$this->expectException( \RuntimeException::class );
		$this->expectExceptionMessage( 'DailyOS detected pre-existing dailyos_* data' );

		DailyOS_Activation::activate();
	}

	/**
	 * Dirty namespace with a mismatching marker refuses activation.
	 */
	public function test_activation_branch_dirty_namespace_with_mismatching_marker_refuses(): void {
		$marker                        = $this->valid_marker();
		$marker['runtime_instance_id'] = 'runtime-mismatch';

		update_option( 'dailyos_projection_cache', 'present', false );
		update_option( DailyOS_Activation::PAIRING_MARKER_OPTION, $marker, false );

		$this->expectException( \RuntimeException::class );
		$this->expectExceptionMessage( 'DailyOS detected pre-existing dailyos_* data' );

		DailyOS_Activation::activate();
	}

	/**
	 * Activation fails closed when the dailyos_substrate user cannot be created.
	 *
	 * Every MCP request runs as `dailyos_substrate`; activation must surface
	 * user-creation failures rather than silently masking later denials.
	 */
	public function test_activation_refuses_when_substrate_user_creation_fails(): void {
		$GLOBALS['dailyos_test_force_wp_create_user_error'] = true;

		$this->expectException( \RuntimeException::class );
		$this->expectExceptionMessage( 'DailyOS could not create or recover the dailyos_substrate' );

		DailyOS_Activation::activate();
	}

	/**
	 * Activation-created plugin instance UUID is stable across reactivation.
	 */
	public function test_activation_creates_plugin_instance_uuid_idempotently(): void {
		DailyOS_Activation::activate();

		$first = get_option( DailyOS_Activation::PLUGIN_INSTANCE_UUID_OPTION );

		DailyOS_Activation::deactivate();
		DailyOS_Activation::activate();

		$this->assertSame( $first, get_option( DailyOS_Activation::PLUGIN_INSTANCE_UUID_OPTION ) );
	}

	/**
	 * Plugin init registers a callback for the scheduled nonce sweep hook.
	 */
	public function test_plugin_init_registers_nonce_sweep_callback(): void {
		$this->reset_plugin_init_state();

		DailyOS_Plugin::instance()->init();

		$this->assertNotFalse( has_action( 'dailyos_nonce_sweep' ) );
	}

	/**
	 * Malformed markers never match prior pairing.
	 *
	 * @dataProvider malformed_marker_provider
	 *
	 * @param mixed $marker Marker candidate.
	 */
	public function test_marker_matches_prior_pair_rejects_malformed_markers( mixed $marker ): void {
		$method = new \ReflectionMethod( DailyOS_Activation::class, 'marker_matches_prior_pair' );
		$method->setAccessible( true );

		$this->assertFalse( $method->invoke( null, $marker ) );
	}

	/**
	 * Marker fixtures that must not match.
	 *
	 * @return array<string, array{0: mixed}>
	 */
	public static function malformed_marker_provider(): array {
		$valid                          = self::static_valid_marker();
		$missing_required               = $valid;
		$mismatching_runtime_instance   = $valid;
		$missing_required['session_id'] = '';
		$mismatching_runtime_instance['runtime_instance_id'] = 'runtime-mismatch';

		return [
			'not-array'                    => [ 'not-a-marker' ],
			'missing-required-field'       => [ $missing_required ],
			'mismatching-runtime-instance' => [ $mismatching_runtime_instance ],
		];
	}

	/**
	 * Uninstall deletes the dedicated substrate user.
	 */
	public function test_uninstall_deletes_substrate_user(): void {
		DailyOS_Activation::activate();

		$user_id = (int) get_option( DailyOS_Mcp_Roles::USER_ID_OPTION );

		$this->assertGreaterThan( 0, $user_id );

		DailyOS_Activation::uninstall();

		$this->assertContains( $user_id, $GLOBALS['dailyos_test_deleted_users'] );
		$this->assertFalse( get_option( DailyOS_Mcp_Roles::USER_ID_OPTION, false ) );
	}

	/**
	 * Build a valid unified pairing marker.
	 *
	 * @return array<string, mixed>
	 */
	private function valid_marker(): array {
		return self::static_valid_marker();
	}

	/**
	 * Build a valid unified pairing marker for data providers.
	 *
	 * @return array<string, mixed>
	 */
	private static function static_valid_marker(): array {
		return [
			'marker_version'      => 1,
			'runtime_instance_id' => 'runtime-123',
			'runtime_url'         => 'http://127.0.0.1:54321',
			'site_nonce_hash'     => str_repeat( 'a', 64 ),
			'site_nonce_full'     => 'siteNonceAlpha123',
			'site_binding_digest' => str_repeat( 'b', 64 ),
			'wp_site_id'          => 'install-1:1',
			'wp_install_uuid'     => 'install-1',
			'plugin_instance_uuid' => 'plugin-1',
			'projection_version'  => '2026.05.13',
			'instance_id'         => 'runtime-123',
			'session_id'          => 'session-123',
			'granted_scopes'      => [ 'read.account_overview' ],
			'endpoint_version'    => 'v1',
			'paired_at_gmt'       => '2026-05-13 00:00:00',
			'last_use_gmt'        => '2026-05-13 00:00:00',
		];
	}

	/**
	 * Reset the plugin singleton initialization flag for hook-registration tests.
	 */
	private function reset_plugin_init_state(): void {
		$property = new \ReflectionProperty( DailyOS_Plugin::class, 'initialized' );
		$property->setAccessible( true );
		$property->setValue( DailyOS_Plugin::instance(), false );
	}
}
