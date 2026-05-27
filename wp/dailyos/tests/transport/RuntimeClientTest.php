<?php
/**
 * DailyOS runtime client transport tests.
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
 * Verifies runtime client HTTP arguments preserve transport caveats.
 */
final class DailyOS_RuntimeClientTest extends TestCase {
	/**
	 * Reset WordPress stubs before each runtime-client test.
	 */
	protected function setUp(): void {
		parent::setUp();

		dailyos_test_reset_globals();
		DailyOS_Plugin::invalidate_runtime_endpoint_cache();
		( new DailyOS_Credential_Store() )->register_session_key_filter_safeguard();
	}

	/**
	 * Local ability invokes send string bodies and only trace headers.
	 */
	public function test_local_post_uses_string_body_two_headers_and_request_id(): void {
		$this->save_marker();

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
		$this->assertSame(
			[
				'ability' => 'briefing.daily',
				'input'   => [ 'depth' => 'standard' ],
			],
			json_decode( $args['body'], true )
		);
		$this->assertSame( 'application/json', $headers['Content-Type'] );
		$this->assertArrayHasKey( 'X-DailyOS-Request-Id', $headers );
		$this->assertSame( [ 'Content-Type', 'X-DailyOS-Request-Id' ], array_keys( $headers ) );
		$this->assertSame( 0, $args['redirection'] );
		$this->assertSame( 90, $args['timeout'] );
		$this->assertSame( 'http://127.0.0.1:54321/v1/local/invoke', $call['url'] );
	}

	/**
	 * Entity intelligence invokes are normalized to the current Rust DTO.
	 */
	public function test_get_entity_intelligence_payload_is_normalized_to_runtime_contract(): void {
		$this->save_marker();

		$GLOBALS['dailyos_test_remote_post_response'] = [
			'response' => [
				'code' => 200,
			],
			'body'     => '{"ok":true}',
		];

		$client = new DailyOS_Runtime_Client( new DailyOS_Credential_Store(), new DailyOS_Hmac_Signer() );
		$client->invoke_ability(
			'get_entity_intelligence',
			[
				'schema_version' => 1,
				'entity_type'    => 'account',
				'entity_id'      => 'acct-test-001',
				'depth'          => 'Full',
				'sections'       => null,
			],
			[ 'read.entity_intelligence' ]
		);

		$body = json_decode( $GLOBALS['dailyos_test_remote_post_calls'][0]['args']['body'], true );

		$this->assertSame( 'get_entity_intelligence', $body['ability'] );
		$this->assertSame( 1, $body['input']['schemaVersion'] );
		$this->assertSame( 'account', $body['input']['entityType'] );
		$this->assertSame( 'acct-test-001', $body['input']['entityId'] );
		$this->assertSame( 'deep', $body['input']['depth'] );
		$this->assertArrayNotHasKey( 'sections', $body['input'] );
		$this->assertArrayNotHasKey( 'schema_version', $body['input'] );
		$this->assertArrayNotHasKey( 'entity_type', $body['input'] );
		$this->assertArrayNotHasKey( 'entity_id', $body['input'] );
	}

