<?php
/**
 * REST endpoint registration + permission_callback tests for the W4 feedback path.
 *
 * @package DailyOS
 */

declare(strict_types=1);

use DailyOS\DailyOS_Plugin;
use DailyOS\Transport\DailyOS_Credential_Store;
use PHPUnit\Framework\TestCase;

/**
 * Verifies /dailyos/v1/nonce/verify registers correctly and the permission_callback
 * is the literal can_issue_presence_nonce per packet F §16 #7.
 */
final class DailyOS_SurfaceNonceFeedbackEndpointTest extends TestCase {
	protected function setUp(): void {
		parent::setUp();
		dailyos_test_reset_globals();
		( new DailyOS_Credential_Store() )->register_session_key_filter_safeguard();
	}

	public function test_rest_route_registers_nonce_verify_route(): void {
		DailyOS_Plugin::instance()->register_rest_routes();

		$this->assertArrayHasKey( 'dailyos/v1/nonce/verify', $GLOBALS['dailyos_test_rest_routes'] );
		$this->assertSame( 'POST', $GLOBALS['dailyos_test_rest_routes']['dailyos/v1/nonce/verify']['methods'] );
	}

	public function test_verify_route_reuses_can_issue_presence_nonce_callback(): void {
		DailyOS_Plugin::instance()->register_rest_routes();

		$verify_callback = $GLOBALS['dailyos_test_rest_routes']['dailyos/v1/nonce/verify']['permission_callback'];
		$issue_callback  = $GLOBALS['dailyos_test_rest_routes']['dailyos/v1/nonce']['permission_callback'];

		$this->assertSame( $issue_callback, $verify_callback, 'Verify route MUST reuse the literal can_issue_presence_nonce callback (packet F §16 #7).' );
	}

	public function test_verify_rejects_unauthenticated_user(): void {
		$GLOBALS['dailyos_test_is_user_logged_in'] = false;

		$result = DailyOS_Plugin::instance()->can_issue_presence_nonce( $this->verify_request() );

		$this->assertTrue( is_wp_error( $result ) );
		$this->assertSame( 'dailyos_nonce_unauthenticated', $result->get_error_code() );
	}

	public function test_verify_rejects_when_not_paired(): void {
		$GLOBALS['dailyos_test_is_user_logged_in'] = true;
		$GLOBALS['dailyos_test_current_user_id']   = 42;
		$GLOBALS['dailyos_test_current_user_can']  = true;

		$result = DailyOS_Plugin::instance()->verify_presence_nonce( $this->verify_request() );

		$this->assertTrue( is_wp_error( $result ) );
		$this->assertSame( 'dailyos_not_paired', $result->get_error_code() );
	}

	public function test_verify_builds_runtime_payload_with_server_derived_wp_user_id(): void {
		$GLOBALS['dailyos_test_is_user_logged_in'] = true;
		$GLOBALS['dailyos_test_current_user_id']   = 42;
		$this->save_marker();
		$this->add_session_key_filter();

		$GLOBALS['dailyos_test_remote_post_response'] = [
			'response' => [ 'code' => 200 ],
			'body'     => '{"ok":true,"feedback_id":"feedback-1","new_verification_state":"superseded"}',
		];

		$result = DailyOS_Plugin::instance()->verify_presence_nonce(
			$this->verify_request( [
				'wp_user_id' => 999,
			] )
		);

		$this->assertTrue( $result['ok'] );
		$this->assertCount( 1, $GLOBALS['dailyos_test_remote_post_calls'] );

		$call    = $GLOBALS['dailyos_test_remote_post_calls'][0];
		$payload = json_decode( $call['args']['body'], true );

		$this->assertSame( 'http://127.0.0.1:54321/v1/surface/nonce/verify', $call['url'] );
		$this->assertSame( 42, $payload['wp_user_id'], 'wp_user_id MUST be server-derived, never trusted from request body (packet F decision §6 #8).' );
		$this->assertSame( 'surface-session-id', $payload['session_id'] );
		$this->assertSame( 'nonce-digest-token', $payload['nonce_digest'] );
	}

	private function verify_request( array $overrides = [] ): object {
		$params = array_merge(
			[
				'nonce_digest' => 'nonce-digest-token',
				'post_id'      => 123,
			],
			$overrides
		);

		return new class( $params ) {
			public function __construct( private array $params ) {}
			public function get_json_params(): array {
				return $this->params;
			}
		};
	}

	private function add_session_key_filter(): void {
		add_filter(
			'dailyos_wp_bridge_session_key',
			static function (): array {
				return [
					'hmac_key'   => str_repeat( chr( 0 ), 32 ),
					'session_id' => 'surface-session-id',
				];
			},
			10,
			1
		);
	}

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
