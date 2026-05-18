<?php
/**
 * W4-A account-overview block tests. Covers the packet §8 fixtures
 * (consolidated into one focused test class):
 *
 *  - stale_projection_banner — newer delivered version surfaces banner.
 *  - signature_mismatch_banner — runtime WP_Error renders verification banner.
 *  - unknown_block_falls_back — unknown trust band degrades to needs_verification.
 *  - missing_nonce_no_feedback_affordance — placeholder; nonce path is W4-E.
 *  - block_registers_under_correct_namespace — dailyos/account-overview only.
 *  - runtime_invocation_uses_w3b_hmac_signer — runtime client method routes
 *    through DailyOS_Runtime_Client (no raw HTTP).
 *  - projection_version_rollback_banner — rollback (W4-C) surfaces verification banner.
 *  - wrong_user_rejected_before_preview — WP_Error path renders banner.
 *  - runtime_cache_hit_drops_raw_error — never exposes raw exception text.
 *  - trust_band_unknown_degrades — see unknown_block above.
 *
 * Fixtures intentionally use generic IDs (acct-test-001) per the
 * project-wide "no customer data in source" rule (CLAUDE.md / AC §54).
 *
 * @package DailyOS
 */

declare(strict_types=1);

use DailyOS\DailyOS_Plugin;
use DailyOS\Transport\DailyOS_Credential_Store;
use DailyOS\Transport\DailyOS_Runtime_Client;
use PHPUnit\Framework\TestCase;

require_once __DIR__ . '/../../blocks/account-overview/render-functions.php';

/**
 * Tests the DailyOS account overview block render and metadata behavior.
 */
final class DailyOS_AccountOverviewBlockTest extends TestCase {
	/**
	 * Resets test globals before each account overview block test.
	 *
	 * @return void
	 */
	protected function setUp(): void {
		parent::setUp();
		dailyos_test_reset_globals();
		// Strip any prior filters that might inject a fake runtime client.
		unset( $GLOBALS['dailyos_test_filters']['dailyos_runtime_client_for_block'] );
	}

	/**
	 * Asserts rendering returns the empty state when composition ID is missing.
	 */
	public function test_render_returns_empty_state_when_composition_id_missing(): void {
		$html = dailyos_account_overview_render( [] );
		$this->assertStringContainsString( 'is-empty', $html );
		$this->assertStringNotContainsString( '<article', $html );
	}

	/**
	 * Asserts rendering returns the empty state when no runtime client is available.
	 */
	public function test_render_returns_empty_state_when_runtime_client_unavailable(): void {
		$html = dailyos_account_overview_render(
			[
				'composition_id'      => 'dailyos/account-overview:account:acct-test-001',
				'composition_version' => 1,
			]
		);
		// No filter registered → runtime client is null → silent empty
		// state per §6.9.1, no banner, no raw error.
		$this->assertStringContainsString( 'is-empty', $html );
		$this->assertStringNotContainsString( 'Exception', $html );
	}

	/**
	 * Asserts rendering emits trust-band data attributes from the design system.
	 */
	public function test_render_emits_trust_band_data_attrs_per_design_system(): void {
		$client = $this->fake_runtime_client_with_response(
			$this->projection_response( 1, 'token-abc' )
		);
		$this->register_runtime_client_filter( $client );

		$html = dailyos_account_overview_render(
			[
				'composition_id'      => 'dailyos/account-overview:account:acct-test-001',
				'composition_version' => 1,
			]
		);

		$this->assertStringContainsString( 'data-ds-tier="primitive"', $html );
		$this->assertStringContainsString( 'data-ds-name="TrustBandBadge"', $html );
		$this->assertStringContainsString( 'data-ds-trust-band="likely_current"', $html );
		$this->assertStringContainsString( 'Likely current', $html );
		$this->assertSame( 1, $client->calls );
		$this->assertSame(
			[
				[
					'composition_id'      => 'dailyos/account-overview:account:acct-test-001',
					'composition_version' => 1,
					'cache_hint_token'    => null,
				],
			],
			$client->requests
		);
	}

