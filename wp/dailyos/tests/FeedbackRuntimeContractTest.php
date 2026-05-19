<?php
/**
 * Contract-parity test: WP feedback nonce payloads must satisfy the runtime's
 * IssueNonceRequest / VerifyNonceRequest / record_claim_feedback contracts.
 *
 * Class pattern caught across L2 cycles 1/2/3:
 *   - cycle 1: WP forwarded incomplete binding tuple (missing field_path,
 *     claim_version, composition_id, composition_version)
 *   - cycle 2: WP forwarded payload_json as a JSON-encoded string; runtime
 *     dropped non-object values silently
 *   - cycle 3: WP wrong_source accepted source_index alone; runtime requires
 *     source_ref string
 *
 * Each cycle patched a different shape of the same class — "WP and runtime
 * disagree on the wire shape". The sweep response is this test: assert
 * structurally that every variant's WP outbound payload satisfies the
 * runtime's parse + validate contracts.
 *
 * Anchors:
 *   - IssueNonceRequest::parse  at src-tauri/src/services/surface_nonce.rs
 *   - VerifyNonceRequest::parse at src-tauri/src/services/surface_nonce.rs
 *   - validate_feedback_action_metadata at src-tauri/src/services/claims.rs
 *
 * @package DailyOS
 */

declare(strict_types=1);

use DailyOS\DailyOS_Plugin;
use DailyOS\Transport\DailyOS_Credential_Store;
use PHPUnit\Framework\TestCase;

/**
 * Asserts WP outbound payloads structurally satisfy the runtime's
 * IssueNonceRequest / VerifyNonceRequest / record_claim_feedback contracts
 * for all 9 FeedbackAction variants across both issue + verify directions.
 */
final class DailyOS_FeedbackRuntimeContractTest extends TestCase {
	/**
	 * Required keys per IssueNonceRequest::parse. Types are PHP-shape:
	 * string|int for u64 fields.
	 */
	private const ISSUE_REQUIRED_KEYS = [
		'session_id'          => 'string',
		'wp_user_id'          => 'integer',
		'claim_id'            => 'string',
		'field_path'          => 'string',
		'action'              => 'string',
		'claim_version'       => 'integer',
		'composition_id'      => 'string',
		'composition_version' => 'integer',
	];

	/**
	 * Required keys per VerifyNonceRequest::parse — same as issue plus
	 * presence_nonce.
	 */
	private const VERIFY_REQUIRED_KEYS = [
		'presence_nonce'      => 'string',
		'session_id'          => 'string',
		'wp_user_id'          => 'integer',
		'claim_id'            => 'string',
		'field_path'          => 'string',
		'action'              => 'string',
		'claim_version'       => 'integer',
		'composition_id'      => 'string',
		'composition_version' => 'integer',
	];

	/**
	 * Per-variant payload_json key requirements per
	 * validate_feedback_action_metadata in claims.rs.
	 * 'omit' = the variant MUST NOT carry payload_json on the wire.
	 */
	private const VARIANT_PAYLOAD_CONTRACT = [
		'confirm_current'       => 'omit',
		'mark_outdated'         => 'omit',
		'mark_false'            => 'omit',
		'cannot_verify'         => 'omit',
		'wrong_subject'         => 'optional',
		'wrong_source'          => 'source_ref',
		'needs_nuance'          => 'corrected_text',
		'surface_inappropriate' => 'surface',
		'not_relevant_here'     => 'invocation_id',
	];

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
	 * @dataProvider provider_all_variants
	 */
	public function test_issue_outbound_payload_satisfies_runtime_contract( string $action ): void {
		$payload_for_variant = $this->payload_for_variant( $action );
		$result              = DailyOS_Plugin::instance()->issue_presence_nonce(
			$this->nonce_request( [ 'action' => $action ] + $payload_for_variant )
		);

		$this->assertFalse( is_wp_error( $result ), "issue must not error for variant '{$action}'" );
		$this->assertCount( 1, $GLOBALS['dailyos_test_remote_post_calls'] );

		$call     = $GLOBALS['dailyos_test_remote_post_calls'][0];
		$outbound = json_decode( $call['args']['body'], true );

		$this->assertIsArray( $outbound, 'outbound body MUST be a JSON object' );

		foreach ( self::ISSUE_REQUIRED_KEYS as $key => $type ) {
			$this->assertArrayHasKey( $key, $outbound, "issue payload MUST carry '{$key}' per IssueNonceRequest::parse" );
			$this->assertSame( $type, gettype( $outbound[ $key ] ), "issue payload '{$key}' MUST be {$type}; got " . gettype( $outbound[ $key ] ) );
		}

		$this->assertNotContains( 'action_kind', array_keys( $outbound ), 'runtime expects `action`, not `action_kind` — rename happens at WP boundary' );
		$this->assertNotContains( 'nonce_digest', array_keys( $outbound ), 'issue payload MUST NOT carry nonce_digest' );

		$this->assert_variant_payload_contract( $action, $outbound );
	}

