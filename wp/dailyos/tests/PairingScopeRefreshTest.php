<?php
/**
 * Auto scope-refresh hook tests.
 *
 * The plugin auto-calls the runtime's POST /v1/surface/pairing/refresh-scopes
 * endpoint on admin_init, throttled to once per 24h via a transient. A
 * runtime endpoint_version change forces an out-of-band refresh so deploys
 * that updated the scope catalog propagate before the next 24h window.
 *
 * @package DailyOS
 */

declare(strict_types=1);

use DailyOS\DailyOS_Plugin;
use DailyOS\Transport\DailyOS_Credential_Store;
use PHPUnit\Framework\TestCase;

// Local transient stubs — the test bootstrap does not provide them and the
// plugin's scope-refresh path is the only consumer at present.
if ( ! function_exists( 'get_transient' ) ) {
	/**
	 * @param string $key Transient key.
	 * @return mixed
	 */
	function get_transient( string $key ): mixed {
		if ( ! isset( $GLOBALS['dailyos_test_transients'] ) || ! is_array( $GLOBALS['dailyos_test_transients'] ) ) {
			return false;
		}
		return $GLOBALS['dailyos_test_transients'][ $key ] ?? false;
	}
}

if ( ! function_exists( 'set_transient' ) ) {
	/**
	 * @param string $key Transient key.
	 * @param mixed  $value Value.
	 * @param int    $ttl TTL in seconds (ignored — tests don't simulate expiry).
	 * @return bool
	 */
	function set_transient( string $key, mixed $value, int $ttl = 0 ): bool {
		unset( $ttl );
		if ( ! isset( $GLOBALS['dailyos_test_transients'] ) || ! is_array( $GLOBALS['dailyos_test_transients'] ) ) {
			$GLOBALS['dailyos_test_transients'] = [];
		}
		$GLOBALS['dailyos_test_transients'][ $key ] = $value;
		return true;
	}
}

/**
 * Asserts mechanism #2 of the scope-refresh design — auto scope refresh — gates correctly on
 * the transient throttle, forces refresh on endpoint_version change, and
 * only writes the marker when the runtime reports `changed: true`.
 */
final class DailyOS_PairingScopeRefreshTest extends TestCase {
	protected function setUp(): void {
		parent::setUp();
		dailyos_test_reset_globals();
		$GLOBALS['dailyos_test_transients']     = [];
		$GLOBALS['dailyos_test_is_user_logged_in'] = true;
		$GLOBALS['dailyos_test_current_user_id']   = 42;
		( new DailyOS_Credential_Store() )->register_session_key_filter_safeguard();
		$this->add_session_key_filter();
	}

	public function test_refresh_no_op_when_unpaired(): void {
		// No marker saved.
		DailyOS_Plugin::instance()->maybe_refresh_pairing_scopes();
		$this->assertCount( 0, $GLOBALS['dailyos_test_remote_post_calls'] );
		$this->assertSame( [], $GLOBALS['dailyos_test_transients'] );
	}

	public function test_refresh_throttled_within_24h_when_endpoint_version_unchanged(): void {
		$this->save_marker();
		$GLOBALS['dailyos_test_transients']['dailyos_last_scope_refresh_at'] = [
			'at'               => time() - 60,
			'endpoint_version' => 'v1',
		];

		DailyOS_Plugin::instance()->maybe_refresh_pairing_scopes();

		$this->assertCount( 0, $GLOBALS['dailyos_test_remote_post_calls'] );
	}

	public function test_refresh_forced_when_endpoint_version_differs(): void {
		$this->save_marker(); // marker endpoint_version is 'v1'.
		$GLOBALS['dailyos_test_transients']['dailyos_last_scope_refresh_at'] = [
			'at'               => time() - 60,
			'endpoint_version' => 'v0', // stale.
		];
		$this->set_runtime_response_changed( [ 'read.account_overview', 'submit.feedback' ] );

		DailyOS_Plugin::instance()->maybe_refresh_pairing_scopes();

		$this->assertCount( 1, $GLOBALS['dailyos_test_remote_post_calls'] );
		$this->assertStringContainsString(
			'/v1/surface/pairing/refresh-scopes',
			$GLOBALS['dailyos_test_remote_post_calls'][0]['url']
		);
	}

