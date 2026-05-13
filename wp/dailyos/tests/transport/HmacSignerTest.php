<?php
/**
 * DailyOS HMAC signer tests.
 *
 * @package DailyOS
 */

declare(strict_types=1);

use DailyOS\Transport\DailyOS_Hmac_Key;
use DailyOS\Transport\DailyOS_Hmac_Signer;
use PHPUnit\Framework\TestCase;

/**
 * Verifies byte-exact HMAC canonicalization and signing.
 */
final class DailyOS_HmacSignerTest extends TestCase {
	/**
	 * Fixture vectors match the 15-field runtime canonical byte format.
	 *
	 * @dataProvider canonical_vector_provider
	 *
	 * @param array<string, mixed> $vector Canonical vector.
	 */
	public function test_canonical_vectors_match_runtime_fixture( array $vector ): void {
		$signer = new DailyOS_Hmac_Signer();
		// phpcs:ignore WordPress.PHP.DiscouragedPHPFunctions.obfuscation_base64_decode -- Fixture stores raw request bytes as base64.
		$body_bytes = base64_decode( (string) $vector['body_b64'], true );
		$key_bytes  = hex2bin( (string) $vector['session_key_hex'] );

		$this->assertIsString( $body_bytes );
		$this->assertIsString( $key_bytes );

		$canonical_bytes = $signer->canonical_bytes(
			(string) $vector['method'],
			(string) $vector['path_query'],
			(string) $vector['content_type'],
			$body_bytes,
			$this->identity_from_vector( $vector ),
			(string) $vector['nonce'],
			(string) $vector['timestamp']
		);
		$signature       = $signer->sign_request(
			new DailyOS_Hmac_Key( $key_bytes ),
			(string) $vector['method'],
			(string) $vector['path_query'],
			(string) $vector['content_type'],
			$body_bytes,
			$this->identity_from_vector( $vector ),
			(string) $vector['nonce'],
			(string) $vector['timestamp']
		);

		// phpcs:ignore WordPress.PHP.DiscouragedPHPFunctions.obfuscation_base64_encode -- Assertion compares raw canonical bytes through fixture-safe base64.
		$this->assertSame( (string) $vector['expected_canonical_bytes_b64'], base64_encode( $canonical_bytes ) );
		$this->assertStringStartsWith( 'v1=', $signature );
		$this->assertSame( (string) $vector['expected_signature_hex'], substr( $signature, 3 ) );
	}

	/**
	 * Minimal canonical bytes expose field order and zero-length encoding.
	 */
	public function test_minimal_canonical_byte_sequence(): void {
		$signer   = new DailyOS_Hmac_Signer();
		$identity = [
			'site_binding_digest'  => '',
			'site_nonce'           => '',
			'wp_user_id'           => '0',
			'wp_site_id'           => '',
			'home_url'             => '',
			'site_url'             => '',
			'wp_install_uuid'      => '',
			'plugin_instance_uuid' => '',
			'multisite_blog_id'    => '',
		];
		$expected = "DAILYOS-WP-BRIDGE-HMAC-V1\n"
			. "method:3\nGET\n"
			. "path_query:1\n/\n"
			. "content_type:0\n\n"
			. "body:0\n\n"
			. "site_binding_digest:0\n\n"
			. "site_nonce:0\n\n"
			. "wp_user_id:1\n0\n"
			. "wp_site_id:0\n\n"
			. "home_url:0\n\n"
			. "site_url:0\n\n"
			. "wp_install_uuid:0\n\n"
			. "plugin_instance_uuid:0\n\n"
			. "multisite_blog_id:0\n\n"
			. "nonce:1\nn\n"
			. "timestamp:1\n1\n";

		$this->assertSame(
			$expected,
			$signer->canonical_bytes( 'GET', '/', '', '', $identity, 'n', '1' )
		);
	}

	/**
	 * Nonces are 32 lowercase hexadecimal characters.
	 */
	public function test_generate_nonce_returns_32_hex_chars(): void {
		$nonce = ( new DailyOS_Hmac_Signer() )->generate_nonce();

		$this->assertMatchesRegularExpression( '/^[0-9a-f]{32}$/', $nonce );
	}

