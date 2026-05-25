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
	private const CONTENT_TYPE = 'application/json';

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
	 * @return array<string, mixed> Handshake envelope.
	 */
	public function handshake( string $pairing_code, array $wp_context ): array {
		$runtime_base_url = $this->runtime_base_url_for_pairing( $pairing_code );

		if ( null === $runtime_base_url ) {
			return $this->handshake_error_envelope( 'dailyos_not_paired', 'DailyOS pairing code did not include a loopback runtime port.' );
		}

		$body_bytes = $this->encode_json(
			array_merge(
				$wp_context,
				[
					'pairing_code' => $pairing_code,
				]
			)
		);

		if ( null === $body_bytes ) {
			return $this->handshake_error_envelope( 'json_encode_failed', 'DailyOS pairing request could not be encoded.' );
		}

		return $this->normalize_handshake_response(
			$this->plain_post( $runtime_base_url, '/v1/pairing/handshake', $body_bytes ),
			$runtime_base_url,
			$wp_context
		);
	}

	/**
	 * Invoke a scoped runtime ability.
	 *
	 * @param string               $name Ability name.
	 * @param array<string, mixed> $payload Ability payload.
	 * @param array<int, string>   $scope_set Requested scope set.
	 * @return array<string, mixed>|\WP_Error Runtime response envelope or typed pairing error.
	 */
	public function invoke_ability( string $name, array $payload, array $scope_set ): array|\WP_Error {
		unset( $scope_set );
		$payload = $this->normalize_ability_payload( $name, $payload );

		// Wire key is `input` per src-tauri/src/surface_runtime/mod.rs::SurfaceInvokeRequest.
		// Sending `payload` (the legacy key) gets dropped during deserialization
		// and invoke.input defaults to Value::Null, so the producer fails to
		// deserialize EntityIntelligenceInput. With the DOS-762 wire-code split
		// the bridge now maps that case to AbilityUnavailable → wire code
		// `ability_not_registered` (HTTP 404) rather than `auth_missing`.
		$body_bytes = $this->encode_json(
			[
				'ability' => $name,
				'input'   => $payload,
			]
		);

		if ( null === $body_bytes ) {
			return $this->error_response( 'json_encode_failed', 'DailyOS ability request could not be encoded.' );
		}

		return $this->local_post( '/v1/local/invoke', $body_bytes );
	}

	/**
	 * Read a sanitized markdown preview for an opaque workspace source handle.
	 *
	 * @param string $source_handle Opaque source handle from the workspace graph.
	 * @return array<string, mixed>|\WP_Error Runtime response with top-level `data`.
	 */
	public function read_markdown_preview( string $source_handle ): array|\WP_Error {
		$body_bytes = $this->encode_json(
			[
				'ability' => 'markdown_preview',
				'input'   => [
					'schemaVersion' => 1,
					'sourceHandle'  => $source_handle,
				],
			]
		);

		if ( null === $body_bytes ) {
			return $this->error_response( 'json_encode_failed', 'DailyOS markdown preview request could not be encoded.' );
		}

		return $this->normalize_surface_ability_response(
			$this->signed_post( '/v1/surface/invoke', $body_bytes )
		);
	}

	/**
	 * Read the source-management ledger for an entity-scoped workspace surface.
	 *
	 * @param string $entity_type Entity type.
	 * @param string $entity_id Entity identifier.
	 * @param int    $page_size Maximum rows to read.
	 * @return array<string, mixed>|\WP_Error Runtime response with top-level `data`.
	 */
	public function read_source_management_ledger( string $entity_type, string $entity_id, int $page_size = 25 ): array|\WP_Error {
		$body_bytes = $this->encode_json(
			[
				'ability' => 'source_management_ledger',
				'input'   => [
					'schemaVersion' => 1,
					'entityType'    => $entity_type,
					'entityId'      => $entity_id,
					'pageSize'      => max( 1, min( 100, $page_size ) ),
				],
			]
		);

		if ( null === $body_bytes ) {
			return $this->error_response( 'json_encode_failed', 'DailyOS source-management request could not be encoded.' );
		}

		return $this->normalize_surface_ability_response(
			$this->signed_post( '/v1/surface/invoke', $body_bytes )
		);
	}

	/**
	 * Apply a source-management action for an entity-scoped source.
	 *
	 * @param string $action Action key.
	 * @param string $entity_type Entity type.
	 * @param string $entity_id Entity identifier.
	 * @param string $source_key Opaque source key.
	 * @return array<string, mixed>|\WP_Error Runtime response with top-level `data`.
	 */
	public function apply_source_management_action( string $action, string $entity_type, string $entity_id, string $source_key ): array|\WP_Error {
		$body_bytes = $this->encode_json(
			[
				'ability' => 'source_management_action',
				'input'   => [
					'schemaVersion' => 1,
					'action'        => $action,
					'entityType'    => $entity_type,
					'entityId'      => $entity_id,
					'sourceKey'     => $source_key,
				],
			]
		);

		if ( null === $body_bytes ) {
			return $this->error_response( 'json_encode_failed', 'DailyOS source action request could not be encoded.' );
		}

		return $this->normalize_surface_ability_response(
			$this->signed_post( '/v1/surface/invoke', $body_bytes )
		);
	}

	/**
	 * Normalize a signed surface ability response for block render helpers.
	 *
	 * @param array<string, mixed>|\WP_Error $response Runtime response.
	 * @return array<string, mixed>|\WP_Error Response with `data` when present.
	 */
	private function normalize_surface_ability_response( array|\WP_Error $response ): array|\WP_Error {
		if ( is_wp_error( $response ) || ! is_array( $response ) ) {
			return $response;
		}

		if ( isset( $response['ability'] ) && is_array( $response['ability'] ) ) {
			$ability = $response['ability'];
			if ( isset( $ability['data'] ) && is_array( $ability['data'] ) ) {
				return [
					'ok'         => true === ( $response['ok'] ?? false ),
					'request_id' => $response['request_id'] ?? null,
					'ability'    => $ability,
					'data'       => $ability['data'],
				];
			}
		}

		return $response;
	}

	/**
	 * Normalize ability-specific wire payloads before JSON encoding.
	 *
	 * @param string               $name Ability name.
	 * @param array<string, mixed> $payload Ability payload.
	 * @return array<string, mixed> Runtime wire payload.
	 */
	private function normalize_ability_payload( string $name, array $payload ): array {
		if ( 'get_entity_intelligence' === $name ) {
			return $this->normalize_entity_intelligence_payload( $payload );
		}

		if ( 'claim_receipt' === $name ) {
			return $this->normalize_claim_receipt_payload( $payload );
		}

		return $payload;
	}

	/**
	 * Normalize the WordPress-side entity intelligence request to the Rust DTO.
	 *
	 * The runtime contract is `EntityIntelligenceInput`:
	 * `{ schemaVersion, entityType, entityId, depth, sections? }`.
	 * Older block code used no schema version and sent depth values like
	 * `Full`; the runtime now expects `deep` / `standard` / `shallow`.
	 *
	 * @param array<string, mixed> $payload Ability payload.
	 * @return array<string, mixed> Runtime wire payload.
	 */
	private function normalize_entity_intelligence_payload( array $payload ): array {
		if ( isset( $payload['schema_version'] ) && ! isset( $payload['schemaVersion'] ) ) {
			$payload['schemaVersion'] = $payload['schema_version'];
		}
		unset( $payload['schema_version'] );

		if ( ! isset( $payload['schemaVersion'] ) ) {
			$payload['schemaVersion'] = 1;
		}

		if ( array_key_exists( 'sections', $payload ) && null === $payload['sections'] ) {
			unset( $payload['sections'] );
		}

		if ( isset( $payload['entity_type'] ) && ! isset( $payload['entityType'] ) ) {
			$payload['entityType'] = $payload['entity_type'];
		}
		unset( $payload['entity_type'] );

		if ( isset( $payload['entity_id'] ) && ! isset( $payload['entityId'] ) ) {
			$payload['entityId'] = $payload['entity_id'];
		}
		unset( $payload['entity_id'] );

		if ( ! isset( $payload['depth'] ) || ! is_scalar( $payload['depth'] ) || '' === trim( (string) $payload['depth'] ) ) {
			$payload['depth'] = 'deep';
			return $payload;
		}

		$depth = strtolower( trim( (string) $payload['depth'] ) );
		if ( in_array( $depth, [ 'full', 'deep' ], true ) ) {
			$payload['depth'] = 'deep';
		} elseif ( in_array( $depth, [ 'default', 'standard' ], true ) ) {
			$payload['depth'] = 'standard';
		} elseif ( 'shallow' === $depth ) {
			$payload['depth'] = 'shallow';
		}

		return $payload;
	}

	/**
	 * Normalize a WordPress claim receipt request to the Rust DTO.
	 *
	 * Runtime contract:
	 * `{ schemaVersion, target: { kind: "claim", claimId, subject, fieldPath? }, surface }`.
	 * WordPress envelope items and stored claim rows use `{ kind, id }` subject
	 * refs, while the ability schema expects the serde shape for `SubjectRef`
	 * (`{ account: "acct-test-001" }`, `{ person: "person-test-001" }`, etc.).
	 *
	 * @param array<string, mixed> $payload Ability payload.
	 * @return array<string, mixed> Runtime wire payload.
	 */
	private function normalize_claim_receipt_payload( array $payload ): array {
		if ( isset( $payload['schema_version'] ) && ! isset( $payload['schemaVersion'] ) ) {
			$payload['schemaVersion'] = $payload['schema_version'];
		}
		unset( $payload['schema_version'] );

		if ( ! isset( $payload['schemaVersion'] ) ) {
			$payload['schemaVersion'] = 1;
		}

		if ( isset( $payload['target'] ) && is_array( $payload['target'] ) ) {
			$target = $payload['target'];
		} else {
			$claim_id = $payload['claimId'] ?? $payload['claim_id'] ?? '';
			$target   = [
				'kind'    => 'claim',
				'claimId' => $claim_id,
				'subject' => $payload['subject'] ?? $payload['subjectRef'] ?? $payload['subject_ref'] ?? null,
			];
			$field_path = $payload['fieldPath'] ?? $payload['field_path'] ?? null;
			if ( null !== $field_path && '' !== (string) $field_path ) {
				$target['fieldPath'] = $field_path;
			}
		}

		$payload['target']  = $this->normalize_claim_receipt_target( $target );
		$payload['surface'] = $this->normalize_claim_receipt_surface( $payload['surface'] ?? 'entity_detail' );

		unset(
			$payload['claimId'],
			$payload['claim_id'],
			$payload['subject'],
			$payload['subjectRef'],
			$payload['subject_ref'],
			$payload['fieldPath'],
			$payload['field_path']
		);

		return $payload;
	}

	/**
	 * Normalize a claim receipt target.
	 *
	 * @param array<string, mixed> $target Raw target.
	 * @return array<string, mixed> Runtime target.
	 */
	private function normalize_claim_receipt_target( array $target ): array {
		if ( isset( $target['claim_id'] ) && ! isset( $target['claimId'] ) ) {
			$target['claimId'] = $target['claim_id'];
		}
		unset( $target['claim_id'] );

		if ( isset( $target['field_path'] ) && ! isset( $target['fieldPath'] ) ) {
			$target['fieldPath'] = $target['field_path'];
		}
		unset( $target['field_path'] );

		if ( isset( $target['subject_ref'] ) && ! isset( $target['subject'] ) ) {
			$target['subject'] = $target['subject_ref'];
		}
		if ( isset( $target['subjectRef'] ) && ! isset( $target['subject'] ) ) {
			$target['subject'] = $target['subjectRef'];
		}
		unset( $target['subject_ref'], $target['subjectRef'] );

		$target['kind'] = $this->normalize_claim_receipt_target_kind( $target['kind'] ?? 'claim' );
		if ( array_key_exists( 'subject', $target ) ) {
			$target['subject'] = $this->normalize_subject_ref( $target['subject'] );
		}

		return $target;
	}

	/**
	 * Normalize the claim receipt target tag.
	 *
	 * @param mixed $kind Raw kind.
	 * @return string Runtime target kind.
	 */
	private function normalize_claim_receipt_target_kind( mixed $kind ): string {
		$value = strtolower( trim( (string) $kind ) );
		return match ( $value ) {
			'proposal' => 'proposal',
			'work_item', 'workitem', 'work-item' => 'workItem',
			default => 'claim',
		};
	}

	/**
	 * Normalize the receipt surface enum.
	 *
	 * @param mixed $surface Raw surface.
	 * @return string Runtime surface.
	 */
	private function normalize_claim_receipt_surface( mixed $surface ): string {
		$value = strtolower( trim( (string) $surface ) );
		return match ( $value ) {
			'actionswork', 'actions-work', 'action', 'actions' => 'actions_work',
			'entitydetail', 'entity-detail', 'tauri_entity_detail' => 'entity_detail',
			'dailybriefing', 'daily-briefing', 'briefing', 'briefing_prep' => 'daily_briefing',
			'meetingdetail', 'meeting-detail', 'tauri_meeting_detail' => 'meeting_detail',
			'mcptool', 'mcp-tool', 'mcp_tool' => 'mcp',
			default => '' !== $value ? $value : 'entity_detail',
		};
	}

	/**
	 * Normalize WordPress/DB subject refs to the serde SubjectRef wire shape.
	 *
	 * @param mixed $subject_ref Raw subject ref.
	 * @return mixed Runtime subject ref.
	 */
	private function normalize_subject_ref( mixed $subject_ref ): mixed {
		if ( is_string( $subject_ref ) ) {
			$value = strtolower( trim( $subject_ref ) );
			if ( in_array( $value, [ 'global', 'unknown' ], true ) ) {
				return $value;
			}
			return $subject_ref;
		}

		if ( ! is_array( $subject_ref ) ) {
			return $subject_ref;
		}

		foreach ( [ 'account', 'project', 'person', 'meeting', 'user', 'multi' ] as $serde_key ) {
			if ( array_key_exists( $serde_key, $subject_ref ) ) {
				if ( 'multi' !== $serde_key || ! is_array( $subject_ref[ $serde_key ] ) ) {
					return $subject_ref;
				}
				return [
					'multi' => array_map(
						fn ( mixed $item ): mixed => $this->normalize_subject_ref( $item ),
						$subject_ref['multi']
					),
				];
			}
		}

		$kind = strtolower( trim( (string) ( $subject_ref['kind'] ?? '' ) ) );
		$id   = $subject_ref['id'] ?? null;
		if ( in_array( $kind, [ 'global', 'unknown' ], true ) ) {
			return $kind;
		}
		if ( 'multi' === $kind && isset( $subject_ref['subjects'] ) && is_array( $subject_ref['subjects'] ) ) {
			return [
				'multi' => array_map(
					fn ( mixed $item ): mixed => $this->normalize_subject_ref( $item ),
					$subject_ref['subjects']
				),
			];
		}
		if ( null === $id || '' === (string) $id ) {
			return $subject_ref;
		}

		return match ( $kind ) {
			'account', 'accounts' => [ 'account' => (string) $id ],
			'project', 'projects' => [ 'project' => (string) $id ],
			'person', 'people' => [ 'person' => (string) $id ],
			'meeting', 'meetings' => [ 'meeting' => (string) $id ],
			'user', 'users' => [ 'user' => (string) $id ],
			default => $subject_ref,
		};
	}


	/**
	 * Request a scope-filtered projected composition for a WordPress render.
	 *
	 * Calls POST /v1/local/project-composition. The substrate side
	 * orchestrates: cache lookup → producer ability invocation → W4-D
	 * projection → cache store → response. The block surface receives only
	 * the scope-filtered ProjectedComposition DTO plus an opaque
	 * cache_hint_token (advisory; never interpreted by PHP).
	 *
	 * @param string      $composition_id      Composition identifier (UUID/string from block attribute).
	 * @param int         $composition_version Expected composition version watermark.
	 * @param string|null $cache_hint_token    Opaque runtime cache hint from a prior render.
	 * @return array<string, mixed>|\WP_Error Response envelope with `projection`, `cache_hint_token`, `served_from_cache`.
	 */
	public function project_composition_for_surface(
		string $composition_id,
		int $composition_version,
		?string $cache_hint_token = null
	): array|\WP_Error {
		$payload = [
			'composition_id'      => $composition_id,
			'composition_version' => $composition_version,
		];
		if ( null !== $cache_hint_token && '' !== $cache_hint_token ) {
			$payload['cache_hint_token'] = $cache_hint_token;
		}

		$body_bytes = $this->encode_json( $payload );
		if ( null === $body_bytes ) {
			return $this->error_response(
				'json_encode_failed',
				'DailyOS project_composition request could not be encoded.'
			);
		}

		return $this->local_post( '/v1/local/project-composition', $body_bytes );
	}

	/**
	 * Request a runtime-issued user-presence nonce.
	 *
	 * @param array<string, mixed> $payload Nonce binding tuple.
	 * @return array<string, mixed>|\WP_Error Runtime nonce response or typed pairing error.
	 */
	public function issue_nonce( array $payload ): array|\WP_Error {
		$body_bytes = $this->encode_json( $payload );

		if ( null === $body_bytes ) {
			return $this->error_response( 'json_encode_failed', 'DailyOS nonce request could not be encoded.' );
		}

		// dormant: only signed-path callers remain for the legacy presence-nonce contract.
		return $this->signed_post( '/v1/surface/nonce/issue', $body_bytes );
	}

	/**
	 * Verify and consume a runtime-issued user-presence nonce.
	 *
	 * @param array<string, mixed> $payload Nonce binding tuple and nonce token.
	 * @return array<string, mixed>|\WP_Error Runtime verify response or typed pairing error.
	 */
	public function verify_nonce( array $payload ): array|\WP_Error {
		$body_bytes = $this->encode_json( $payload );

		if ( null === $body_bytes ) {
			return $this->error_response( 'json_encode_failed', 'DailyOS nonce verify request could not be encoded.' );
		}

		// dormant: only signed-path callers remain for the legacy presence-nonce contract.
		return $this->signed_post( '/v1/surface/nonce/verify', $body_bytes );
	}

	/**
	 * Request a runtime-issued session nonce.
	 *
	 * @param array<string, mixed> $payload Nonce binding tuple.
	 * @return array<string, mixed>|\WP_Error Runtime nonce response or typed pairing error.
	 */
	public function get_session_nonce( array $payload = [] ): array|\WP_Error {
		return $this->issue_nonce( $payload );
	}

	/**
	 * Re-grant scopes (union with the runtime's current DEFAULT_GRANTED_SCOPES)
	 * to this paired SurfaceClient without rotating the HMAC session key or
	 * forcing a re-pair .
	 *
	 * @return array<string, mixed>|\WP_Error Refresh envelope or typed pairing error.
	 */
	public function refresh_pairing_scopes(): array|\WP_Error {
		$body_bytes = $this->encode_json( [ 'request_id' => wp_generate_uuid4() ] );
		if ( null === $body_bytes ) {
			return $this->error_response(
				'json_encode_failed',
				'DailyOS refresh-scopes request could not be encoded.'
			);
		}

		// dormant: only signed-path callers remain for pairing-maintenance refreshes.
		return $this->signed_post( '/v1/surface/pairing/refresh-scopes', $body_bytes );
	}


	/**
	 * Send an HMAC-signed JSON POST request.
	 *
	 * @param string $path Runtime path and optional query.
	 * @param string $body_bytes Exact body bytes to sign and send.
	 * @return array<string, mixed>|\WP_Error Runtime response envelope or typed pairing error.
	 */
	private function signed_post( string $path, string $body_bytes ): array|\WP_Error {
		$marker = $this->credential_store->get_marker();

		if ( null === $marker ) {
			return $this->not_paired_error();
		}

		$runtime_base_url = $this->discover_runtime_base_url( $marker );

		if ( null === $runtime_base_url ) {
			return $this->not_paired_error();
		}

		$identity = $this->canonical_identity( $marker );

		if ( null === $identity ) {
			return $this->not_paired_error();
		}

		$credential = $this->credential_store->retrieve_session_key();
		$hmac_key   = $this->credential_store->retrieve_hmac_key();

		if ( null === $credential || null === $hmac_key ) {
			return $this->error_response( 'missing_session_key', 'DailyOS is not paired with an active runtime session.' );
		}

		$nonce      = $this->signer->generate_nonce();
		$timestamp  = $this->signer->current_timestamp();
		$request_id = wp_generate_uuid4();
		$signature  = $this->signer->sign_request(
			$hmac_key,
			'POST',
			$path,
			self::CONTENT_TYPE,
			$body_bytes,
			$identity,
			$nonce,
			$timestamp,
			$request_id
		);
		$session_id = $credential->session_id();
		$url        = $this->runtime_url( $runtime_base_url, $path );
		$headers    = [
			'Content-Type'                   => self::CONTENT_TYPE,
			'Accept'                         => self::CONTENT_TYPE,
			'X-DailyOS-Session-Id'           => $session_id,
			'X-DailyOS-SurfaceClient'        => $this->surface_client_id( $marker ),
			'X-DailyOS-Key-Id'               => $session_id,
			'X-DailyOS-Signature'            => $signature,
			'X-DailyOS-Timestamp'            => $timestamp,
			'X-DailyOS-Nonce'                => $nonce,
			'X-DailyOS-Request-Id'           => $request_id,
			'X-DailyOS-Site-Binding-Digest'  => $identity['site_binding_digest'],
			'X-DailyOS-Site-Nonce'           => $identity['site_nonce'],
			'X-DailyOS-WP-User-Id'           => $identity['wp_user_id'],
			'X-DailyOS-WP-Site-Id'           => $identity['wp_site_id'],
			'X-DailyOS-Home-Url'             => $identity['home_url'],
			'X-DailyOS-Site-Url'             => $identity['site_url'],
			'X-DailyOS-WP-Install-UUID'      => $identity['wp_install_uuid'],
			'X-DailyOS-Plugin-Instance-UUID' => $identity['plugin_instance_uuid'],
		];

		if ( '' !== $identity['multisite_blog_id'] ) {
			$headers['X-DailyOS-Multisite-Blog-Id'] = $identity['multisite_blog_id'];
		}

		// Cold-cache producer commits + writer-mutex contention from background
		// workers can take >5s in local-to-local deployments. The original 5s
		// timeout was sized for a remote-shaped expectation; local renders
		// against a busy substrate DB reliably exceed it.
		$post_args = [
			'body'        => $body_bytes,
			'headers'     => $headers,
			'redirection' => 0,
			'timeout'     => 90,
			'sslverify'   => false,
			'blocking'    => true,
			'data_format' => 'body',
		];

		$response = wp_remote_post( $url, $post_args );

		// On ECONNREFUSED, invalidate the sentinel cache, re-discover, and
		// retry the request once. The retry fires whether or not the URL
		// changed — the runtime may have restarted on the same port, or the
		// original failure may be transient. A previous "only retry if URL
		// differs" guard missed same-port transient refusals.
		if ( self::is_connection_refused( $response ) ) {
			\DailyOS\DailyOS_Plugin::invalidate_runtime_endpoint_cache();
			$retry_base_url = $this->discover_runtime_base_url( $marker );
			if ( null !== $retry_base_url ) {
				$response = wp_remote_post( $this->runtime_url( $retry_base_url, $path ), $post_args );
			}
		}

		$parsed = $this->parse_response( $response );

		if ( true === ( $parsed['ok'] ?? false ) ) {
			$this->credential_store->update_last_use();
		}

		return $parsed;
	}

	/**
	 * Send a first-party local loopback JSON POST request.
	 *
	 * @param string $path Runtime local path.
	 * @param string $body_bytes Exact body bytes to send.
	 * @return array<string, mixed>|\WP_Error Runtime response envelope or typed pairing error.
	 */
	private function local_post( string $path, string $body_bytes ): array|\WP_Error {
		$marker = $this->credential_store->get_marker();

		if ( null === $marker ) {
			return $this->not_paired_error();
		}

		$runtime_base_url = $this->discover_runtime_base_url( $marker );

		if ( null === $runtime_base_url ) {
			return $this->not_paired_error();
		}

		$request_id = wp_generate_uuid4();
		$url        = $this->runtime_url( $runtime_base_url, $path );
		$post_args  = [
			'body'        => $body_bytes,
			'headers'     => [
				'Content-Type'          => self::CONTENT_TYPE,
				'X-DailyOS-Request-Id' => $request_id,
			],
			'redirection' => 0,
			'timeout'     => 90,
			'sslverify'   => false,
			'blocking'    => true,
			'data_format' => 'body',
		];

		$response = wp_remote_post( $url, $post_args );

		if ( self::is_connection_refused( $response ) ) {
			\DailyOS\DailyOS_Plugin::invalidate_runtime_endpoint_cache();
			$retry_base_url = $this->discover_runtime_base_url( $marker );
			if ( null !== $retry_base_url ) {
				$response = wp_remote_post( $this->runtime_url( $retry_base_url, $path ), $post_args );
			}
		}

		$parsed = $this->parse_response( $response );

		if ( true === ( $parsed['ok'] ?? false ) ) {
			$this->credential_store->update_last_use();
		}

		return $parsed;
	}

	/**
	 * Detect ECONNREFUSED in a wp_remote_post response. Used by the retry
	 * path to invalidate the sentinel cache, re-discover, and retry once.
	 *
	 * @param array|\WP_Error $response wp_remote_post return value.
	 */
	private static function is_connection_refused( $response ): bool {
		if ( ! is_wp_error( $response ) ) {
			return false;
		}
		$message = strtolower( (string) $response->get_error_message() );
		return false !== strpos( $message, 'connection refused' )
			|| false !== strpos( $message, 'econnrefused' );
	}

	/**
	 * Send an unsigned JSON POST request for pairing.
	 *
	 * @param string $runtime_base_url Runtime base URL.
	 * @param string $path Runtime path.
	 * @param string $body_bytes Exact body bytes to send.
	 * @return array<string, mixed> Runtime response envelope.
	 */
	private function plain_post( string $runtime_base_url, string $path, string $body_bytes ): array {
		$response = wp_remote_post(
			$this->runtime_url( $runtime_base_url, $path ),
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
			if ( ! array_key_exists( 'error', $envelope ) ) {
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
	 * @param string               $fallback_runtime_url Runtime URL derived from pairing code.
	 * @param array<string, mixed> $wp_context WordPress pairing context.
	 * @return array<string, mixed> Handshake envelope.
	 */
	private function normalize_handshake_response( array $response, string $fallback_runtime_url, array $wp_context ): array {
		$payload         = isset( $response['pairing'] ) && is_array( $response['pairing'] )
			? $response['pairing']
			: $response;
		$error           = isset( $response['error'] ) && is_array( $response['error'] ) ? $response['error'] : null;
		$ok              = true === ( $response['ok'] ?? false ) || (
			null === $error
			&& $this->has_field( $payload, 'session_id', 'sessionId' )
			&& (
				$this->has_field( $payload, 'runtime_instance_id', 'runtimeInstanceId' )
				|| $this->has_field( $payload, 'instance_id', 'instanceId' )
				|| $this->has_field( $payload, 'surface_client_id', 'surfaceClientId' )
			)
		);
		$scopes          = $payload['scopes'] ?? $payload['granted_scopes'] ?? $payload['grantedScopes'] ?? [];
		$site_nonce_full = $this->string_field( $payload, 'site_nonce', 'siteNonce' );
		$runtime_url     = $this->runtime_url_from_handshake_payload( $payload ) ?? $fallback_runtime_url;

		return [
			'ok'                   => $ok,
			'runtime_instance_id'  => $this->string_field( $payload, 'runtime_instance_id', 'runtimeInstanceId' )
				?? $this->string_field( $payload, 'surface_client_id', 'surfaceClientId' ),
			'surface_client_id'    => $this->string_field( $payload, 'surface_client_id', 'surfaceClientId' ),
			'runtime_url'          => $runtime_url,
			'site_nonce_hash'      => $this->string_field( $payload, 'site_nonce_hash', 'siteNonceHash' )
				?? ( null === $site_nonce_full ? null : hash( 'sha256', $site_nonce_full ) ),
			'site_nonce_full'      => $site_nonce_full,
			'site_binding_digest'  => $this->string_field( $payload, 'site_binding_digest', 'siteBindingDigest' ),
			'wp_site_id'           => isset( $wp_context['wp_site_id'] ) ? (string) $wp_context['wp_site_id'] : null,
			'wp_install_uuid'      => isset( $wp_context['wp_install_uuid'] ) ? (string) $wp_context['wp_install_uuid'] : null,
			'plugin_instance_uuid' => isset( $wp_context['plugin_instance_uuid'] ) ? (string) $wp_context['plugin_instance_uuid'] : null,
			'projection_version'   => $this->string_field( $payload, 'projection_version', 'projectionVersion' ),
			'instance_id'          => $this->string_field( $payload, 'instance_id', 'instanceId' ),
			'session_id'           => $this->string_field( $payload, 'session_id', 'sessionId' ),
			'scopes'               => is_array( $scopes ) ? array_values( array_map( 'strval', $scopes ) ) : [],
			'endpoint_version'     => $this->string_field( $payload, 'endpoint_version', 'endpointVersion' ),
			'error'                => $error,
		];
	}

	/**
	 * Build a handshake error envelope.
	 *
	 * @param string $code Error code.
	 * @param string $message Error message.
	 * @return array<string, mixed> Error envelope.
	 */
	private function handshake_error_envelope( string $code, string $message ): array {
		return [
			'ok'                   => false,
			'runtime_instance_id'  => null,
			'surface_client_id'    => null,
			'runtime_url'          => null,
			'site_nonce_hash'      => null,
			'site_nonce_full'      => null,
			'site_binding_digest'  => null,
			'wp_site_id'           => null,
			'wp_install_uuid'      => null,
			'plugin_instance_uuid' => null,
			'projection_version'   => null,
			'instance_id'          => null,
			'session_id'           => null,
			'scopes'               => [],
			'endpoint_version'     => null,
			'error'                => [
				'code'    => $code,
				'message' => $message,
			],
		];
	}

	/**
	 * Build a typed not-paired error.
	 */
	private function not_paired_error(): \WP_Error {
		return new \WP_Error(
			'dailyos_not_paired',
			__( 'DailyOS is not paired with an active loopback runtime. Pair this site before making runtime requests.', 'dailyos' )
		);
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
	 * @param string $runtime_base_url Runtime base URL.
	 * @param string $path Runtime path.
	 * @return string Full URL.
	 */
	private function runtime_url( string $runtime_base_url, string $path ): string {
		return $runtime_base_url . $path;
	}

	/**
	 * Discover the loopback runtime base URL from sentinel, marker, and gated filter.
	 *
	 * @param array<string, mixed> $marker Pairing marker.
	 * @return string|null Base URL, or null when not paired.
	 */
	private function discover_runtime_base_url( array $marker ): ?string {
		// Prefer the sentinel-discovered URL (current runtime port across
		// restarts) over the stored marker (may be stale after the runtime
		// restarts on a new port). Signed callers still authenticate at the
		// route; local callers only need this shared discovery result.
		$sentinel_url = \DailyOS\DailyOS_Plugin::discover_runtime_base_url();

		$marker_url = isset( $marker['runtime_url'] ) ? self::normalize_loopback_runtime_url( (string) $marker['runtime_url'] ) : null;

		// Admin filter override path retained — admins can still pin a specific URL
		// for dev/testing. Sentinel is the runtime-tracked default; marker is the
		// post-pairing baseline; filter overrides both for power users.
		if ( function_exists( 'current_user_can' ) && ! current_user_can( 'manage_options' ) ) {
			return $sentinel_url ?? $marker_url;
		}

		$filter_seed  = $sentinel_url ?? ( $marker_url ?? '' );
		$filtered_url = apply_filters( 'dailyos_wp_bridge_runtime_url', $filter_seed );

		if ( is_string( $filtered_url ) && '' !== trim( $filtered_url ) ) {
			$normalized_filtered_url = self::normalize_loopback_runtime_url( $filtered_url );

			if ( null !== $normalized_filtered_url ) {
				return $normalized_filtered_url;
			}

			$this->log_invalid_runtime_url_override();
		}

		return $sentinel_url ?? $marker_url;
	}

	/**
	 * Resolve the runtime URL to handshake against.
	 *
	 * Accepts either a full `dailyos://pair?port=N&code=...` URL (port read
	 * from the query string) or a bare pairing code (e.g. `K8XJ-2M4N-9PQR`,
	 * port discovered via the sentinel file the runtime writes on bind).
	 *
	 * @param string $pairing_code Pairing code or DailyOS pairing URL.
	 */
	private function runtime_base_url_for_pairing( string $pairing_code ): ?string {
		$query = wp_parse_url( trim( $pairing_code ), PHP_URL_QUERY );

		if ( is_string( $query ) ) {
			parse_str( $query, $parts );
			if ( isset( $parts['port'] ) && is_scalar( $parts['port'] ) ) {
				return self::normalize_loopback_runtime_url( 'http://127.0.0.1:' . (string) $parts['port'] );
			}
		}

		// Bare code: port comes from the runtime sentinel (~/.dailyos/runtime-endpoint.json).
		if ( class_exists( '\\DailyOS\\DailyOS_Plugin' ) ) {
			$endpoint = \DailyOS\DailyOS_Plugin::discover_runtime_endpoint();
			if ( is_array( $endpoint ) && isset( $endpoint['port'] ) && is_int( $endpoint['port'] ) ) {
				return self::normalize_loopback_runtime_url( 'http://127.0.0.1:' . (string) $endpoint['port'] );
			}
		}

		return null;
	}

	/**
	 * Return the canonical identity fields for signed requests.
	 *
	 * @param array<string, mixed> $marker Pairing marker.
	 * @return array<string, string>|null Identity fields, or null when marker is incomplete.
	 */
	private function canonical_identity( array $marker ): ?array {
		$site_binding_digest  = isset( $marker['site_binding_digest'] ) ? (string) $marker['site_binding_digest'] : '';
		$site_nonce           = isset( $marker['site_nonce_full'] ) ? (string) $marker['site_nonce_full'] : '';
		$wp_install_uuid      = isset( $marker['wp_install_uuid'] ) ? (string) $marker['wp_install_uuid'] : '';
		$plugin_instance_uuid = isset( $marker['plugin_instance_uuid'] ) ? (string) $marker['plugin_instance_uuid'] : '';

		if ( '' === $site_binding_digest || '' === $site_nonce || '' === $wp_install_uuid || '' === $plugin_instance_uuid ) {
			return null;
		}

		// The runtime binds wp_user_hash at pairing time; subsequent signed requests
		// must present the same wp_user_id or the runtime suspends the pairing as
		// a wp_user mismatch. MCP invocations run as the substrate user (not the
		// admin who paired), so we read the paired wp_user_id from the marker and
		// only fall back to current_user_id for installs that paired before this
		// field was tracked.
		$paired_wp_user_id = isset( $marker['paired_wp_user_id'] )
			? (string) $marker['paired_wp_user_id']
			: '';
		if ( '' === $paired_wp_user_id ) {
			$paired_wp_user_id = function_exists( 'get_current_user_id' )
				? (string) get_current_user_id()
				: '0';
		}

		return [
			'site_binding_digest'  => $site_binding_digest,
			'site_nonce'           => $site_nonce,
			'wp_user_id'           => $paired_wp_user_id,
			'wp_site_id'           => $this->wp_site_id( $marker, $wp_install_uuid ),
			'home_url'             => function_exists( 'home_url' ) ? home_url() : '',
			'site_url'             => function_exists( 'site_url' ) ? site_url() : '',
			'wp_install_uuid'      => $wp_install_uuid,
			'plugin_instance_uuid' => $plugin_instance_uuid,
			'multisite_blog_id'    => $this->multisite_blog_id(),
		];
	}

	/**
	 * Return the signed surface client identifier.
	 *
	 * @param array<string, mixed> $marker Pairing marker.
	 */
	private function surface_client_id( array $marker ): string {
		$surface_client_id = isset( $marker['surface_client_id'] ) ? (string) $marker['surface_client_id'] : '';

		return '' === $surface_client_id && isset( $marker['runtime_instance_id'] )
			? (string) $marker['runtime_instance_id']
			: $surface_client_id;
	}

	/**
	 * Return the stable WordPress site ID claim.
	 *
	 * @param array<string, mixed> $marker Pairing marker.
	 * @param string               $wp_install_uuid Stable DailyOS install UUID.
	 */
	private function wp_site_id( array $marker, string $wp_install_uuid ): string {
		if ( isset( $marker['wp_site_id'] ) && '' !== (string) $marker['wp_site_id'] ) {
			return (string) $marker['wp_site_id'];
		}

		return $wp_install_uuid . ':' . $this->current_blog_id();
	}

	/**
	 * Return the current blog ID as a string.
	 */
	private function current_blog_id(): string {
		return function_exists( 'get_current_blog_id' ) ? (string) get_current_blog_id() : '1';
	}

	/**
	 * Return the multisite blog ID claim when applicable.
	 */
	private function multisite_blog_id(): string {
		if ( function_exists( 'is_multisite' ) && is_multisite() ) {
			return $this->current_blog_id();
		}

		return '';
	}

	/**
	 * Extract a runtime URL from a pairing handshake payload.
	 *
	 * @param array<string, mixed> $payload Handshake payload.
	 */
	private function runtime_url_from_handshake_payload( array $payload ): ?string {
		$runtime_url = $this->string_field( $payload, 'runtime_url', 'runtimeUrl' );

		if ( null !== $runtime_url ) {
			return self::normalize_loopback_runtime_url( $runtime_url );
		}

		$runtime_port = $payload['runtime_port'] ?? $payload['runtimePort'] ?? null;

		if ( is_scalar( $runtime_port ) ) {
			return self::normalize_loopback_runtime_url( 'http://127.0.0.1:' . (string) $runtime_port );
		}

		$bound_addr = $this->string_field( $payload, 'bound_addr', 'boundAddr' );

		if ( null !== $bound_addr ) {
			$candidate = str_starts_with( $bound_addr, 'http://' ) ? $bound_addr : 'http://' . $bound_addr;

			return self::normalize_loopback_runtime_url( $candidate );
		}

		return null;
	}

	/**
	 * Validate and normalize a loopback runtime base URL.
	 *
	 * @param string $runtime_url Runtime URL candidate.
	 */
	private static function normalize_loopback_runtime_url( string $runtime_url ): ?string {
		$parts = wp_parse_url( trim( $runtime_url ) );

		if ( ! is_array( $parts ) ) {
			return null;
		}

		$scheme    = isset( $parts['scheme'] ) ? strtolower( (string) $parts['scheme'] ) : '';
		$host      = isset( $parts['host'] ) ? strtolower( (string) $parts['host'] ) : '';
		$port      = isset( $parts['port'] ) ? (int) $parts['port'] : 0;
		$path      = isset( $parts['path'] ) ? (string) $parts['path'] : '';
		$has_extra = isset( $parts['query'] ) || isset( $parts['fragment'] ) || ( '' !== $path && '/' !== $path );

		if ( 'http' !== $scheme || '127.0.0.1' !== $host || 1 > $port || 65535 < $port || $has_extra ) {
			return null;
		}

		return 'http://127.0.0.1:' . $port;
	}

	/**
	 * Return whether any candidate field exists in an array.
	 *
	 * @param array<string, mixed> $payload Payload.
	 * @param string               ...$keys Candidate keys.
	 */
	private function has_field( array $payload, string ...$keys ): bool {
		foreach ( $keys as $key ) {
			if ( array_key_exists( $key, $payload ) ) {
				return true;
			}
		}

		return false;
	}

	/**
	 * Return the first scalar string field from an array.
	 *
	 * @param array<string, mixed> $payload Payload.
	 * @param string               ...$keys Candidate keys.
	 */
	private function string_field( array $payload, string ...$keys ): ?string {
		foreach ( $keys as $key ) {
			if ( isset( $payload[ $key ] ) && is_scalar( $payload[ $key ] ) ) {
				return (string) $payload[ $key ];
			}
		}

		return null;
	}

	/**
	 * Log an invalid runtime URL override without including the URL value.
	 */
	private function log_invalid_runtime_url_override(): void {
		// phpcs:ignore WordPress.PHP.DevelopmentFunctions.error_log_error_log -- Security-relevant invalid local override is logged without user data.
		error_log( 'DailyOS ignored an invalid runtime URL override.' );
	}
}