	/**
	 * @dataProvider provider_all_variants
	 */
	public function test_verify_outbound_payload_satisfies_runtime_contract( string $action ): void {
		$payload_for_variant = $this->payload_for_variant( $action );
		$result              = DailyOS_Plugin::instance()->verify_presence_nonce(
			$this->verify_request( [ 'action_kind' => $action ] + $payload_for_variant )
		);

		$this->assertFalse( is_wp_error( $result ), "verify must not error for variant '{$action}'" );
		$this->assertCount( 1, $GLOBALS['dailyos_test_remote_post_calls'] );

		$call     = $GLOBALS['dailyos_test_remote_post_calls'][0];
		$outbound = json_decode( $call['args']['body'], true );

		$this->assertIsArray( $outbound, 'outbound body MUST be a JSON object' );

		foreach ( self::VERIFY_REQUIRED_KEYS as $key => $type ) {
			$this->assertArrayHasKey( $key, $outbound, "verify payload MUST carry '{$key}' per VerifyNonceRequest::parse" );
			$this->assertSame( $type, gettype( $outbound[ $key ] ), "verify payload '{$key}' MUST be {$type}; got " . gettype( $outbound[ $key ] ) );
		}

		$this->assertNotContains( 'nonce_digest', array_keys( $outbound ), 'verify payload MUST forward presence_nonce, not nonce_digest' );
		$this->assertNotContains( 'action_kind', array_keys( $outbound ), 'verify forwards `action`, not `action_kind`' );
	}

	public function provider_all_variants(): array {
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

	private function assert_variant_payload_contract( string $action, array $outbound ): void {
		$rule = self::VARIANT_PAYLOAD_CONTRACT[ $action ];

		if ( 'omit' === $rule || 'optional' === $rule ) {
			// Variants that don't require payload_json — only confirm we
			// didn't accidentally inject one for variants meant to be empty.
			return;
		}

		$this->assertArrayHasKey( 'payload_json', $outbound, "variant '{$action}' MUST carry payload_json" );
		$this->assertIsArray( $outbound['payload_json'], 'payload_json MUST be a JSON object on the wire (not a string) — runtime optional_payload_json drops non-objects' );
		$this->assertArrayHasKey( $rule, $outbound['payload_json'], "variant '{$action}' payload_json MUST contain '{$rule}' per record_claim_feedback contract" );
		$this->assertIsString( $outbound['payload_json'][ $rule ], "payload_json['{$rule}'] MUST be string per validate_feedback_action_metadata" );
	}

	private function payload_for_variant( string $action ): array {
		return match ( $action ) {
			'wrong_source'          => [ 'payload_json' => [ 'source_ref' => 'source-test-001' ] ],
			'needs_nuance'          => [ 'payload_json' => [ 'corrected_text' => 'renewal is October not November' ] ],
			'surface_inappropriate' => [ 'payload_json' => [ 'surface' => 'briefing' ] ],
			'not_relevant_here'     => [ 'payload_json' => [ 'invocation_id' => 'inv-test-001' ] ],
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