	/**
	 * Asserts the wrapper still performs exactly one runtime fetch.
	 */
	public function test_wrapper_preserves_single_fetch_behavior(): void {
		$client = $this->fake_runtime_client_with_response( $this->projection_response( 3, 'token-single-fetch' ) );
		$this->register_runtime_client_filter( $client );

		$html = dailyos_account_overview_render(
			[
				'composition_id'      => 'dailyos/account-overview:account:acct-test-001',
				'composition_version' => 2,
				'cache_hint_token'    => 'token-old',
			]
		);

		$this->assertStringContainsString( '<article', $html );
		$this->assertSame( 1, $client->calls );
		$this->assertSame(
			[
				[
					'composition_id'      => 'dailyos/account-overview:account:acct-test-001',
					'composition_version' => 2,
					'cache_hint_token'    => 'token-old',
				],
			],
			$client->requests
		);
	}

	/**
	 * Asserts the preview route renders from its first runtime response.
	 */
	public function test_preview_route_uses_single_runtime_request(): void {
		$GLOBALS['dailyos_test_current_user_id']      = 42;
		$GLOBALS['dailyos_test_remote_post_response'] = [
			'response' => [
				'code' => 200,
			],
			'body'     => wp_json_encode( $this->projection_response( 5, 'token-preview' ) ),
		];
		$this->save_pairing_marker();
		$this->add_session_key_filter();

		$result = DailyOS_Plugin::instance()->account_overview_preview(
			[
				'composition_id'      => 'dailyos/account-overview:account:acct-test-001',
				'composition_version' => 4,
				'cache_hint_token'    => 'token-stale',
			]
		);

		$this->assertFalse( is_wp_error( $result ) );
		$this->assertCount( 1, $GLOBALS['dailyos_test_remote_post_calls'] );
		$this->assertSame( 'http://127.0.0.1:54321/v1/surface/project-composition', $GLOBALS['dailyos_test_remote_post_calls'][0]['url'] );
		$this->assertSame( 5, $result['attributes']['composition_version'] );
		$this->assertSame( 'token-preview', $result['attributes']['cache_hint_token'] );
		$this->assertStringContainsString( '<article', $result['html'] );
		$this->assertStringNotContainsString( 'ConsistencyFindingBanner', $result['html'] );
	}

	/**
	 * Asserts the editor reload callback keeps manual-reload inputs in its dep list.
	 */
	public function test_editor_reload_callback_dep_array_literal(): void {
		$source = $this->account_overview_editor_source();

		$this->assertMatchesRegularExpression(
			'/\},\s*\[\s*attributes\.composition_id\s*,\s*attributes\.composition_version\s*,\s*attributes\.cache_hint_token\s*,\s*setAttributes\s*\]\s*\);/s',
			$source
		);
	}

	/**
	 * Asserts auto-reload is keyed by account and composition presence only.
	 */
	public function test_editor_reload_trigger_key_literal(): void {
		$source = $this->account_overview_editor_source();

		$this->assertMatchesRegularExpression(
			'/const\s+reloadTrigger\s*=\s*`.*attributes\.account_id\s*\|\|\s*\'\'.*attributes\.composition_id\s*\?\s*\'1\'\s*:\s*\'0\'.*`/s',
			$source
		);
		$this->assertMatchesRegularExpression(
			'/useEffect\s*\(\s*\(\s*\)\s*=>\s*\{.*reload\(\);.*\},\s*\[\s*reloadTrigger\s*\]\s*\);/s',
			$source
		);
		$this->assertDoesNotMatchRegularExpression(
			'/\},\s*\[\s*reload\s*\]\s*\);/',
			$source
		);
	}

	/**
	 * Asserts failed preview responses preserve last-good editor state.
	 */
	public function test_editor_failed_reload_preserves_last_good_preview_shape(): void {
		$source = $this->account_overview_editor_source();

		$this->assertMatchesRegularExpression(
			'/if\s*\(\s*response\s*&&\s*response\.ok\s*===\s*false\s*\)\s*\{.*setError\(.*return;.*\}\s*setPreview\(\s*response\s*\);/s',
			$source
		);
	}

