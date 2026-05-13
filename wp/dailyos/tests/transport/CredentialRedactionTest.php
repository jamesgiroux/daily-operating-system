<?php
/**
 * DailyOS credential redaction tests.
 *
 * @package DailyOS
 */

declare(strict_types=1);

use DailyOS\Transport\DailyOS_Hmac_Key;
use DailyOS\Transport\DailyOS_Session_Credential;
use PHPUnit\Framework\TestCase;

/**
 * Verifies accidental debug output does not expose transport secrets.
 */
final class DailyOS_CredentialRedactionTest extends TestCase {
	/**
	 * Session credentials redact debug and serialization output.
	 */
	public function test_session_credential_redacts_debug_and_serialization_output(): void {
		$secret     = 'surface-bearer-secret-token';
		$session_id = 'surface-session-secret-id';
		$credential = new DailyOS_Session_Credential( $secret, $session_id );

		$this->assert_redacted_output( $credential, [ $secret, $session_id ] );
	}

	/**
	 * HMAC keys redact debug and serialization output.
	 */
	public function test_hmac_key_redacts_debug_and_serialization_output(): void {
		$secret = str_repeat( 'k', 32 );
		$key    = new DailyOS_Hmac_Key( $secret );

		$this->assert_redacted_output( $key, [ $secret ] );
	}

	/**
	 * Assert common stringification paths are redacted.
	 *
	 * @param object            $subject Object under test.
	 * @param array<int,string> $secrets Secret strings that must not appear.
	 * @return void
	 */
	private function assert_redacted_output( object $subject, array $secrets ): void {
		// phpcs:ignore WordPress.PHP.DevelopmentFunctions.error_log_print_r -- Redaction behavior under print_r() is the test subject.
		$print_output = print_r( $subject, true );
		// phpcs:ignore WordPress.PHP.DevelopmentFunctions.error_log_var_export -- Redaction behavior under var_export() is the test subject.
		$export_output = var_export( $subject, true );
		// phpcs:ignore WordPress.WP.AlternativeFunctions.json_encode_json_encode -- Native json_encode() behavior is the test subject.
		$json_output = json_encode( $subject );
		$cast_output = (string) $subject;
		$log_output  = sprintf( 'debug=%s', $subject );

		$this->assertStringContainsString( '***REDACTED***', $print_output );
		$this->assertStringContainsString( '***REDACTED***', $export_output );
		$this->assertSame( '"***REDACTED***"', $json_output );
		$this->assertSame( '***REDACTED***', $cast_output );
		$this->assertSame( 'debug=***REDACTED***', $log_output );

		foreach ( $secrets as $secret ) {
			$this->assertStringNotContainsString( $secret, $print_output );
			$this->assertStringNotContainsString( $secret, $export_output );
			$this->assertStringNotContainsString( $secret, (string) $json_output );
			$this->assertStringNotContainsString( $secret, $cast_output );
			$this->assertStringNotContainsString( $secret, $log_output );
		}
	}
}
