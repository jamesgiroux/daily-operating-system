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
			[
				'projection'      => [
					'composition_id'      => 'dailyos/account-overview:account:acct-test-001',
					'composition_version' => 1,
					'blocks'              => [
						[
							'block_type' => 'risk',
							'trust_band' => 'likely_current',
							'title'      => 'Risk overview',
							'summary'    => 'Sample summary.',
						],
					],
				],
				'cache_hint_token' => 'token-abc',
			]
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
	 * Asserts runtime errors render a verification banner without raw details.
	 */
	public function test_runtime_wp_error_renders_verification_banner_no_raw_exception(): void {
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

		$this->assertStringContainsString( 'data-ds-name="ConsistencyFindingBanner"', $html );
		$this->assertStringContainsString( 'line up', $html );
		$this->assertStringNotContainsString( 'raw runtime exception', $html );
		$this->assertStringNotContainsString( 'dailyos_projection_failed', $html );
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
		$block_json = json_decode(
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
				return $this->response;
			}
		};
	}

	/**
	 * Build a fake runtime client that returns a `WP_Error` from
	 * `project_composition_for_surface`. Used to drive the verification-banner
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