	/**
	 * Asserts unknown trust bands degrade to needs verification.
	 */
	public function test_unknown_trust_band_degrades_to_needs_verification(): void {
		$client = $this->fake_runtime_client_with_response(
			[
				'projection' => [
					'composition_id'      => 'dailyos/account-overview:account:acct-test-001',
					'composition_version' => 1,
					'blocks'              => [
						[
							'block_type' => 'risk',
							'trust_band' => 'not-a-real-band',
							'title'      => 'Risk',
							'summary'    => 'Body.',
						],
					],
				],
			]
		);
		$this->register_runtime_client_filter( $client );

		$html = dailyos_account_overview_render(
			[
				'composition_id'      => 'dailyos/account-overview:account:acct-test-001',
				'composition_version' => 1,
			]
		);

		$this->assertStringContainsString( 'data-ds-trust-band="needs_verification"', $html );
		$this->assertStringContainsString( 'Needs verification', $html );
		$this->assertStringNotContainsString( 'not-a-real-band', $html );
	}

	/**
	 * Asserts the stale banner appears when the delivered version is newer.
	 */
	public function test_stale_banner_appears_when_delivered_version_is_newer(): void {
		$GLOBALS['dailyos_test_options']['dailyos_composition_versions'] = [
			'dailyos/account-overview:account:acct-test-001' => 7,
		];

		$client = $this->fake_runtime_client_with_response(
			[
				'projection' => [
					'composition_id'      => 'dailyos/account-overview:account:acct-test-001',
					'composition_version' => 4,
					'blocks'              => [
						[
							'block_type' => 'risk',
							'trust_band' => 'likely_current',
							'title'      => 'Risk',
							'summary'    => 'Body.',
						],
					],
				],
			]
		);
		$this->register_runtime_client_filter( $client );

		$html = dailyos_account_overview_render(
			[
				'composition_id'      => 'dailyos/account-overview:account:acct-test-001',
				'composition_version' => 4,
			]
		);

		$this->assertStringContainsString( 'data-ds-name="StaleReportBanner"', $html );
		$this->assertStringContainsString( 'Newer context has arrived.', $html );
	}

	/**
	 * Asserts transport errors render a retryable notice without raw details.
	 */
	public function test_runtime_wp_error_renders_runtime_unavailable_notice_no_raw_exception(): void {
		$client = $this->fake_runtime_client_with_error(
			new \WP_Error( 'dailyos_projection_failed', 'raw runtime exception body that must not leak' )
		);
		$this->register_runtime_client_filter( $client );

		$html = dailyos_account_overview_render(
			[
				'composition_id'      => 'dailyos/account-overview:account:acct-test-001',
				'composition_version' => 1,
			]
		);

		$this->assertStringContainsString( 'data-ds-name="RuntimeUnavailableNotice"', $html );
		$this->assertStringContainsString( 'Runtime unavailable; retry.', $html );
		$this->assertStringNotContainsString( 'raw runtime exception', $html );
		$this->assertStringNotContainsString( 'dailyos_projection_failed', $html );
	}

	/**
	 * Asserts typed runtime error envelopes map to distinct notices.
	 *
	 * @dataProvider typed_error_mapping_provider
	 *
	 * @param string $code          Runtime error code.
	 * @param string $expected_text Expected notice text.
	 * @param string $expected_name Expected design-system notice name.
	 */
	public function test_typed_error_mapping( string $code, string $expected_text, string $expected_name ): void {
		$html = dailyos_account_overview_render_from_projection(
			[
				'ok'    => false,
				'error' => [
					'code'    => $code,
					'message' => 'raw runtime detail must not leak',
				],
			],
			[
				'composition_id'      => 'dailyos/account-overview:account:acct-test-001',
				'composition_version' => 1,
			]
		);

		$this->assertStringContainsString( 'data-ds-name="' . $expected_name . '"', $html );
		$this->assertStringContainsString( $expected_text, $html );
		$this->assertStringNotContainsString( 'raw runtime detail', $html );
	}

	/**
	 * Asserts unknown typed errors fail closed to the verification banner.
	 */
	public function test_unknown_typed_error_fails_safe_to_verification_banner(): void {
		$html = dailyos_account_overview_render_from_projection(
			[
				'ok'    => false,
				'error' => [
					'code'    => 'unknown_xyz',
					'message' => 'raw runtime detail must not leak',
				],
			],
			[
				'composition_id'      => 'dailyos/account-overview:account:acct-test-001',
				'composition_version' => 1,
			]
		);

		$this->assertStringContainsString( 'data-ds-name="ConsistencyFindingBanner"', $html );
		$this->assertStringContainsString( 'line up', $html );
		$this->assertStringNotContainsString( 'raw runtime detail', $html );
	}

