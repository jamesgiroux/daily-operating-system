<?php
/**
 * DailyOS runtime client transport tests.
 *
 * @package DailyOS
 */

declare(strict_types=1);

use DailyOS\Transport\DailyOS_Credential_Store;
use DailyOS\Transport\DailyOS_Hmac_Key;
use DailyOS\Transport\DailyOS_Hmac_Signer;
use DailyOS\Transport\DailyOS_Runtime_Client;
use PHPUnit\Framework\TestCase;

/**
 * Verifies runtime client HTTP arguments preserve transport caveats.
 */
final class DailyOS_RuntimeClientTest extends TestCase {
	/**
	 * Reset WordPress stubs before each runtime-client test.
	 */
	protected function setUp(): void {
		parent::setUp();

		dailyos_test_reset_globals();
		( new DailyOS_Credential_Store() )->register_session_key_filter_safeguard();
	}

	/**
	 * Signed requests send string bodies and sign the exact transmitted bytes.
	 */
	public function test_signed_post_uses_string_body_headers_and_byte_exact_signature(): void {
		$hmac_key_bytes = str_repeat( "\x02", 32 );

		$GLOBALS['dailyos_test_current_user_id'] = 42;
		$this->save_marker();
		$this->add_session_key_filter();

		$GLOBALS['dailyos_test_remote_post_response'] = [
			'response' => [
				'code' => 200,
			],
			'body'     => '{"ok":true}',
		];

		$client = new DailyOS_Runtime_Client( new DailyOS_Credential_Store(), new DailyOS_Hmac_Signer() );
		$client->invoke_ability( 'briefing.daily', [ 'depth' => 'standard' ], [ 'dailyos.read' ] );

		$this->assertCount( 1, $GLOBALS['dailyos_test_remote_post_calls'] );

		$call    = $GLOBALS['dailyos_test_remote_post_calls'][0];
		$args    = $call['args'];
		$headers = $args['headers'];

		$this->assertSame( 'string', gettype( $args['body'] ) );
		$this->assertSame( 'application/json', $headers['Content-Type'] );
		$this->assertSame( 0, $args['redirection'] );

		$expected_signature = ( new DailyOS_Hmac_Signer() )->sign_request(
			new DailyOS_Hmac_Key( $hmac_key_bytes ),
			'POST',
			'/v1/surface/invoke',
			'application/json',
			$args['body'],
			$this->canonical_identity(),
			$headers['X-DailyOS-Nonce'],
			$headers['X-DailyOS-Timestamp']
		);

		$this->assertSame( $expected_signature, $headers['X-DailyOS-Signature'] );
		$this->assertSame( 'http://127.0.0.1:54321/v1/surface/invoke', $call['url'] );
		$this->assertSame( 'surface-client-123', $headers['X-DailyOS-SurfaceClient'] );
		$this->assertSame( str_repeat( 'a', 64 ), $headers['X-DailyOS-Site-Binding-Digest'] );
		$this->assertSame( 'siteNonceAlpha123', $headers['X-DailyOS-Site-Nonce'] );
		$this->assertSame( '42', $headers['X-DailyOS-WP-User-Id'] );
	}

	/**
	 * Signed requests refuse to guess a default runtime URL when no marker exists.
	 */
	public function test_signed_post_returns_not_paired_without_marker(): void {
		$this->add_session_key_filter();

		$client = new DailyOS_Runtime_Client( new DailyOS_Credential_Store(), new DailyOS_Hmac_Signer() );
		$result = $client->invoke_ability( 'briefing.daily', [], [] );

		$this->assertTrue( is_wp_error( $result ) );
		$this->assertSame( 'dailyos_not_paired', $result->get_error_code() );
		$this->assertSame( [], $GLOBALS['dailyos_test_remote_post_calls'] );
	}

	/**
	 * A manage-options runtime URL filter can override the marker with loopback only.
	 */
	public function test_runtime_url_filter_accepts_loopback_override(): void {
		$this->save_marker();
		$this->add_session_key_filter();

		add_filter(
			'dailyos_wp_bridge_runtime_url',
			static function (): string {
				return 'http://127.0.0.1:54322';
			},
			10,
			1
		);

		$client = new DailyOS_Runtime_Client( new DailyOS_Credential_Store(), new DailyOS_Hmac_Signer() );
		$client->invoke_ability( 'briefing.daily', [], [] );

		$this->assertSame( 'http://127.0.0.1:54322/v1/surface/invoke', $GLOBALS['dailyos_test_remote_post_calls'][0]['url'] );
	}