	/**
	 * Claim receipt invokes normalize raw claim refs to the Rust ability DTO.
	 */
	public function test_claim_receipt_payload_normalizes_raw_claim_ref_to_runtime_contract(): void {
		$this->save_marker();
		$this->add_session_key_filter();

		$GLOBALS['dailyos_test_remote_post_response'] = [
			'response' => [
				'code' => 200,
			],
			'body'     => '{"ok":true}',
		];

		$client = new DailyOS_Runtime_Client( new DailyOS_Credential_Store(), new DailyOS_Hmac_Signer() );
		$client->invoke_ability(
			'claim_receipt',
			[
				'claim_id'    => 'claim-test-001',
				'subject_ref' => [
					'kind' => 'account',
					'id'   => 'acct-test-001',
				],
				'field_path'  => 'health.risk',
				'surface'     => 'entityDetail',
			],
			[ 'read.claim_receipt' ]
		);

		$body = json_decode( $GLOBALS['dailyos_test_remote_post_calls'][0]['args']['body'], true );

		$this->assertSame( 'claim_receipt', $body['ability'] );
		$this->assertSame( 1, $body['input']['schemaVersion'] );
		$this->assertSame( 'entity_detail', $body['input']['surface'] );
		$this->assertSame( 'claim', $body['input']['target']['kind'] );
		$this->assertSame( 'claim-test-001', $body['input']['target']['claimId'] );
		$this->assertSame( [ 'account' => 'acct-test-001' ], $body['input']['target']['subject'] );
		$this->assertSame( 'health.risk', $body['input']['target']['fieldPath'] );
		$this->assertArrayNotHasKey( 'claim_id', $body['input'] );
		$this->assertArrayNotHasKey( 'subject_ref', $body['input'] );
		$this->assertArrayNotHasKey( 'field_path', $body['input'] );
	}

	/**
	 * Claim receipt invokes normalize shaped targets from envelope subject refs.
	 */
	public function test_claim_receipt_payload_normalizes_shaped_target_subject_ref(): void {
		$this->save_marker();
		$this->add_session_key_filter();

		$GLOBALS['dailyos_test_remote_post_response'] = [
			'response' => [
				'code' => 200,
			],
			'body'     => '{"ok":true}',
		];

		$client = new DailyOS_Runtime_Client( new DailyOS_Credential_Store(), new DailyOS_Hmac_Signer() );
		$client->invoke_ability(
			'claim_receipt',
			[
				'schema_version' => 1,
				'target'         => [
					'kind'       => 'claim',
					'claim_id'   => 'claim-test-002',
					'subjectRef' => [
						'kind' => 'person',
						'id'   => 'person-test-001',
					],
					'field_path' => 'role.current',
				],
				'surface'        => 'tauri_entity_detail',
			],
			[ 'read.claim_receipt' ]
		);

		$body = json_decode( $GLOBALS['dailyos_test_remote_post_calls'][0]['args']['body'], true );

		$this->assertSame( 'claim_receipt', $body['ability'] );
		$this->assertSame( 1, $body['input']['schemaVersion'] );
		$this->assertArrayNotHasKey( 'schema_version', $body['input'] );
		$this->assertSame( 'entity_detail', $body['input']['surface'] );
		$this->assertSame( 'claim', $body['input']['target']['kind'] );
		$this->assertSame( 'claim-test-002', $body['input']['target']['claimId'] );
		$this->assertSame( [ 'person' => 'person-test-001' ], $body['input']['target']['subject'] );
		$this->assertSame( 'role.current', $body['input']['target']['fieldPath'] );
		$this->assertArrayNotHasKey( 'claim_id', $body['input']['target'] );
		$this->assertArrayNotHasKey( 'subjectRef', $body['input']['target'] );
		$this->assertArrayNotHasKey( 'field_path', $body['input']['target'] );
	}