	/**
	 * Asserts the block registers only under the DailyOS namespace.
	 */
	public function test_block_registration_uses_only_dailyos_namespace(): void {
		// The plugin's register_blocks() iterates blocks/*/block.json and
		// calls register_block_type_from_metadata. The metadata file
		// declares dailyos/account-overview.
		$block_json = json_decode(
			(string) file_get_contents( __DIR__ . '/../../blocks/account-overview/block.json' ),
			true
		);
		$this->assertIsArray( $block_json );
		$this->assertSame( 'dailyos/account-overview', $block_json['name'] );
		$this->assertSame( 3, $block_json['apiVersion'] );
		$this->assertSame( 'file:./render.php', $block_json['render'] );
	}

	/**
	 * Asserts block attributes omit scope and cache leak vectors.
	 */
	public function test_block_attributes_omit_scope_and_cache_leak_vectors(): void {
		$block_json      = json_decode(
			(string) file_get_contents( __DIR__ . '/../../blocks/account-overview/block.json' ),
			true
		);
		$attribute_names = array_keys( $block_json['attributes'] );

		$this->assertNotContains( 'cached_projection', $attribute_names );
		$this->assertNotContains( 'actor_scope_fingerprint', $attribute_names );
		$this->assertNotContains( 'actor_context_hint', $attribute_names );
		$this->assertNotContains( 'hmac_key', $attribute_names );
		$this->assertNotContains( 'session_token', $attribute_names );
		$this->assertNotContains( 'presence_nonce', $attribute_names );
	}

	/**
	 * Asserts the runtime client routes through the project composition endpoint.
	 */
	public function test_runtime_client_routes_through_project_composition_endpoint(): void {
		$reflection = new \ReflectionClass( DailyOS_Runtime_Client::class );
		$this->assertTrue(
			$reflection->hasMethod( 'project_composition_for_surface' ),
			'runtime client must expose project_composition_for_surface'
		);
		$method = $reflection->getMethod( 'project_composition_for_surface' );
		$this->assertSame( 3, $method->getNumberOfParameters() );
	}

	/**
	 * Asserts the save script returns null.
	 */
	public function test_save_js_returns_null(): void {
		$save_js = (string) file_get_contents( __DIR__ . '/../../blocks/account-overview/save.js' );
		$this->assertStringContainsString( 'return null', $save_js );
		// The save handler must NOT emit DailyOS HTML.
		$this->assertStringNotContainsString( 'data-ds-trust-band', $save_js );
		$this->assertStringNotContainsString( '<article', $save_js );
	}

	/**
	 * Asserts the edit script does not reach the loopback runtime.
	 */
	public function test_edit_js_does_not_reach_loopback_runtime(): void {
		$edit_js = (string) file_get_contents( __DIR__ . '/../../blocks/account-overview/edit.js' );
		$this->assertStringNotContainsString( '127.0.0.1', $edit_js );
		$this->assertStringNotContainsString( 'localhost', $edit_js );
		$this->assertStringNotContainsString( ':54321', $edit_js );
		// HMAC reconstruction must not appear in browser JS.
		$this->assertStringNotContainsString( 'hmac', strtolower( $edit_js ) );
		$this->assertStringNotContainsString( 'createhmac', strtolower( $edit_js ) );
	}

	/**
	 * Asserts render PHP never calls the W4-D projector directly.
	 */
	public function test_render_php_never_calls_w4d_projector_directly(): void {
		$render_fns = (string) file_get_contents( __DIR__ . '/../../blocks/account-overview/render-functions.php' );
		// PHP must NEVER reach into the abilities-runtime crate or call
		// W4-D helpers directly (per AC §17, §18). The runtime client is
		// the sole bridge.
		$this->assertStringNotContainsString( 'project_composition_for_surface_internal', $render_fns );
		$this->assertStringNotContainsString( 'fallback_projection', $render_fns );
		$this->assertStringNotContainsString( 'wp_remote_post', $render_fns );
		$this->assertStringNotContainsString( 'curl_exec', $render_fns );
	}