	/**
	 * Runtime URL filters reject non-loopback hosts and fall back to the marker.
	 *
	 * @dataProvider invalid_runtime_url_provider
	 *
	 * @param string $runtime_url Runtime URL override candidate.
	 */
	public function test_runtime_url_filter_rejects_non_loopback_override( string $runtime_url ): void {
		$this->save_marker();
		$this->add_session_key_filter();

		add_filter(
			'dailyos_wp_bridge_runtime_url',
			static function () use ( $runtime_url ): string {
				return $runtime_url;
			},
			10,
			1
		);

		$client = new DailyOS_Runtime_Client( new DailyOS_Credential_Store(), new DailyOS_Hmac_Signer() );
		$client->invoke_ability( 'briefing.daily', [], [] );

		$this->assertSame( 'http://127.0.0.1:54321/v1/surface/invoke', $GLOBALS['dailyos_test_remote_post_calls'][0]['url'] );
	}

	/**
	 * Runtime client construction does not register HTTP mutation hooks.
	 */
	public function test_runtime_client_constructor_does_not_register_http_request_args_hooks(): void {
		$this->assertArrayNotHasKey( 'http_request_args', $GLOBALS['dailyos_test_filters'] );
		$this->assertArrayNotHasKey( 'http_request_args', $GLOBALS['dailyos_test_actions'] );

		new DailyOS_Runtime_Client( new DailyOS_Credential_Store(), new DailyOS_Hmac_Signer() );

		$this->assertArrayNotHasKey( 'http_request_args', $GLOBALS['dailyos_test_filters'] );
		$this->assertArrayNotHasKey( 'http_request_args', $GLOBALS['dailyos_test_actions'] );
	}

	/**
	 * Pairing handshake preserves unified marker fields from the runtime.
	 */
	public function test_handshake_response_preserves_unified_marker_fields(): void {
		$GLOBALS['dailyos_test_remote_post_response'] = [
			'response' => [
				'code' => 200,
			],
			'body'     => wp_json_encode(
				[
					'runtime_instance_id' => 'runtime-123',
					'runtime_url'         => 'http://127.0.0.1:54321',
					'site_binding_digest' => str_repeat( 'b', 64 ),
					'site_nonce'          => 'siteNonceAlpha123',
					'projection_version'  => '2026.05.13',
					'session_id'          => 'session-123',
					'granted_scopes'      => [ 'read.account_overview' ],
					'endpoint_version'    => 'v1',
				]
			),
		];

		$client = new DailyOS_Runtime_Client( new DailyOS_Credential_Store(), new DailyOS_Hmac_Signer() );
		$result = $client->handshake(
			'dailyos://pair?port=54321&code=pair-code',
			[
				'wp_site_id'           => 'install-1:1',
				'wp_install_uuid'      => 'install-1',
				'plugin_instance_uuid' => 'plugin-1',
			]
		);

		$this->assertTrue( $result['ok'] );
		$this->assertSame( 'runtime-123', $result['runtime_instance_id'] );
		$this->assertSame( 'http://127.0.0.1:54321', $result['runtime_url'] );
		$this->assertSame( hash( 'sha256', 'siteNonceAlpha123' ), $result['site_nonce_hash'] );
		$this->assertSame( 'siteNonceAlpha123', $result['site_nonce_full'] );
		$this->assertSame( str_repeat( 'b', 64 ), $result['site_binding_digest'] );
		$this->assertSame( 'install-1:1', $result['wp_site_id'] );
		$this->assertSame( '2026.05.13', $result['projection_version'] );
	}

	/**
	 * Invalid runtime URL provider.
	 *
	 * @return array<string, array{0: string}>
	 */
	public static function invalid_runtime_url_provider(): array {
		return [
			'any-address' => [ 'http://0.0.0.0:54321' ],
			'hostname'    => [ 'http://attacker.com' ],
		];
	}

	/**
	 * Add a valid session key filter.
	 */
	private function add_session_key_filter(): void {
		add_filter(
			'dailyos_wp_bridge_session_key',
			static function (): array {
				return [
					'bearer'     => 'surface-bearer-token',
					'hmac_key'   => str_repeat( "\x02", 32 ),
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
				'granted_scopes'       => [ 'read.account_overview' ],
				'endpoint_version'     => 'v1',
				'paired_at_gmt'        => '2026-05-13 00:00:00',
				'last_use_gmt'         => '2026-05-13 00:00:00',
			]
		);
	}

	/**
	 * Expected canonical identity for the saved marker.
	 *
	 * @return array<string, string>
	 */
	private function canonical_identity(): array {
		return [
			'site_binding_digest'  => str_repeat( 'a', 64 ),
			'site_nonce'           => 'siteNonceAlpha123',
			'wp_user_id'           => '42',
			'wp_site_id'           => 'install-1:1',
			'home_url'             => 'https://example.test',
			'site_url'             => 'https://example.test',
			'wp_install_uuid'      => 'install-1',
			'plugin_instance_uuid' => 'plugin-1',
			'multisite_blog_id'    => '',
		];
	}
}
