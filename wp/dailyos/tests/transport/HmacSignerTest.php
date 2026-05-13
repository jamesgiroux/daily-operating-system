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
	private const METHOD       = 'POST';
	private const PATH_QUERY   = '/v1/surface/invoke?ability=briefing.daily';
	private const CONTENT_TYPE = 'application/json';
	private const BODY_BYTES   = '{"depth":"standard"}';
	private const NONCE        = 'deadbeefcafebabedeadbeefcafebabe';
	private const TIMESTAMP    = '2026-05-10T17:20:31Z';

	/**
	 * Canonical request matches the byte-exact length-prefixed fixture.
	 */
	public function test_canonical_request_happy_path(): void {
		$signer = new DailyOS_Hmac_Signer();

		$expected = "DAILYOS-WP-BRIDGE-HMAC-V1\n"
			. "method:4\n"
			. "POST\n"
			. "path_query:41\n"
			. "/v1/surface/invoke?ability=briefing.daily\n"
			. "content_type:16\n"
			. "application/json\n"
			. "body:20\n"
			. "{\"depth\":\"standard\"}\n"
			. "nonce:32\n"
			. "deadbeefcafebabedeadbeefcafebabe\n"
			. "timestamp:20\n"
			. "2026-05-10T17:20:31Z\n";

		$this->assertSame(
			$expected,
			$signer->canonical_request(
				self::METHOD,
				self::PATH_QUERY,
				self::CONTENT_TYPE,
				self::BODY_BYTES,
				self::NONCE,
				self::TIMESTAMP
			)
		);
	}

	/**
	 * Signatures match HMAC-SHA256 using a known zero key.
	 */
	public function test_sign_with_known_key(): void {
		$signer          = new DailyOS_Hmac_Signer();
		$key_bytes       = str_repeat( "\x00", 32 );
		$key             = new DailyOS_Hmac_Key( $key_bytes );
		$canonical_bytes = $signer->canonical_request(
			self::METHOD,
			self::PATH_QUERY,
			self::CONTENT_TYPE,
			self::BODY_BYTES,
			self::NONCE,
			self::TIMESTAMP
		);
		$expected_hex    = hash_hmac( 'sha256', $canonical_bytes, $key_bytes );
		$header_value    = $signer->sign_request(
			$key,
			self::METHOD,
			self::PATH_QUERY,
			self::CONTENT_TYPE,
			self::BODY_BYTES,
			self::NONCE,
			self::TIMESTAMP
		);
		$signature_hex   = substr( $header_value, 3 );

		$this->assertStringStartsWith( 'v1=', $header_value );
		$this->assertSame( 'v1=' . $expected_hex, $header_value );
		$this->assertSame( 64, strlen( $signature_hex ) );
		$this->assertSame( strtolower( $signature_hex ), $signature_hex );
		$this->assertMatchesRegularExpression( '/^[0-9a-f]{64}$/', $signature_hex );
	}

	/**
	 * Nonces are 32 lowercase hexadecimal characters.
	 */
	public function test_generate_nonce_returns_32_hex_chars(): void {
		$nonce = ( new DailyOS_Hmac_Signer() )->generate_nonce();

		$this->assertMatchesRegularExpression( '/^[0-9a-f]{32}$/', $nonce );
	}

	/**
	 * Current timestamp is RFC3339 UTC with a trailing Z.
	 */
	public function test_current_timestamp_matches_expected_format(): void {
		$timestamp = ( new DailyOS_Hmac_Signer() )->current_timestamp();

		$this->assertMatchesRegularExpression( '/^\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}Z$/', $timestamp );
	}

	/**
	 * Mutating one body byte changes the request signature.
	 */
	public function test_canonicalization_byte_exactness(): void {
		$signer    = new DailyOS_Hmac_Signer();
		$key       = new DailyOS_Hmac_Key( str_repeat( "\x01", 32 ) );
		$signature = $signer->sign_request(
			$key,
			self::METHOD,
			self::PATH_QUERY,
			self::CONTENT_TYPE,
			self::BODY_BYTES,
			self::NONCE,
			self::TIMESTAMP
		);
		$changed   = $signer->sign_request(
			$key,
			self::METHOD,
			self::PATH_QUERY,
			self::CONTENT_TYPE,
			'{"depth":"standare"}',
			self::NONCE,
			self::TIMESTAMP
		);

		$this->assertNotSame( $signature, $changed );
	}
}