	/**
	 * Markdown preview reads use the signed SurfaceClient invoke path.
	 */
	public function test_read_markdown_preview_uses_signed_surface_invoke_and_normalizes_data(): void {
		$this->save_marker();
		$this->add_session_key_filter();

		$GLOBALS['dailyos_test_remote_post_response'] = [
			'response' => [
				'code' => 200,
			],
			'body'     => wp_json_encode(
				[
					'ok'         => true,
					'request_id' => 'request-alpha',
					'ability'    => [
						'ability_name' => 'markdown_preview',
						'data'         => [
							'schemaVersion'          => 1,
							'previewHtml'            => '<p>Safe</p>',
							'sourceAsof'             => '2026-05-24T10:00:00Z',
							'lifecycleState'         => 'ingested',
							'trustBandSummary'       => 'needs_verification',
							'sourceLabel'            => 'Workspace source',
							'blockedAssetCount'      => 0,
							'assetResolverAvailable' => false,
							'sanitizerVersion'       => 'markdown-preview-v1',
						],
					],
				]
			),
		];

		$client = new DailyOS_Runtime_Client( new DailyOS_Credential_Store(), new DailyOS_Hmac_Signer() );
		$result = $client->read_markdown_preview( 'source_opaque_123' );

		$this->assertSame( '<p>Safe</p>', $result['data']['previewHtml'] );
		$this->assertCount( 1, $GLOBALS['dailyos_test_remote_post_calls'] );

		$call    = $GLOBALS['dailyos_test_remote_post_calls'][0];
		$args    = $call['args'];
		$headers = $args['headers'];
		$body    = json_decode( $args['body'], true );

		$this->assertSame( 'http://127.0.0.1:54321/v1/surface/invoke', $call['url'] );
		$this->assertSame(
			[
				'ability' => 'markdown_preview',
				'input'   => [
					'schemaVersion' => 1,
					'sourceHandle'  => 'source_opaque_123',
				],
			],
			$body
		);
		$this->assertSame( 'string', gettype( $args['body'] ) );
		$this->assertArrayHasKey( 'X-DailyOS-Signature', $headers );
		$this->assertArrayHasKey( 'X-DailyOS-Session-Id', $headers );
		$this->assertArrayHasKey( 'X-DailyOS-SurfaceClient', $headers );
		$this->assertArrayHasKey( 'X-DailyOS-Request-Id', $headers );
		$this->assertArrayHasKey( 'Accept', $headers );
	}

	/**
	 * Source-management ledger reads use the signed SurfaceClient invoke path.
	 */
	public function test_read_source_management_ledger_uses_signed_surface_invoke_and_normalizes_data(): void {
		$this->save_marker();
		$this->add_session_key_filter();

		$GLOBALS['dailyos_test_remote_post_response'] = [
			'response' => [
				'code' => 200,
			],
			'body'     => wp_json_encode(
				[
					'ok'         => true,
					'request_id' => 'request-source-ledger',
					'ability'    => [
						'ability_name' => 'source_management_ledger',
						'data'         => [
							'schemaVersion' => 1,
							'sources'       => [],
						],
					],
				]
			),
		];

		$client = new DailyOS_Runtime_Client( new DailyOS_Credential_Store(), new DailyOS_Hmac_Signer() );
		$result = $client->read_source_management_ledger( 'account', 'acct-test-001', 50 );

		$this->assertSame( 1, $result['data']['schemaVersion'] );
		$this->assertCount( 1, $GLOBALS['dailyos_test_remote_post_calls'] );

		$call    = $GLOBALS['dailyos_test_remote_post_calls'][0];
		$args    = $call['args'];
		$headers = $args['headers'];
		$body    = json_decode( $args['body'], true );

		$this->assertSame( 'http://127.0.0.1:54321/v1/surface/invoke', $call['url'] );
		$this->assertSame(
			[
				'ability' => 'source_management_ledger',
				'input'   => [
					'schemaVersion' => 1,
					'entityType'    => 'account',
					'entityId'      => 'acct-test-001',
					'pageSize'      => 50,
				],
			],
			$body
		);
		$this->assertSame( 'string', gettype( $args['body'] ) );
		$this->assertArrayHasKey( 'X-DailyOS-Signature', $headers );
		$this->assertArrayHasKey( 'X-DailyOS-Session-Id', $headers );
		$this->assertArrayHasKey( 'X-DailyOS-SurfaceClient', $headers );
		$this->assertArrayHasKey( 'X-DailyOS-Request-Id', $headers );
		$this->assertArrayHasKey( 'Accept', $headers );
	}