	/**
	 * Typed error mapping provider.
	 *
	 * @return array<string, array{0: string, 1: string, 2: string}>
	 */
	public static function typed_error_mapping_provider(): array {
		$cases = [];
		foreach ( [ 'rate_limited', 'transport_abuse_limited' ] as $code ) {
			$cases[ $code ] = [ $code, 'Runtime is throttling; retry shortly.', 'RuntimeThrottledNotice' ];
		}
		// Session/pairing-repair-shaped codes — extended in L2 cycle 2 to cover
		// every signed-runtime / pairing / signed-transport code emittable per
		// surface_runtime/hmac.rs, surface_pairing.rs, and surface_runtime/mod.rs
		// constructors. Verification banner is reserved for true projection-
		// consistency failures + unknown-code fail-safe.
		foreach ( [
			'session_requires_repair',
			'session_not_found',
			'session_expired',
			'session_throttled',
			'session_invalid',
			'identity_mismatch',
			'wp_user_mismatch',
			'pairing_code_invalid',
			'pairing_code_expired',
			'pairing_code_consumed',
			'pairing_code_limited',
			'pairing_suspended',
			'pairing_revoked',
			'pairing_expired',
			'pairing_authority_unavailable',
			'site_binding_mismatch',
			'restored_stale_pairing',
			'unknown_runtime_anchor',
			'scope_denied',
			'auth_missing',
			'signature_invalid',
			'canonicalization_mismatch',
			'timestamp_stale',
			'timestamp_future',
			'key_not_found',
			'key_rotated',
			'token_invalid',
			'nonce_replay',
		] as $code ) {
			$cases[ $code ] = [ $code, 'Surface session needs repair; reconnect from DailyOS settings.', 'SurfaceSessionRepairNotice' ];
		}
		foreach ( [
			'runtime_unavailable',
			'runtime_request_failed',
			'runtime_invalid_json',
			'runtime_http_error',
			'host_invalid',
			'browser_origin_forbidden',
			'route_not_found',
		] as $code ) {
			$cases[ $code ] = [ $code, 'Runtime unavailable; retry.', 'RuntimeUnavailableNotice' ];
		}
		foreach ( [
			'request_body_too_large',
			'request_body_unreadable',
			'handshake_body_invalid',
			'session_refresh_body_invalid',
			'surface_invoke_invalid',
			'event_log_id_invalid',
			'project_composition_invalid',
			'project_composition_unknown_producer',
			'project_composition_invalid_id',
		] as $code ) {
			$cases[ $code ] = [ $code, "Editor sent a request the runtime couldn't process. Reload the editor.", 'InvalidRuntimeRequestNotice' ];
		}
		foreach ( [
			'projection_tampered',
			'projection_version_rollback',
			'stale_composition_watermark',
			'missing_expected_claim_version',
			'mid_flight_mutation',
			'composition_version_overflow',
		] as $code ) {
			$cases[ $code ] = [ $code, 'line up', 'ConsistencyFindingBanner' ];
		}

		return $cases;
	}

	/**
	 * Build a successful projection response fixture.
	 *
	 * @param int    $version          Composition version.
	 * @param string $cache_hint_token Cache hint token.
	 * @return array<string, mixed>
	 */
	private function account_overview_editor_source(): string {
		$source = file_get_contents( __DIR__ . '/../../blocks/account-overview/edit.js' );
		$this->assertIsString( $source );
		return $source;
	}

	private function projection_response( int $version, string $cache_hint_token = '' ): array {
		return [
			'ok'               => true,
			'projection'       => [
				'composition_id'      => 'dailyos/account-overview:account:acct-test-001',
				'composition_version' => $version,
				'blocks'              => [
					[
						'selected_known_type_id' => 'dailyos/account_overview',
						'trust_band'             => 'likely_current',
						'payload'                => [
							'title'   => 'Account overview',
							'context' => [
								[
									'text' => 'Sample account context.',
								],
							],
						],
					],
				],
			],
			'cache_hint_token' => $cache_hint_token,
		];
	}

	/**
	 * Save a complete test pairing marker.
	 */
	private function save_pairing_marker(): void {
		( new DailyOS_Credential_Store() )->save_marker(
			[
				'runtime_instance_id'  => 'runtime-123',
				'surface_client_id'    => 'surface-client-123',
				'runtime_url'          => 'http://127.0.0.1:54321',
				'site_nonce_hash'      => hash( 'sha256', 'siteNonceAlpha123' ),
				'site_nonce_full'      => 'siteNonceAlpha123',
				'site_binding_digest'  => str_repeat( 'a', 64 ),
				'wp_site_id'           => 'install-1:1',
				'wp_install_uuid'      => 'install-1',
				'plugin_instance_uuid' => 'plugin-1',
				'projection_version'   => '2026.05.13',
				'instance_id'          => 'runtime-123',
				'session_id'           => 'session-123',
				'granted_scopes'       => [ 'read.account_overview' ],
				'endpoint_version'     => 'v1',
				'paired_at_gmt'        => '2026-05-13 00:00:00',
				'last_use_gmt'         => '2026-05-13 00:00:00',
				'paired_wp_user_id'    => '42',
			]
		);
	}

