<?php
/**
 * Action allowlist + payload_json shape validation tests for the W4 feedback path.
 *
 * @package DailyOS
 */

declare(strict_types=1);

use DailyOS\DailyOS_Plugin;
use DailyOS\Transport\DailyOS_Credential_Store;
use PHPUnit\Framework\TestCase;

/**
 * Verifies the action_kind allowlist and payload_json variant-shape validation.
 */
final class DailyOS_SurfaceNonceFeedbackInputTest extends TestCase {
	protected function setUp(): void {
		parent::setUp();
		dailyos_test_reset_globals();
		( new DailyOS_Credential_Store() )->register_session_key_filter_safeguard();

		$GLOBALS['dailyos_test_is_user_logged_in'] = true;
		$GLOBALS['dailyos_test_current_user_id']   = 42;
		$this->save_marker();
		$this->add_session_key_filter();
		$GLOBALS['dailyos_test_remote_post_response'] = [
			'response' => [ 'code' => 200 ],
			'body'     => '{"ok":true}',
		];
	}

	/**
	 * @dataProvider provider_valid_actions
	 */
	public function test_accepts_each_of_the_9_actions( string $action ): void {
		$result = DailyOS_Plugin::instance()->issue_presence_nonce( $this->nonce_request( [ 'action' => $action ] + $this->payload_for( $action ) ) );

		$this->assertFalse( is_wp_error( $result ), "action_kind '{$action}' MUST be accepted; got " . ( is_wp_error( $result ) ? $result->get_error_code() : 'ok' ) );
	}

	public function provider_valid_actions(): array {
		return [
			[ 'confirm_current' ],
			[ 'mark_outdated' ],
			[ 'mark_false' ],
			[ 'wrong_subject' ],
			[ 'wrong_source' ],
			[ 'cannot_verify' ],
			[ 'needs_nuance' ],
			[ 'surface_inappropriate' ],
			[ 'not_relevant_here' ],
		];
	}

	/**
	 * @dataProvider provider_old_action_strings
	 */
	public function test_rejects_pre_v143_w4_action_strings( string $old_action ): void {
		$result = DailyOS_Plugin::instance()->issue_presence_nonce( $this->nonce_request( [ 'action' => $old_action ] ) );

		$this->assertTrue( is_wp_error( $result ), "Old action string '{$old_action}' MUST be rejected after W4." );
		$this->assertSame( 'malformed_request', $result->get_error_code() );
	}

	public function provider_old_action_strings(): array {
		return [
			[ 'correct' ],
			[ 'dismiss' ],
			[ 'corroborate' ],
			[ 'contradict' ],
		];
	}

	public function test_rejects_unknown_action(): void {
		$result = DailyOS_Plugin::instance()->issue_presence_nonce( $this->nonce_request( [ 'action' => 'shrug' ] ) );

		$this->assertTrue( is_wp_error( $result ) );
		$this->assertSame( 'malformed_request', $result->get_error_code() );
	}

	public function test_payload_json_required_for_needs_nuance(): void {
		$result = DailyOS_Plugin::instance()->issue_presence_nonce( $this->nonce_request( [ 'action' => 'needs_nuance' ] ) );

		$this->assertTrue( is_wp_error( $result ) );
		$this->assertSame( 'malformed_request', $result->get_error_code() );
	}

	public function test_payload_json_required_for_wrong_source(): void {
		$result = DailyOS_Plugin::instance()->issue_presence_nonce( $this->nonce_request( [ 'action' => 'wrong_source' ] ) );

		$this->assertTrue( is_wp_error( $result ) );
		$this->assertSame( 'malformed_request', $result->get_error_code() );
	}

	public function test_payload_json_rejected_for_variants_that_should_have_none(): void {
		$result = DailyOS_Plugin::instance()->issue_presence_nonce(
			$this->nonce_request(
				[
					'action'       => 'confirm_current',
					'payload_json' => [ 'extra' => 'not allowed' ],
				]
			)
		);

		$this->assertTrue( is_wp_error( $result ) );
		$this->assertSame( 'malformed_request', $result->get_error_code() );
	}

	public function test_payload_json_rejects_arrays(): void {
		$result = DailyOS_Plugin::instance()->issue_presence_nonce(
			$this->nonce_request(
				[
					'action'       => 'needs_nuance',
					'payload_json' => [ 'a', 'b', 'c' ],
				]
			)
		);

		$this->assertTrue( is_wp_error( $result ) );
		$this->assertSame( 'malformed_request', $result->get_error_code() );
	}

	public function test_payload_json_rejects_nested_objects(): void {
		$result = DailyOS_Plugin::instance()->issue_presence_nonce(
			$this->nonce_request(
				[
					'action'       => 'needs_nuance',
					'payload_json' => [ 'corrected_text' => [ 'nested' => 'object' ] ],
				]
			)
		);

		$this->assertTrue( is_wp_error( $result ) );
		$this->assertSame( 'malformed_request', $result->get_error_code() );
	}

	public function test_payload_json_caps_corrected_text_at_500_chars(): void {
		$result = DailyOS_Plugin::instance()->issue_presence_nonce(
			$this->nonce_request(
				[
					'action'       => 'needs_nuance',
					'payload_json' => [ 'corrected_text' => str_repeat( 'x', 501 ) ],
				]
			)
		);

		$this->assertTrue( is_wp_error( $result ) );
		$this->assertSame( 'malformed_request', $result->get_error_code() );
	}

	public function test_payload_json_accepts_valid_needs_nuance(): void {
		$result = DailyOS_Plugin::instance()->issue_presence_nonce(
			$this->nonce_request(
				[
					'action'       => 'needs_nuance',
					'payload_json' => [ 'corrected_text' => 'The claim missed the new acquisition date.' ],
				]
			)
		);

		$this->assertFalse( is_wp_error( $result ) );
	}

	public function test_payload_json_wrong_source_requires_source_ref_string(): void {
		// Source_index alone is rejected — record_claim_feedback requires
		// source_ref (string) per claims.rs:5185. Cycle-3 codex challenge
		// caught WP previously accepting source_index alone.
		$result = DailyOS_Plugin::instance()->issue_presence_nonce(
			$this->nonce_request(
				[
					'action'       => 'wrong_source',
					'payload_json' => [ 'source_index' => 2 ],
				]
			)
		);

		$this->assertTrue( is_wp_error( $result ) );
		$this->assertSame( 'malformed_request', $result->get_error_code() );
	}

	public function test_payload_json_wrong_source_accepts_source_ref_string(): void {
		$result = DailyOS_Plugin::instance()->issue_presence_nonce(
			$this->nonce_request(
				[
					'action'       => 'wrong_source',
					'payload_json' => [ 'source_ref' => 'source-test-001' ],
				]
			)
		);

		$this->assertFalse( is_wp_error( $result ) );
	}

	private function payload_for( string $action ): array {
		return match ( $action ) {
			'wrong_source'          => [
				'payload_json' => [
					'source_ref'   => 'source-test-001',
					'source_index' => 0,
				],
			],
			'needs_nuance'          => [ 'payload_json' => [ 'corrected_text' => 'short correction' ] ],
			'surface_inappropriate' => [ 'payload_json' => [ 'surface' => 'briefing' ] ],
			'not_relevant_here'     => [ 'payload_json' => [ 'invocation_id' => 'inv-1' ] ],
			default                 => [],
		};
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