	/**
	 * Source-management actions use the signed SurfaceClient invoke path.
	 */
	public function test_apply_source_management_action_uses_signed_surface_invoke(): void {
		$this->save_marker();
		$this->add_session_key_filter();

		$GLOBALS['dailyos_test_remote_post_response'] = [
			'response' => [
				'code' => 200,
			],
			'body'     => wp_json_encode(
				[
					'ok'         => true,
					'request_id' => 'request-source-action',
					'ability'    => [
						'ability_name' => 'source_management_action',
						'data'         => [
							'schemaVersion'  => 1,
							'action'         => 'reingest',
							'status'         => 'reingested',
							'sourceKey'      => 'source:v1:abcdefghijklmnopqrstuvwxyzABCDEF0123456789_-',
							'lifecycleState' => 'ingested',
						],
					],
				]
			),
		];

		$client = new DailyOS_Runtime_Client( new DailyOS_Credential_Store(), new DailyOS_Hmac_Signer() );
		$result = $client->apply_source_management_action( 'reingest', 'account', 'acct-test-001', 'source:v1:abcdefghijklmnopqrstuvwxyzABCDEF0123456789_-' );

		$this->assertSame( 'reingested', $result['data']['status'] );
		$call = $GLOBALS['dailyos_test_remote_post_calls'][0];
		$body = json_decode( $call['args']['body'], true );

		$this->assertSame( 'http://127.0.0.1:54321/v1/surface/invoke', $call['url'] );
		$this->assertSame(
			[
				'ability' => 'source_management_action',
				'input'   => [
					'schemaVersion' => 1,
					'action'        => 'reingest',
					'entityType'    => 'account',
					'entityId'      => 'acct-test-001',
					'sourceKey'     => 'source:v1:abcdefghijklmnopqrstuvwxyzABCDEF0123456789_-',
				],
			],
			$body
		);
		$this->assertArrayHasKey( 'X-DailyOS-Signature', $call['args']['headers'] );
	}

	/**
	 * Local requests refuse to guess a default runtime URL when no marker exists.
	 */
	public function test_local_post_returns_not_paired_without_marker(): void {
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

		$this->assertSame( 'http://127.0.0.1:54322/v1/local/invoke', $GLOBALS['dailyos_test_remote_post_calls'][0]['url'] );
	}

	/**
	 * Sentinel discovery follows the new port after a hot Tauri restart.
	 */
	public function test_runtime_sentinel_cache_resets_after_restart_and_uses_new_port(): void {
		$original_home = getenv( 'HOME' );
		$home          = sys_get_temp_dir() . '/dailyos-sentinel-' . uniqid( '', true );
		mkdir( $home . '/.dailyos', 0700, true );

		try {
			putenv( 'HOME=' . $home );
			$this->save_marker();
			$this->add_session_key_filter();
			$GLOBALS['dailyos_test_remote_post_response'] = [
				'response' => [ 'code' => 200 ],
				'body'     => '{"ok":true}',
			];

			$this->write_runtime_sentinel( $home, 54322 );
			DailyOS_Plugin::invalidate_runtime_endpoint_cache();
			$client = new DailyOS_Runtime_Client( new DailyOS_Credential_Store(), new DailyOS_Hmac_Signer() );
			$client->invoke_ability( 'briefing.daily', [], [] );
			$this->assertSame( 'http://127.0.0.1:54322/v1/local/invoke', $GLOBALS['dailyos_test_remote_post_calls'][0]['url'] );

			$this->write_runtime_sentinel( $home, 54323 );
			DailyOS_Plugin::invalidate_runtime_endpoint_cache();
			$GLOBALS['dailyos_test_remote_post_calls'] = [];
			$client->invoke_ability( 'briefing.daily', [], [] );
			$this->assertSame( 'http://127.0.0.1:54323/v1/local/invoke', $GLOBALS['dailyos_test_remote_post_calls'][0]['url'] );
		} finally {
			DailyOS_Plugin::invalidate_runtime_endpoint_cache();
			if ( false === $original_home ) {
				putenv( 'HOME' );
			} else {
				putenv( 'HOME=' . $original_home );
			}
			$this->remove_runtime_sentinel_home( $home );
		}
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

		$this->assertSame( 'http://127.0.0.1:54321/v1/local/invoke', $GLOBALS['dailyos_test_remote_post_calls'][0]['url'] );
	}

