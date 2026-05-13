<?php
/**
 * DailyOS loopback runtime client.
 *
 * @package DailyOS
 */

declare(strict_types=1);

namespace DailyOS\Transport;

/**
 * Sends byte-exact JSON requests to the local DailyOS runtime.
 */
final class DailyOS_Runtime_Client {
	private const DEFAULT_RUNTIME_URL = 'http://127.0.0.1:8765';
	private const CONTENT_TYPE        = 'application/json';

	/**
	 * Non-secret marker and process-local credential source.
	 *
	 * @var DailyOS_Credential_Store
	 */
	private DailyOS_Credential_Store $credential_store;

	/**
	 * Canonical request signer.
	 *
	 * @var DailyOS_Hmac_Signer
	 */
	private DailyOS_Hmac_Signer $signer;

	/**
	 * Create a runtime client.
	 *
	 * @param DailyOS_Credential_Store $credential_store Credential store.
	 * @param DailyOS_Hmac_Signer      $signer HMAC signer.
	 */
	public function __construct( DailyOS_Credential_Store $credential_store, DailyOS_Hmac_Signer $signer ) {
		$this->credential_store = $credential_store;
		$this->signer           = $signer;
	}

	/**
	 * Complete a pairing handshake with the local runtime.
	 *
	 * @param string               $pairing_code Pairing code from the runtime.
	 * @param array<string, mixed> $wp_context WordPress context.
	 * @return array{ok: bool, runtime_instance_id: ?string, site_nonce_hash: ?string, projection_version: ?string, instance_id: ?string, session_id: ?string, scopes: array<int, string>, endpoint_version: ?string, error: ?array<string, mixed>} Handshake envelope.
	 */
	public function handshake( string $pairing_code, array $wp_context ): array {
		$body_bytes = $this->encode_json(
			[
				'pairing_code' => $pairing_code,
				'wp_context'   => $wp_context,
			]
		);

		if ( null === $body_bytes ) {
			return $this->handshake_error_envelope( 'json_encode_failed', 'DailyOS pairing request could not be encoded.' );
		}

		return $this->normalize_handshake_response( $this->plain_post( '/v1/pairing/handshake', $body_bytes ) );
	}

	/**
	 * Invoke a scoped runtime ability.
	 *
	 * @param string               $name Ability name.
	 * @param array<string, mixed> $payload Ability payload.
	 * @param array<int, string>   $scope_set Requested scope set.
	 * @return array<string, mixed> Runtime response envelope.
	 */
	public function invoke_ability( string $name, array $payload, array $scope_set ): array {
		$body_bytes = $this->encode_json(
			[
				'ability' => $name,
				'payload' => $payload,
				'scopes'  => $scope_set,
			]
		);

		if ( null === $body_bytes ) {
			return $this->error_response( 'json_encode_failed', 'DailyOS ability request could not be encoded.' );
		}

		return $this->signed_post( '/v1/surface/invoke', $body_bytes );
	}

	/**
	 * Submit feedback for a runtime surface claim.
	 *
	 * @param string $claim_id Claim id.
	 * @param string $field Field name.
	 * @param string $action Feedback action.
	 * @param string $presence_nonce User-presence nonce.
	 * @return array<string, mixed> Runtime response envelope.
	 */
	public function submit_feedback( string $claim_id, string $field, string $action, string $presence_nonce ): array {
		$body_bytes = $this->encode_json(
			[
				'claim_id'       => $claim_id,
				'field'          => $field,
				'action'         => $action,
				'presence_nonce' => $presence_nonce,
			]
		);

		if ( null === $body_bytes ) {
			return $this->error_response( 'json_encode_failed', 'DailyOS feedback request could not be encoded.' );
		}

		return $this->signed_post( '/v1/surface/feedback', $body_bytes );
	}

	/**
	 * Request a runtime-issued session nonce.
	 *
	 * @return array<string, mixed> Runtime nonce response.
	 */
	public function get_session_nonce(): array {
		return $this->signed_post( '/v1/surface/nonce/issue', '{}' );
	}