	public function test_refresh_updates_granted_scopes_when_changed(): void {
		$this->save_marker(); // stored scopes = ['submit.feedback'].
		$widened = [ 'read.account_overview', 'submit.feedback', 'manage.pairing' ];
		$this->set_runtime_response_changed( $widened );

		DailyOS_Plugin::instance()->maybe_refresh_pairing_scopes();

		$marker = ( new DailyOS_Credential_Store() )->get_marker();
		$this->assertNotNull( $marker );
		$this->assertSame( $widened, array_values( $marker['granted_scopes'] ) );
		$this->assertArrayHasKey( 'dailyos_last_scope_refresh_at', $GLOBALS['dailyos_test_transients'] );
	}

	public function test_refresh_skips_marker_write_when_unchanged(): void {
		$this->save_marker(); // stored scopes = ['submit.feedback'].
		$this->set_runtime_response_unchanged( [ 'submit.feedback' ] );

		DailyOS_Plugin::instance()->maybe_refresh_pairing_scopes();

		$marker = ( new DailyOS_Credential_Store() )->get_marker();
		$this->assertNotNull( $marker );
		$this->assertSame( [ 'submit.feedback' ], array_values( $marker['granted_scopes'] ) );
		$this->assertArrayHasKey( 'dailyos_last_scope_refresh_at', $GLOBALS['dailyos_test_transients'] );
	}

	public function test_refresh_does_not_persist_transient_on_wp_error(): void {
		$this->save_marker();
		$GLOBALS['dailyos_test_remote_post_response'] = new \WP_Error(
			'http_request_failed',
			'connection refused'
		);

		DailyOS_Plugin::instance()->maybe_refresh_pairing_scopes();

		// At least one remote_post happened (the refresh attempt itself).
		// The contract for this test is the transient: on transport failure,
		// the throttle MUST NOT advance so the next admin pageview retries.
		$this->assertGreaterThanOrEqual( 1, count( $GLOBALS['dailyos_test_remote_post_calls'] ) );
		$this->assertArrayNotHasKey(
			'dailyos_last_scope_refresh_at',
			$GLOBALS['dailyos_test_transients']
		);
	}

	/**
	 * @param array<int, string> $granted_scopes Scopes the runtime "now grants".
	 */
	private function set_runtime_response_changed( array $granted_scopes ): void {
		$GLOBALS['dailyos_test_remote_post_response'] = [
			'response' => [ 'code' => 200 ],
			'body'     => wp_json_encode(
				[
					'ok'               => true,
					'request_id'       => 'req-test-1',
					'endpoint_version' => 'v1',
					'refresh'          => [
						'surface_client_id'     => 'surface-client-123',
						'session_id'            => 'session-123',
						'granted_scopes'        => $granted_scopes,
						'scope_digest'          => str_repeat( 'b', 64 ),
						'previous_scope_digest' => str_repeat( 'a', 64 ),
						'changed'               => true,
						'ability_projection'    => [],
					],
				]
			),
		];
	}

	/**
	 * @param array<int, string> $granted_scopes Scopes the runtime reports unchanged.
	 */
	private function set_runtime_response_unchanged( array $granted_scopes ): void {
		$GLOBALS['dailyos_test_remote_post_response'] = [
			'response' => [ 'code' => 200 ],
			'body'     => wp_json_encode(
				[
					'ok'               => true,
					'request_id'       => 'req-test-2',
					'endpoint_version' => 'v1',
					'refresh'          => [
						'surface_client_id'     => 'surface-client-123',
						'session_id'            => 'session-123',
						'granted_scopes'        => $granted_scopes,
						'scope_digest'          => str_repeat( 'a', 64 ),
						'previous_scope_digest' => str_repeat( 'a', 64 ),
						'changed'               => false,
						'ability_projection'    => [],
					],
				]
			),
		];
	}

	private function add_session_key_filter(): void {
		add_filter(
			'dailyos_wp_bridge_session_key',
			static function (): array {
				return [
					'hmac_key'   => str_repeat( chr( 0 ), 32 ),
					'session_id' => 'session-123',
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
