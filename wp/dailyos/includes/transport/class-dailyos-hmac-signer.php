<?php
/**
 * DailyOS HMAC request signer.
 *
 * @package DailyOS
 */

declare(strict_types=1);

namespace DailyOS\Transport;

use InvalidArgumentException;

/**
 * Builds and signs byte-exact DailyOS runtime canonical requests.
 */
final class DailyOS_Hmac_Signer {
	private const DOMAIN_SEPARATOR = 'DAILYOS-WP-BRIDGE-HMAC-V1';

	/**
	 * Build the length-prefixed canonical request bytes.
	 *
	 * @param string $method HTTP method.
	 * @param string $path_query Request path and query string exactly as sent.
	 * @param string $content_type Content-Type header value.
	 * @param string $body_bytes Exact request body bytes.
	 * @param string $nonce X-DailyOS-Nonce header value.
	 * @param string $timestamp X-DailyOS-Timestamp header value.
	 * @return string Canonical request bytes.
	 */
	public function canonical_request(
		string $method,
		string $path_query,
		string $content_type,
		string $body_bytes,
		string $nonce,
		string $timestamp
	): string {
		$normalized_method       = strtoupper( $method );
		$normalized_content_type = trim( $content_type, " \t\n\r\0\x0B" );

		$this->assert_utf8( 'method', $normalized_method );
		$this->assert_utf8( 'path_query', $path_query );
		$this->assert_utf8( 'content_type', $normalized_content_type );
		$this->assert_utf8( 'nonce', $nonce );
		$this->assert_utf8( 'timestamp', $timestamp );

		return self::DOMAIN_SEPARATOR . "\n"
			. $this->canonical_field( 'method', $normalized_method )
			. $this->canonical_field( 'path_query', $path_query )
			. $this->canonical_field( 'content_type', $normalized_content_type )
			. $this->canonical_field( 'body', $body_bytes )
			. $this->canonical_field( 'nonce', $nonce )
			. $this->canonical_field( 'timestamp', $timestamp );
	}

	/**
	 * Sign a request and return the transport signature header value.
	 *
	 * @param DailyOS_Hmac_Key $key Per-session HMAC key.
	 * @param string           $method HTTP method.
	 * @param string           $path_query Request path and query string exactly as sent.
	 * @param string           $content_type Content-Type header value.
	 * @param string           $body_bytes Exact request body bytes.
	 * @param string           $nonce X-DailyOS-Nonce header value.
	 * @param string           $timestamp X-DailyOS-Timestamp header value.
	 * @return string Header value in v1=<lowercase-hex> form.
	 */
	public function sign_request(
		DailyOS_Hmac_Key $key,
		string $method,
		string $path_query,
		string $content_type,
		string $body_bytes,
		string $nonce,
		string $timestamp
	): string {
		$canonical_bytes = $this->canonical_request(
			$method,
			$path_query,
			$content_type,
			$body_bytes,
			$nonce,
			$timestamp
		);

		return 'v1=' . $key->hmac_sha256( $canonical_bytes );
	}

	/**
	 * Generate a 128-bit random lowercase hexadecimal nonce.
	 *
	 * @return string Nonce.
	 */
	public function generate_nonce(): string {
		return bin2hex( random_bytes( 16 ) );
	}

	/**
	 * Return the current UTC timestamp in RFC3339 form with a trailing Z.
	 *
	 * @return string Timestamp.
	 */
	public function current_timestamp(): string {
		return gmdate( 'Y-m-d\TH:i:s\Z', time() );
	}

	/**
	 * Serialize one canonical field.
	 *
	 * @param string $label Field label.
	 * @param string $value Field bytes.
	 * @return string Serialized field.
	 */
	private function canonical_field( string $label, string $value ): string {
		return $label . ':' . strlen( $value ) . "\n" . $value . "\n";
	}

	/**
	 * Assert that non-body canonical fields are valid UTF-8.
	 *
	 * @param string $label Field label.
	 * @param string $value Field value.
	 *
	 * @throws InvalidArgumentException When the field is not UTF-8.
	 */
	private function assert_utf8( string $label, string $value ): void {
		if ( 1 !== preg_match( '//u', $value ) ) {
			throw new InvalidArgumentException( 'DailyOS canonical field is not valid UTF-8: ' . esc_html( $label ) );
		}
	}
}