	/**
	 * Current timestamp is Unix seconds encoded as ASCII decimal digits.
	 */
	public function test_current_timestamp_matches_expected_format(): void {
		$timestamp = ( new DailyOS_Hmac_Signer() )->current_timestamp();

		$this->assertMatchesRegularExpression( '/^\d+$/', $timestamp );
	}

	/**
	 * Mutating one body byte changes the request signature.
	 */
	public function test_canonicalization_byte_exactness(): void {
		$signer    = new DailyOS_Hmac_Signer();
		$key       = new DailyOS_Hmac_Key( str_repeat( "\x01", 32 ) );
		$identity  = [
			'site_binding_digest'  => str_repeat( 'a', 64 ),
			'site_nonce'           => 'siteNonceAlpha123',
			'wp_user_id'           => '42',
			'wp_site_id'           => 'site-1',
			'home_url'             => 'https://example.test',
			'site_url'             => 'https://example.test',
			'wp_install_uuid'      => '00000000-0000-4000-8000-000000000101',
			'plugin_instance_uuid' => '00000000-0000-4000-8000-000000000201',
			'multisite_blog_id'    => '',
		];
		$signature = $signer->sign_request(
			$key,
			'POST',
			'/dailyos/v1/example',
			'application/json',
			'{"depth":"standard"}',
			$identity,
			'deadbeefcafebabedeadbeefcafebabe',
			'1731500000'
		);
		$changed   = $signer->sign_request(
			$key,
			'POST',
			'/dailyos/v1/example',
			'application/json',
			'{"depth":"standare"}',
			$identity,
			'deadbeefcafebabedeadbeefcafebabe',
			'1731500000'
		);

		$this->assertNotSame( $signature, $changed );
	}

	/**
	 * Canonical vector provider.
	 *
	 * @return array<string, array{0: array<string, mixed>}>
	 */
	public static function canonical_vector_provider(): array {
		// phpcs:ignore WordPress.WP.AlternativeFunctions.file_get_contents_file_get_contents -- Local test fixture.
		$contents = file_get_contents( __DIR__ . '/../fixtures/hmac_canonical_vectors.json' );

		if ( false === $contents ) {
			return [];
		}

		$decoded = json_decode( $contents, true );

		if ( ! is_array( $decoded ) ) {
			return [];
		}

		$cases = [];

		foreach ( $decoded as $vector ) {
			if ( is_array( $vector ) && isset( $vector['name'] ) ) {
				$cases[ (string) $vector['name'] ] = [ $vector ];
			}
		}

		return $cases;
	}

	/**
	 * Return canonical identity fields from a vector.
	 *
	 * @param array<string, mixed> $vector Canonical vector.
	 * @return array<string, string>
	 */
	private function identity_from_vector( array $vector ): array {
		$identity = isset( $vector['identity'] ) && is_array( $vector['identity'] )
			? $vector['identity']
			: [];

		return [
			'site_binding_digest'  => isset( $identity['site_binding_digest'] ) ? (string) $identity['site_binding_digest'] : '',
			'site_nonce'           => isset( $identity['site_nonce'] ) ? (string) $identity['site_nonce'] : '',
			'wp_user_id'           => isset( $identity['wp_user_id'] ) ? (string) $identity['wp_user_id'] : '',
			'wp_site_id'           => isset( $identity['wp_site_id'] ) ? (string) $identity['wp_site_id'] : '',
			'home_url'             => isset( $identity['home_url'] ) ? (string) $identity['home_url'] : '',
			'site_url'             => isset( $identity['site_url'] ) ? (string) $identity['site_url'] : '',
			'wp_install_uuid'      => isset( $identity['wp_install_uuid'] ) ? (string) $identity['wp_install_uuid'] : '',
			'plugin_instance_uuid' => isset( $identity['plugin_instance_uuid'] ) ? (string) $identity['plugin_instance_uuid'] : '',
			'multisite_blog_id'    => isset( $identity['multisite_blog_id'] ) ? (string) $identity['multisite_blog_id'] : '',
		];
	}
}
