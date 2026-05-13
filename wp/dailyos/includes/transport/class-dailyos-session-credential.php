<?php
/**
 * DailyOS process-local session credential.
 *
 * @package DailyOS
 */

declare(strict_types=1);

namespace DailyOS\Transport;

use Closure;
use JsonSerializable;

/**
 * Holds bearer credential material for the active transport session.
 */
final class DailyOS_Session_Credential implements JsonSerializable {
	private const REDACTED = '***REDACTED***';

	/**
	 * Bearer token reader. A closure prevents var_export() from exposing the token.
	 *
	 * @var Closure(): string
	 */
	private Closure $bearer_token_reader;

	/**
	 * Session id reader. A closure keeps debug exports uniformly redacted.
	 *
	 * @var Closure(): string
	 */
	private Closure $session_id_reader;

	/**
	 * Redaction marker intentionally visible to var_export().
	 *
	 * @var string
	 */
	private string $redacted = self::REDACTED;

	/**
	 * Create a process-local session credential.
	 *
	 * @param string $bearer_token Runtime-issued bearer token.
	 * @param string $session_id Runtime-issued session id.
	 */
	public function __construct( string $bearer_token, string $session_id ) {
		$this->bearer_token_reader = static fn(): string => $bearer_token;
		$this->session_id_reader   = static fn(): string => $session_id;
	}

	/**
	 * Return the bearer token for package-internal transport callers.
	 *
	 * @internal
	 *
	 * @return string Bearer token bytes.
	 */
	public function bearer_token(): string {
		$reader = $this->bearer_token_reader;

		return $reader();
	}

	/**
	 * Return the runtime session id.
	 *
	 * @return string Session id.
	 */
	public function session_id(): string {
		$reader = $this->session_id_reader;

		return $reader();
	}

	/**
	 * Redact secret-bearing fields in debug output.
	 *
	 * @return array<string, string> Redacted debug payload.
	 */
	public function __debugInfo(): array {
		return [
			'credential' => self::REDACTED,
		];
	}

	/**
	 * Redact the credential when interpolated.
	 *
	 * @return string Redacted marker.
	 */
	public function __toString(): string {
		return self::REDACTED;
	}

	/**
	 * Redact the credential when JSON encoded.
	 *
	 * @return string Redacted marker.
	 */
	public function jsonSerialize(): string {
		return self::REDACTED;
	}
}