	/**
	 * Send an HMAC-signed JSON POST request.
	 *
	 * @param string $path Runtime path and optional query.
	 * @param string $body_bytes Exact body bytes to sign and send.
	 * @return array<string, mixed> Runtime response envelope.
	 */
	private function signed_post( string $path, string $body_bytes ): array {
		$credential = $this->credential_store->retrieve_session_key();
		$hmac_key   = $this->credential_store->retrieve_hmac_key();

		if ( null === $credential || null === $hmac_key ) {
			return $this->error_response( 'missing_session_key', 'DailyOS is not paired with an active runtime session.' );
		}

		$nonce      = $this->signer->generate_nonce();
		$timestamp  = $this->signer->current_timestamp();
		$signature  = $this->signer->sign_request(
			$hmac_key,
			'POST',
			$path,
			self::CONTENT_TYPE,
			$body_bytes,
			$nonce,
			$timestamp
		);
		$bearer     = $credential->bearer_token();
		$session_id = $credential->session_id();
		$url        = $this->runtime_url( $path );

		$response = wp_remote_post(
			$url,
			[
				'body'        => $body_bytes,
				'headers'     => [
					'Content-Type'        => self::CONTENT_TYPE,
					'Accept'              => self::CONTENT_TYPE,
					'Authorization'       => 'Bearer ' . $bearer,
					'X-DailyOS-Key-Id'    => $session_id,
					'X-DailyOS-Signature' => $signature,
					'X-DailyOS-Timestamp' => $timestamp,
					'X-DailyOS-Nonce'     => $nonce,
				],
				'redirection' => 0,
				'timeout'     => 5,
				'sslverify'   => false,
				'blocking'    => true,
				'data_format' => 'body',
			]
		);

		$parsed = $this->parse_response( $response );

		if ( true === ( $parsed['ok'] ?? false ) ) {
			$this->credential_store->update_last_use();
		}

		return $parsed;
	}

	/**
	 * Send an unsigned JSON POST request for pairing.
	 *
	 * @param string $path Runtime path.
	 * @param string $body_bytes Exact body bytes to send.
	 * @return array<string, mixed> Runtime response envelope.
	 */
	private function plain_post( string $path, string $body_bytes ): array {
		$response = wp_remote_post(
			$this->runtime_url( $path ),
			[
				'body'        => $body_bytes,
				'headers'     => [
					'Content-Type' => self::CONTENT_TYPE,
					'Accept'       => self::CONTENT_TYPE,
				],
				'redirection' => 0,
				'timeout'     => 5,
				'sslverify'   => false,
				'blocking'    => true,
				'data_format' => 'body',
			]
		);

		return $this->parse_response( $response );
	}

	/**
	 * JSON encode a payload for byte-exact request signing.
	 *
	 * @param array<string, mixed> $payload Payload.
	 * @return string|null JSON bytes, or null on failure.
	 */
	private function encode_json( array $payload ): ?string {
		$body_bytes = wp_json_encode( $payload, JSON_UNESCAPED_SLASHES | JSON_UNESCAPED_UNICODE );

		return is_string( $body_bytes ) ? $body_bytes : null;
	}

	/**
	 * Parse a WordPress HTTP API response into an envelope.
	 *
	 * @param mixed $response WordPress HTTP API response.
	 * @return array<string, mixed> Parsed response envelope.
	 */
	private function parse_response( mixed $response ): array {
		if ( is_wp_error( $response ) ) {
			return $this->error_response( 'runtime_request_failed', 'DailyOS runtime request failed.' );
		}

		$status_code = (int) wp_remote_retrieve_response_code( $response );
		$body        = (string) wp_remote_retrieve_body( $response );
		$decoded     = '' === $body ? [] : json_decode( $body, true );

		if ( '' !== $body && ! is_array( $decoded ) ) {
			return $this->error_response( 'runtime_invalid_json', 'DailyOS runtime returned invalid JSON.' );
		}

		$envelope = is_array( $decoded ) ? $decoded : [];

		if ( 200 > $status_code || 299 < $status_code ) {
			if ( ! isset( $envelope['error'] ) || ! is_array( $envelope['error'] ) ) {
				$envelope['error'] = [
					'code'    => 'runtime_http_error',
					'message' => 'DailyOS runtime returned an HTTP error.',
				];
			}

			$envelope['ok'] = false;
		}

		return $envelope;
	}