	/**
	 * Session refresh filter posts marker identity and returns process-local material.
	 */
	public function test_session_refresh_filter_posts_marker_identity_and_returns_session_material(): void {
		$hmac_key_bytes = str_repeat( "\x03", 32 );

		$this->save_marker();

		$GLOBALS['dailyos_test_remote_post_response'] = [
			'response' => [
				'code' => 200,
			],
			'body'     => wp_json_encode(
				[
					'ok'       => true,
					'hmac_key' => bin2hex( $hmac_key_bytes ),
				]
			),
		];

		DailyOS_Plugin::instance()->register_transport();

		$material = apply_filters( 'dailyos_wp_bridge_session_key', null );

		$this->assertIsArray( $material );
		$this->assertSame( $hmac_key_bytes, $material['hmac_key'] );
		$this->assertSame( 'session-123', $material['session_id'] );
		$this->assertCount( 1, $GLOBALS['dailyos_test_remote_post_calls'] );

		$call = $GLOBALS['dailyos_test_remote_post_calls'][0];
		$args = $call['args'];

		$this->assertSame( 'http://127.0.0.1:54321/v1/surface/session/refresh', $call['url'] );
		$this->assertSame( 5, $args['timeout'] );
		$this->assertFalse( $args['sslverify'] );
		$this->assertSame( 'string', gettype( $args['body'] ) );
		$this->assertSame(
			[
				'session_id'           => 'session-123',
				'site_binding_digest'  => str_repeat( 'a', 64 ),
				'wp_install_uuid'      => 'install-1',
				'plugin_instance_uuid' => 'plugin-1',
			],
			json_decode( $args['body'], true )
		);
	}

	/**
	 * Session refresh filter leaves existing candidates untouched.
	 */
	public function test_session_refresh_filter_skips_existing_candidate(): void {
		$candidate = [
			'hmac_key'   => str_repeat( "\x04", 32 ),
			'session_id' => 'existing-session',
		];

		$result = DailyOS_Plugin::instance()->refresh_session_key( $candidate );

		$this->assertSame( $candidate, $result );
		$this->assertSame( [], $GLOBALS['dailyos_test_remote_post_calls'] );
	}

	/**
	 * Session refresh filter rejects failed runtime refresh responses.
	 */
	public function test_session_refresh_filter_rejects_non_200_response(): void {
		$this->save_marker();

		$GLOBALS['dailyos_test_remote_post_response'] = [
			'response' => [
				'code' => 500,
			],
			'body'     => '{"ok":false}',
		];

		DailyOS_Plugin::instance()->register_transport();

		$this->assertNull( apply_filters( 'dailyos_wp_bridge_session_key', null ) );
		$this->assertCount( 1, $GLOBALS['dailyos_test_remote_post_calls'] );
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
	 * Write a strict sentinel payload for tests.
	 */
	private function write_runtime_sentinel( string $home, int $port ): void {
		$path = $home . '/.dailyos/runtime-endpoint.json';
		file_put_contents(
			$path,
			wp_json_encode(
				[
					'port'            => $port,
					'runtime_version' => 'v1-test',
				]
			)
		);
		chmod( $path, 0600 );
	}

	/**
	 * Remove a temporary sentinel home directory.
	 */
	private function remove_runtime_sentinel_home( string $home ): void {
		$path = $home . '/.dailyos/runtime-endpoint.json';
		if ( is_file( $path ) ) {
			unlink( $path );
		}
		if ( is_dir( $home . '/.dailyos' ) ) {
			rmdir( $home . '/.dailyos' );
		}
		if ( is_dir( $home ) ) {
			rmdir( $home );
		}
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