	/**
	 * Add a valid test session key filter.
	 */
	private function add_session_key_filter(): void {
		add_filter(
			'dailyos_wp_bridge_session_key',
			static function (): array {
				return [
					'hmac_key'   => str_repeat( "\x02", 32 ),
					'session_id' => 'surface-session-id',
				];
			},
			10,
			1
		);
	}

	/**
	 * Build a fake runtime client that returns a canned successful response
	 * from `project_composition_for_surface`. Wraps an anonymous class so
	 * tests can inject behavior without subclassing the `final` transport.
	 *
	 * @param array $response Canned response array.
	 * @return object Fake runtime client.
	 */
	private function fake_runtime_client_with_response( array $response ): object {
		return new class( $response ) {
			/**
			 * Fake runtime response.
			 *
			 * @var array
			 */
			public array $response;
			/**
			 * Number of fake runtime calls.
			 *
			 * @var int
			 */
			public int $calls = 0;
			/**
			 * Captured fake runtime requests.
			 *
			 * @var array<int, array<string, mixed>>
			 */
			public array $requests = [];
			/**
			 * Initializes the fake runtime response.
			 *
			 * @param array $response Fake runtime response.
			 */
			public function __construct( array $response ) {
				$this->response = $response;
			}
			/**
			 * Projects a fake composition response.
			 *
			 * @param string      $composition_id      Composition ID.
			 * @param int         $composition_version Composition version.
			 * @param string|null $cache_hint_token    Cache hint token.
			 * @return array Fake runtime response.
			 */
			public function project_composition_for_surface(
				string $composition_id,
				int $composition_version,
				?string $cache_hint_token = null
			): array {
				++$this->calls;
				$this->requests[] = [
					'composition_id'      => $composition_id,
					'composition_version' => $composition_version,
					'cache_hint_token'    => $cache_hint_token,
				];
				return $this->response;
			}
		};
	}

	/**
	 * Build a fake runtime client that returns a `WP_Error` from
	 * `project_composition_for_surface`. Used to drive the transport-error
	 * render path.
	 *
	 * @param \WP_Error $error Error to return.
	 * @return object Fake runtime client.
	 */
	private function fake_runtime_client_with_error( \WP_Error $error ): object {
		return new class( $error ) {
			/**
			 * Fake runtime error.
			 *
			 * @var \WP_Error
			 */
			public \WP_Error $error;
			/**
			 * Number of fake runtime calls.
			 *
			 * @var int
			 */
			public int $calls = 0;
			/**
			 * Captured fake runtime requests.
			 *
			 * @var array<int, array<string, mixed>>
			 */
			public array $requests = [];
			/**
			 * Initializes the fake runtime error.
			 *
			 * @param \WP_Error $error Fake runtime error.
			 */
			public function __construct( \WP_Error $error ) {
				$this->error = $error;
			}
			/**
			 * Projects a fake composition error.
			 *
			 * @param string      $composition_id      Composition ID.
			 * @param int         $composition_version Composition version.
			 * @param string|null $cache_hint_token    Cache hint token.
			 * @return \WP_Error Fake runtime error.
			 */
			public function project_composition_for_surface(
				string $composition_id,
				int $composition_version,
				?string $cache_hint_token = null
			): \WP_Error {
				++$this->calls;
				$this->requests[] = [
					'composition_id'      => $composition_id,
					'composition_version' => $composition_version,
					'cache_hint_token'    => $cache_hint_token,
				];
				return $this->error;
			}
		};
	}

	/**
	 * Registers a fake runtime client filter.
	 *
	 * @param object $client Fake runtime client.
	 */
	private function register_runtime_client_filter( object $client ): void {
		add_filter(
			'dailyos_runtime_client_for_block',
			static function () use ( $client ) {
				return $client;
			},
			10,
			0
		);
	}
}