	/**
	 * Normalize the pairing handshake response shape.
	 *
	 * @param array<string, mixed> $response Raw response.
	 * @return array{ok: bool, runtime_instance_id: ?string, site_nonce_hash: ?string, projection_version: ?string, instance_id: ?string, session_id: ?string, scopes: array<int, string>, endpoint_version: ?string, error: ?array<string, mixed>} Handshake envelope.
	 */
	private function normalize_handshake_response( array $response ): array {
		$error  = isset( $response['error'] ) && is_array( $response['error'] ) ? $response['error'] : null;
		$ok     = true === ( $response['ok'] ?? false ) || (
			null === $error
			&& isset( $response['session_id'] )
			&& ( isset( $response['runtime_instance_id'] ) || isset( $response['instance_id'] ) )
		);
		$scopes = $response['scopes'] ?? $response['granted_scopes'] ?? [];

		return [
			'ok'                  => $ok,
			'runtime_instance_id' => isset( $response['runtime_instance_id'] ) ? (string) $response['runtime_instance_id'] : null,
			'site_nonce_hash'     => isset( $response['site_nonce_hash'] ) ? (string) $response['site_nonce_hash'] : null,
			'projection_version'  => isset( $response['projection_version'] ) ? (string) $response['projection_version'] : null,
			'instance_id'         => isset( $response['instance_id'] ) ? (string) $response['instance_id'] : null,
			'session_id'          => isset( $response['session_id'] ) ? (string) $response['session_id'] : null,
			'scopes'              => is_array( $scopes ) ? array_values( array_map( 'strval', $scopes ) ) : [],
			'endpoint_version'    => isset( $response['endpoint_version'] ) ? (string) $response['endpoint_version'] : null,
			'error'               => $error,
		];
	}

	/**
	 * Build a handshake error envelope.
	 *
	 * @param string $code Error code.
	 * @param string $message Error message.
	 * @return array{ok: bool, runtime_instance_id: ?string, site_nonce_hash: ?string, projection_version: ?string, instance_id: ?string, session_id: ?string, scopes: array<int, string>, endpoint_version: ?string, error: array<string, string>} Error envelope.
	 */
	private function handshake_error_envelope( string $code, string $message ): array {
		return [
			'ok'                  => false,
			'runtime_instance_id' => null,
			'site_nonce_hash'     => null,
			'projection_version'  => null,
			'instance_id'         => null,
			'session_id'          => null,
			'scopes'              => [],
			'endpoint_version'    => null,
			'error'               => [
				'code'    => $code,
				'message' => $message,
			],
		];
	}

	/**
	 * Build a generic error envelope.
	 *
	 * @param string $code Error code.
	 * @param string $message Error message.
	 * @return array<string, mixed> Error envelope.
	 */
	private function error_response( string $code, string $message ): array {
		return [
			'ok'    => false,
			'error' => [
				'code'    => $code,
				'message' => $message,
			],
		];
	}

	/**
	 * Build the full runtime URL for a path.
	 *
	 * @param string $path Runtime path.
	 * @return string Full URL.
	 */
	private function runtime_url( string $path ): string {
		return $this->runtime_base_url() . $path;
	}

	/**
	 * Return the runtime base URL from the gated WordPress filter.
	 *
	 * @return string Base URL.
	 */
	private function runtime_base_url(): string {
		$runtime_url = self::DEFAULT_RUNTIME_URL;

		if ( function_exists( 'current_user_can' ) && ! current_user_can( 'manage_options' ) ) {
			return $runtime_url;
		}

		$filtered_url = apply_filters( 'dailyos_wp_bridge_runtime_url', $runtime_url );

		if ( is_string( $filtered_url ) && '' !== trim( $filtered_url ) ) {
			$runtime_url = $filtered_url;
		}

		return rtrim( $runtime_url, '/' );
	}
}
