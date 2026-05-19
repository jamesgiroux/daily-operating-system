<?php
/**
 * payload_json sensitivity=User redaction tests.
 *
 * Per packet F §5.5 + §6 decision #5: user-authored payload_json fields
 * (`corrected_text`, `corrected_to`, `surface`, `invocation_id`) carry
 * sensitivity=User and MUST NEVER leak through WP block-render channels
 * to a non-originating actor.
 *
 * The substrate enforces redaction at the projection layer. This test
 * asserts the WP-side surface NEVER echoes user-authored content from
 * runtime responses into rendered HTML.
 *
 * @package DailyOS
 */

declare(strict_types=1);

use DailyOS\DailyOS_Plugin;
use DailyOS\Transport\DailyOS_Credential_Store;
use PHPUnit\Framework\TestCase;

final class DailyOS_FeedbackPayloadRedactionTest extends TestCase {
	protected function setUp(): void {
		parent::setUp();
		dailyos_test_reset_globals();
		( new DailyOS_Credential_Store() )->register_session_key_filter_safeguard();

		$GLOBALS['dailyos_test_is_user_logged_in'] = true;
		$GLOBALS['dailyos_test_current_user_id']   = 42;
		$this->save_marker();
		$this->add_session_key_filter();
	}

	public function test_verify_response_does_not_echo_payload_json_back_to_caller(): void {
		$GLOBALS['dailyos_test_remote_post_response'] = [
			'response' => [ 'code' => 200 ],
			'body'     => json_encode( [
				'ok'                       => true,
				'feedback_id'              => 'feedback-1',
				'new_verification_state'   => 'superseded',
				'payload_json'             => [ 'corrected_text' => 'leaked user text' ],
			] ),
		];

		$result = DailyOS_Plugin::instance()->verify_presence_nonce(
			$this->verify_request()
		);

		$this->assertFalse(
			isset( $result['payload_json'] ),
			'WP verify response MUST NOT echo runtime payload_json back to a non-originating caller (sensitivity=User leak guard, packet F §6 #5).'
		);

		$serialized = json_encode( $result );
		$this->assertStringNotContainsString( 'leaked user text', $serialized, 'User-authored corrected_text MUST NOT appear anywhere in the WP-surface response.' );
	}

	public function test_issue_response_does_not_echo_payload_json_back_to_caller(): void {
		$GLOBALS['dailyos_test_remote_post_response'] = [
			'response' => [ 'code' => 200 ],
			'body'     => json_encode( [
				'ok'             => true,
				'presence_nonce' => 'nonce-token',
				'nonce_digest'   => 'nonce-digest-token',
				'payload_json'   => [ 'corrected_text' => 'leaked echo' ],
			] ),
		];

		$result = DailyOS_Plugin::instance()->issue_presence_nonce(
			$this->nonce_request( [
				'action'       => 'needs_nuance',
				'payload_json' => [ 'corrected_text' => 'original user text' ],
			] )
		);

		$serialized = json_encode( $result );
		$this->assertStringNotContainsString( 'leaked echo', $serialized );
		$this->assertStringNotContainsString( 'original user text', $serialized, 'Issue response MUST NOT echo the request payload_json back into the response body.' );
	}

	private function nonce_request( array $overrides = [] ): object {
		$params = array_merge(
			[
				'claim_id'            => 'claim-test-001',
				'field_path'          => 'claims[0].summary',
				'action'              => 'confirm_current',
				'claim_version'       => 7,
				'composition_id'      => 'composition-test-001',
				'composition_version' => 17,
				'request_id'          => 'request-test-001',
				'post_id'             => 123,
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

	private function verify_request( array $overrides = [] ): object {
		$params = array_merge(
			[
				'presence_nonce'      => 'nonce-token-value',
				'claim_id'            => 'claim-test-001',
				'action_kind'         => 'confirm_current',
				'field_path'          => 'claims[0].summary',
				'claim_version'       => 7,
				'composition_id'      => 'composition-test-001',
				'composition_version' => 17,
				'post_id'             => 123,
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
