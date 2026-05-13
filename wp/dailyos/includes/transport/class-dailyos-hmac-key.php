<?php
/**
 * DailyOS process-local HMAC key.
 *
 * @package DailyOS
 */

declare(strict_types=1);

namespace DailyOS\Transport;

use Closure;
use InvalidArgumentException;
use JsonSerializable;

/**
 * Holds a 32-byte per-session transport signing key.
 */
final class DailyOS_Hmac_Key implements JsonSerializable {
	private const REDACTED       = '***REDACTED***';
	private const KEY_BYTE_COUNT = 32;

	/**
	 * Key byte reader. A closure prevents var_export() from exposing the key.
	 *
	 * @var Closure(): string
	 */
	private Closure $key_bytes_reader;

	/**
	 * Redaction marker intentionally visible to var_export().
	 *
	 * @var string
	 */
	private string $redacted = self::REDACTED;

	/**
	 * Create a process-local HMAC key wrapper.
	 *
	 * @param string $raw_key_bytes Exactly 32 raw key bytes.
	 *
	 * @throws InvalidArgumentException When the key is not exactly 32 bytes.
	 */
	public function __construct( string $raw_key_bytes ) {
		if ( self::KEY_BYTE_COUNT !== strlen( $raw_key_bytes ) ) {
			throw new InvalidArgumentException( 'DailyOS HMAC keys must be exactly 32 bytes.' );
		}

		$this->key_bytes_reader = static fn(): string => $raw_key_bytes;
	}

	/**
	 * Compute the lowercase hexadecimal HMAC-SHA256 for canonical bytes.
	 *
	 * @param string $canonical_bytes Canonical request bytes.
	 * @return string Lowercase hexadecimal HMAC.
	 */
	public function hmac_sha256( string $canonical_bytes ): string {
		return strtolower( hash_hmac( 'sha256', $canonical_bytes, $this->raw_key_bytes() ) );
	}

	/**
	 * Compare two HMAC keys in constant time.
	 *
	 * @param self $other Other key wrapper.
	 * @return bool True when the wrapped key bytes match.
	 */
	public function equals( self $other ): bool {
		return hash_equals( $this->raw_key_bytes(), $other->raw_key_bytes() );
	}

	/**
	 * Redact secret-bearing fields in debug output.
	 *
	 * @return array<string, string> Redacted debug payload.
	 */
	public function __debugInfo(): array {
		return [
			'key' => self::REDACTED,
		];
	}

	/**
	 * Redact the key when interpolated.
	 *
	 * @return string Redacted marker.
	 */
	public function __toString(): string {
		return self::REDACTED;
	}

	/**
	 * Redact the key when JSON encoded.
	 *
	 * @return string Redacted marker.
	 */
	public function jsonSerialize(): string {
		return self::REDACTED;
	}

	/**
	 * Return raw key bytes for local cryptographic operations.
	 *
	 * @return string Raw key bytes.
	 */
	private function raw_key_bytes(): string {
		$reader = $this->key_bytes_reader;

		return $reader();
	}
}
