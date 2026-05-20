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
	 * Pre-existing dailyos_-prefixed post type without a marker refuses activation.
	 */
	public function test_activation_refuses_with_pre_existing_dailyos_post_type(): void {
		$GLOBALS['dailyos_test_post_types'] = [ 'dailyos_briefing' ];

		$this->expectException( \RuntimeException::class );
		$this->expectExceptionMessage( 'DailyOS detected pre-existing dailyos_* data' );

		DailyOS_Activation::activate();
	}

	/**
	 * Pre-existing dailyos_-prefixed user_meta without a marker refuses activation.
	 */
	public function test_activation_refuses_with_pre_existing_dailyos_user_meta(): void {
		$GLOBALS['dailyos_test_user_meta_keys'] = [ 'dailyos_attribution' ];

		$this->expectException( \RuntimeException::class );
		$this->expectExceptionMessage( 'DailyOS detected pre-existing dailyos_* data' );

		DailyOS_Activation::activate();
	}

	/**
	 * Pre-existing non-substrate dailyos_-prefixed role refuses activation.
	 */
	public function test_activation_refuses_with_pre_existing_foreign_dailyos_role(): void {
		add_role( 'dailyos_intruder', 'DailyOS Intruder', [ 'read' => true ] );

		$this->expectException( \RuntimeException::class );
		$this->expectExceptionMessage( 'DailyOS detected pre-existing dailyos_* data' );

		DailyOS_Activation::activate();
	}

	/**
	 * Real-runtime marker shape (substrate emits sc_<hex> as runtime_instance_id
	 * and does not yet emit projection_version) passes prior-pair validation
	 * so legitimate paired reactivation proceeds.
	 */
	public function test_activation_accepts_real_runtime_marker_shape(): void {
		$marker = $this->valid_marker();
		// Substrate format: `format!("sc_{}", Uuid::new_v4().simple())` → sc_ + 32 hex chars.
		$marker['runtime_instance_id'] = 'sc_a1b2c3d4e5f60718293a4b5c6d7e8f90';
		$marker['instance_id']         = 'sc_a1b2c3d4e5f60718293a4b5c6d7e8f90';
		unset( $marker['projection_version'] );

		update_option( 'dailyos_projection_cache', 'present', false );
		update_option( DailyOS_Activation::PAIRING_MARKER_OPTION, $marker, false );

		DailyOS_Activation::activate();

		$this->assertGreaterThan( 0, (int) get_option( DailyOS_Mcp_Roles::USER_ID_OPTION ) );
	}

	/**
	 * Dirty namespace with a mismatching marker refuses activation.
	 */
	public function test_activation_branch_dirty_namespace_with_mismatching_marker_refuses(): void {
		$marker                        = $this->valid_marker();
		$marker['runtime_instance_id'] = '00000000-0000-4000-8000-000000000999';

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
	 * Plugin init registers the default dailyos_runtime_client_for_block filter so
	 * the WP block-registration render path resolves to a real transport client
	 * when paired. Without this registration every dailyos/* block renders
	 * is-empty regardless of runtime state, masking transport failures and
	 * pre-empting the typed runtime_unavailable_notice infrastructure.
	 *
	 * Asserts registration metadata (priority, callback identity, accepted_args)
	 * rather than just slot presence — a future refactor that re-registers
	 * under priority 5 with the wrong callback would still leave the live
	 * render path broken; this guard catches that.
	 */
	public function test_plugin_init_registers_default_runtime_client_filter(): void {
		$this->reset_plugin_init_state();

		DailyOS_Plugin::instance()->init();

		$this->assertNotEmpty(
			$GLOBALS['dailyos_test_filters']['dailyos_runtime_client_for_block'] ?? [],
			'init() must register the default dailyos_runtime_client_for_block filter so the live render path can reach the typed runtime_unavailable_notice when transport is unreachable'
		);
		$this->assertArrayHasKey(
			5,
			$GLOBALS['dailyos_test_filters']['dailyos_runtime_client_for_block'],
			'default filter must register at priority 5 so per-render overrides at priority 10 (REST preview, test fixtures) continue to win'
		);
		[ $callback, $accepted_args ] = $GLOBALS['dailyos_test_filters']['dailyos_runtime_client_for_block'][5][0];
		$this->assertIsArray( $callback, 'callback must be an instance-method tuple [ $plugin, method_name ]' );
		$this->assertSame( DailyOS_Plugin::instance(), $callback[0], 'callback must dispatch to the plugin singleton' );
		$this->assertSame( 'default_runtime_client_for_block', $callback[1], 'callback must point at default_runtime_client_for_block' );
		$this->assertSame( 1, $accepted_args, 'callback must accept the existing filter value to preserve per-render overrides' );
	}

	/**
	 * Default provider has three behavioral branches — a future refactor that
	 * drops the override-preservation short-circuit would silently break the
	 * REST preview + test seam. Pin each branch directly.
	 */
	public function test_default_runtime_client_for_block_returns_existing_when_already_a_client(): void {
		$store        = new \DailyOS\Transport\DailyOS_Credential_Store();
		$signer       = new \DailyOS\Transport\DailyOS_Hmac_Signer( $store );
		$pre_existing = new \DailyOS\Transport\DailyOS_Runtime_Client( $store, $signer );

		$result = DailyOS_Plugin::instance()->default_runtime_client_for_block( $pre_existing );

		$this->assertSame( $pre_existing, $result, 'default provider must defer to a priority-10 override that already supplied a client (REST preview + test seam)' );
	}

	public function test_default_runtime_client_for_block_returns_existing_when_unpaired(): void {
		// Test bootstrap leaves the credential store unpaired (no options seeded).
		$result = DailyOS_Plugin::instance()->default_runtime_client_for_block( null );

		$this->assertNull( $result, 'unpaired state must surface as null so the renderer falls through to its is-empty path; runtime-unavailable diagnostics fire post-pair when transport itself fails' );
	}

	/**
	 * End-to-end linkage: after init() runs, apply_filters from a block render
	 * resolves through the default provider. Regression guard for the bug
	 * shape where registration without resolution would not surface in unit
	 * tests that exercise either init() or render in isolation.
	 */
	public function test_init_to_render_filter_resolution_invokes_default_provider(): void {
		$this->reset_plugin_init_state();
		DailyOS_Plugin::instance()->init();

		// With no per-render override stacked on top, the priority-5 default
		// provider is the only callback in the chain. Unpaired state returns
		// null (same shape the renderer's no-client branch handles).
		$result = apply_filters( 'dailyos_runtime_client_for_block', null );

		$this->assertNull( $result, 'init() → apply_filters must reach the default provider; an unpaired store returns null, which the renderer routes to is-empty (separate from the runtime-unavailable case)' );
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
		$mismatching_runtime_instance['runtime_instance_id'] = '00000000-0000-4000-8000-000000000999';

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
			'marker_version'       => 1,
			'runtime_instance_id'  => '00000000-0000-4000-8000-000000000301',
			'runtime_url'          => 'http://127.0.0.1:54321',
			'site_nonce_hash'      => str_repeat( 'a', 64 ),
			'site_nonce_full'      => 'siteNonceAlpha123',
			'site_binding_digest'  => str_repeat( 'b', 64 ),
			'wp_site_id'           => 'install-1:1',
			'wp_install_uuid'      => 'install-1',
			'plugin_instance_uuid' => 'plugin-1',
			'projection_version'   => '2026.05.13',
			'instance_id'          => '00000000-0000-4000-8000-000000000301',
			'session_id'           => 'session-123',
			'granted_scopes'       => [ 'read.account_overview' ],
			'endpoint_version'     => 'v1',
			'paired_at_gmt'        => '2026-05-13 00:00:00',
			'last_use_gmt'         => '2026-05-13 00:00:00',
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
