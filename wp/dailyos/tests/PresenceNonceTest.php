<?php
/**
 * DailyOS presence nonce WordPress bridge tests.
 *
 * @package DailyOS
 */

declare(strict_types=1);

use DailyOS\DailyOS_Plugin;
use DailyOS\Transport\DailyOS_Credential_Store;
use DailyOS\Transport\DailyOS_Hmac_Signer;
use DailyOS\Transport\DailyOS_Runtime_Client;
use PHPUnit\Framework\TestCase;

/**
 * Verifies the W4-E WordPress nonce bridge never persists nonce tokens.
 */
final class DailyOS_PresenceNonceTest extends TestCase {
	/**
	 * Reset WordPress stubs before each test.
	 */
	protected function setUp(): void {
		parent::setUp();

		dailyos_test_reset_globals();
		( new DailyOS_Credential_Store() )->register_session_key_filter_safeguard();
	}

	/**
	 * The REST bridge exposes nonce issue through WordPress, not direct browser runtime access.
	 */
	public function test_rest_route_registers_nonce_issue_route(): void {
		DailyOS_Plugin::instance()->register_rest_routes();

		$this->assertArrayHasKey( 'dailyos/v1/nonce', $GLOBALS['dailyos_test_rest_routes'] );
		$this->assertSame( 'POST', $GLOBALS['dailyos_test_rest_routes']['dailyos/v1/nonce']['methods'] );
	}

	/**
	 * The WordPress route builds the mandatory nonce tuple with current session identity.
	 */
	public function test_issue_presence_nonce_builds_runtime_payload_from_current_user_and_session(): void {
		$GLOBALS['dailyos_test_is_user_logged_in'] = true;
		$GLOBALS['dailyos_test_current_user_id']   = 42;
		$this->save_marker();
		$this->add_session_key_filter();

		$GLOBALS['dailyos_test_remote_post_response'] = [
			'response' => [
				'code' => 200,
			],
			'body'     => '{"ok":true,"presence_nonce":"nonce-token"}',
		];

		$result = DailyOS_Plugin::instance()->issue_presence_nonce( $this->nonce_request() );

		$this->assertTrue( $result['ok'] );
		$this->assertCount( 1, $GLOBALS['dailyos_test_remote_post_calls'] );

		$call    = $GLOBALS['dailyos_test_remote_post_calls'][0];
		$payload = json_decode( $call['args']['body'], true );

		$this->assertSame( 'http://127.0.0.1:54321/v1/surface/nonce/issue', $call['url'] );
		$this->assertSame( 'surface-session-id', $payload['session_id'] );
		$this->assertSame( 42, $payload['wp_user_id'] );
		$this->assertSame( 7, $payload['claim_version'] );
		$this->assertSame( 17, $payload['composition_version'] );
		$this->assertSame( 'request-1', $payload['request_id'] );
	}

	/**
	 * claim_version must be a JSON integer, not a coercible string.
	 */
	public function test_issue_presence_nonce_rejects_string_claim_version(): void {
		$GLOBALS['dailyos_test_is_user_logged_in'] = true;
		$GLOBALS['dailyos_test_current_user_id']   = 42;
		$this->save_marker();
		$this->add_session_key_filter();

		$request = $this->nonce_request(
			[
				'claim_version' => '7',
			]
		);

		$result = DailyOS_Plugin::instance()->issue_presence_nonce( $request );

		$this->assertTrue( is_wp_error( $result ) );
		$this->assertSame( 'malformed_claim_version', $result->get_error_code() );
	}

	/**
	 * Ephemeral nonce attributes are removed from parsed block attrs before save.
	 */
	public function test_block_serialization_strips_presence_nonce_attributes(): void {
		$GLOBALS['dailyos_test_parse_blocks_result'] = [
			[
				'blockName'   => 'dailyos/surface-claim',
				'attrs'       => [
					'claim_id'       => 'claim-1',
					'presence_nonce' => 'nonce-token',
					'nested'         => [
						'dailyosPresenceNonce' => 'nested-token',
						'kept'                 => true,
					],
				],
				'innerBlocks' => [],
			],
		];

		DailyOS_Plugin::strip_presence_nonces_from_content( '<!-- wp:dailyos/surface-claim /-->' );

		$blocks = $GLOBALS['dailyos_test_serialized_blocks'];
		$this->assertIsArray( $blocks );
		$this->assertArrayNotHasKey( 'presence_nonce', $blocks[0]['attrs'] );
		$this->assertArrayNotHasKey( 'dailyosPresenceNonce', $blocks[0]['attrs']['nested'] );
		$this->assertTrue( $blocks[0]['attrs']['nested']['kept'] );
	}

	/**
	 * Runtime client keeps nonce rejection bodies intact for callers.
	 */
	public function test_runtime_client_preserves_nonce_rejection_shape(): void {
		$GLOBALS['dailyos_test_current_user_id'] = 42;
		$this->save_marker();
		$this->add_session_key_filter();

		$GLOBALS['dailyos_test_remote_post_response'] = [
			'response' => [
				'code' => 409,
			],
			'body'     => '{"ok":false,"error":"presence_nonce_rejected","reason":"claim_version_stale"}',
		];

		$client = new DailyOS_Runtime_Client( new DailyOS_Credential_Store(), new DailyOS_Hmac_Signer() );
		$result = $client->verify_nonce(
			[
				'presence_nonce'      => 'nonce-token',
				'session_id'          => 'surface-session-id',
				'wp_user_id'          => 42,
				'claim_id'            => 'claim-1',
				'field_path'          => 'claims[0].summary',
				'action'              => 'correct',
				'claim_version'       => 7,
				'composition_id'      => 'composition-1',
				'composition_version' => 17,
			]
		);

		$this->assertFalse( $result['ok'] );
		$this->assertSame( 'presence_nonce_rejected', $result['error'] );
		$this->assertSame( 'claim_version_stale', $result['reason'] );
	}

	/**
	 * Build a nonce issue request double.
	 *
	 * @param array<string, mixed> $overrides Request field overrides.
	 */
	private function nonce_request( array $overrides = [] ): object {
		$params = array_merge(
			[
				'claim_id'            => 'claim-1',
				'field_path'          => 'claims[0].summary',
				'action'              => 'correct',
				'claim_version'       => 7,
				'composition_id'      => 'composition-1',
				'composition_version' => 17,
				'request_id'          => 'request-1',
				'post_id'             => 123,
			],
			$overrides
		);

		return new class( $params ) {
			/**
			 * @param array<string, mixed> $params Request params.
			 */
			public function __construct( private array $params ) {}

			/**
			 * @return array<string, mixed> Request params.
			 */
			public function get_json_params(): array {
				return $this->params;
			}
		};
	}

	/**
	 * Add a valid session key filter.
	 */
	private function add_session_key_filter(): void {
		add_filter(
			'dailyos_wp_bridge_session_key',
			static function (): array {
				return [
					'hmac_key'   => str_repeat( "", 32 ),
					'session_id' => 'surface-session-id',
				];
			},
			10,
			1
		);
	}

	/**
	 * Save a complete pairing marker.
	 */
	private function save_marker(): void {
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
				'granted_scopes'       => [ 'submit.feedback' ],
				'endpoint_version'     => 'v1',
				'paired_wp_user_id'    => '42',
			]
		);
	}
}
