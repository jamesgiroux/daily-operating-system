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
			$headers['X-DailyOS-Nonce'],
			$headers['X-DailyOS-Timestamp']
		);

		$this->assertSame( $expected_signature, $headers['X-DailyOS-Signature'] );
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
}
