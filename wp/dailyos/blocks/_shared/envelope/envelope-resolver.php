<?php
/**
 * Shared envelope resolver — per-request DOS-477 envelope cache shim.
 *
 * Per L0 packet W2 V1.2.1 §5.1 "envelopeHandle resolution contract":
 *
 *   1. The outer entity-detail block invokes get_entity_intelligence ONCE per
 *      render through the runtime client (the 3-arg invoke_ability signature
 *      enforced by check_w1_consumer_skeleton.sh).
 *   2. The producer-side DOS-477 envelope_cache keys the result by
 *      (envelope_render_id, actor_principal_id, surface).
 *   3. The outer block emits dailyos/envelopeHandle into block context.
 *   4. Inner blocks declare usesContext: ["dailyos/envelopeHandle"] and call
 *      dailyos_resolve_envelope(...) below, which short-circuits via the
 *      request-scoped cache to the response already paid for upstream.
 *
 * This file is the PHP-side request-scoped cache (lifetime = single PHP
 * request; evicts on response close). DOS-477 owns the producer-side cache.
 * The two together satisfy "envelope fetched once per request" per V1.2.1.
 *
 * @package DailyOS
 */

declare(strict_types=1);

if ( ! defined( 'ABSPATH' ) ) {
	return;
}

if ( ! function_exists( 'dailyos_envelope_cache_get' ) ) {
	/**
	 * Read a cached envelope by handle. Returns null on miss.
	 *
	 * @param string $handle Envelope handle (server-emitted envelope_render_id).
	 * @return array<string,mixed>|null
	 */
	function dailyos_envelope_cache_get( string $handle ): ?array {
		if ( '' === $handle ) {
			return null;
		}
		static $cache = [];
		if ( isset( $cache[ $handle ] ) ) {
			return $cache[ $handle ];
		}
		$shared = dailyos_envelope_cache_storage();
		return $shared[ $handle ] ?? null;
	}
}

if ( ! function_exists( 'dailyos_envelope_cache_put' ) ) {
	/**
	 * Store an envelope by handle for the remainder of the request.
	 *
	 * @param string               $handle Envelope handle.
	 * @param array<string,mixed>  $envelope Envelope payload (full
	 *                                       EntityIntelligenceEnvelope shape).
	 */
	function dailyos_envelope_cache_put( string $handle, array $envelope ): void {
		if ( '' === $handle ) {
			return;
		}
		$shared             = &dailyos_envelope_cache_storage();
		$shared[ $handle ]  = $envelope;
		dailyos_envelope_claim_item_cache_put_from_envelope( $envelope );
	}
}

if ( ! function_exists( 'dailyos_envelope_cache_storage' ) ) {
	/**
	 * Backing storage for the request-scoped envelope cache. Returned by
	 * reference so put can mutate.
	 *
	 * @return array<string,array<string,mixed>>
	 */
	function &dailyos_envelope_cache_storage(): array {
		static $storage = [];
		return $storage;
	}
}

if ( ! function_exists( 'dailyos_envelope_claim_item_cache_get' ) ) {
	/**
	 * Read an indexed claim-backed envelope item by claim id.
	 *
	 * @param string $claim_id Claim identifier.
	 * @return array<string,mixed>|null
	 */
	function dailyos_envelope_claim_item_cache_get( string $claim_id ): ?array {
		if ( '' === $claim_id ) {
			return null;
		}
		$storage = dailyos_envelope_claim_item_cache_storage();
		return $storage[ $claim_id ] ?? null;
	}
}

if ( ! function_exists( 'dailyos_envelope_claim_item_cache_put_from_envelope' ) ) {
	/**
	 * Index claim-backed envelope items so copied inner-block selectors that
	 * only pass claim ids can recover producer-rendered text without receipt
	 * fan-out.
	 *
	 * @param array<string,mixed> $envelope Cached EntityIntelligenceEnvelope.
	 */
	function dailyos_envelope_claim_item_cache_put_from_envelope( array $envelope ): void {
		$storage = &dailyos_envelope_claim_item_cache_storage();
		dailyos_envelope_claim_item_cache_walk( $envelope, $storage );
	}

	/**
	 * Recursively index claim items from an envelope subtree.
	 *
	 * @param mixed                            $node Envelope subtree.
	 * @param array<string,array<string,mixed>> $storage Cache storage.
	 */
	function dailyos_envelope_claim_item_cache_walk( mixed $node, array &$storage ): void {
		if ( ! is_array( $node ) ) {
			return;
		}
		$claim_id = $node['claimId'] ?? $node['claim_id'] ?? null;
		if ( is_scalar( $claim_id ) && '' !== (string) $claim_id ) {
			$existing          = $storage[ (string) $claim_id ] ?? null;
			$node_has_text     = '' !== dailyos_receipt_rendered_text( $node, '' );
			$existing_has_text = is_array( $existing ) && '' !== dailyos_receipt_rendered_text( $existing, '' );
			if ( ! is_array( $existing ) || ( $node_has_text && ! $existing_has_text ) ) {
				$storage[ (string) $claim_id ] = $node;
			}
		}
		foreach ( $node as $child ) {
			if ( is_array( $child ) ) {
				dailyos_envelope_claim_item_cache_walk( $child, $storage );
			}
		}
	}
}

if ( ! function_exists( 'dailyos_envelope_claim_item_cache_storage' ) ) {
	/**
	 * Backing storage for claim item lookup.
	 *
	 * @return array<string,array<string,mixed>>
	 */
	function &dailyos_envelope_claim_item_cache_storage(): array {
		static $storage = [];
		return $storage;
	}
}

if ( ! function_exists( 'dailyos_envelope_handle_from_response' ) ) {
	/**
	 * Extract the envelope_render_id (or fall back to a deterministic hash)
	 * from a get_entity_intelligence response and cache the envelope under it.
	 *
	 * @param array<string,mixed> $response Runtime client response (decoded).
	 * @param string              $entity_type Entity kind (account/project/...).
	 * @param string              $entity_id   Entity id.
	 * @return string Envelope handle (empty string on degenerate input).
	 */
	function dailyos_envelope_handle_from_response( array $response, string $entity_type, string $entity_id ): string {
		// Runtime returns { ok, request_id, ability: { ability_name, data, ... } }
		// per src-tauri/src/bridges/types.rs::AbilityResponseJson — the actual
		// envelope lives at $response['ability']['data']. The 'envelope' and
		// 'data' top-level fallbacks remain for legacy callers that pre-unwrap.
		$ability  = $response['ability'] ?? null;
		$envelope = $response['envelope']
			?? $response['data']
			?? ( is_array( $ability ) ? ( $ability['data'] ?? null ) : null )
			?? $response;
		if ( ! is_array( $envelope ) ) {
			return '';
		}
		$envelope = dailyos_envelope_normalize_surface_payload( $envelope );
		$handle = '';
		if ( isset( $envelope['envelopeRenderId'] ) && is_string( $envelope['envelopeRenderId'] ) ) {
			$handle = $envelope['envelopeRenderId'];
		} elseif ( isset( $envelope['envelope_render_id'] ) && is_string( $envelope['envelope_render_id'] ) ) {
			$handle = $envelope['envelope_render_id'];
		} else {
			// Fall back to a deterministic per-envelope handle so inner blocks
			// can still resolve via the request-scoped cache. Substrate carries
			// envelope_render_id once DOS-477 envelope-cache wire-up surfaces
			// it on the response; until then this hash keeps the consumer side
			// single-fetch.
			//
			// W1W2 L2 cycle-2 MEDIUM fix: the previous fallback used
			// `spl_object_hash( (object) $envelope )` which casts the array to
			// a FRESH stdClass on every call. PHP allocates a new object per
			// call, so the object-hash differs each invocation — the outer
			// block emitted one handle, the inner block (re-deriving from the
			// same envelope shape) got a DIFFERENT handle, and the cache
			// missed. Net effect: every inner block re-invoked the producer
			// instead of reusing the cached envelope, multiplying ability
			// invocations N-times per render.
			//
			// `md5( serialize( $envelope ) )` is deterministic on the
			// envelope's data shape (associative arrays serialize in insertion
			// order, identical envelopes hash identically), and the cost is
			// dominated by the serialize() of an already-in-memory array —
			// negligible compared to the saved ability round-trip.
			$handle = hash(
				'sha256',
				$entity_type . '|' . $entity_id . '|' . md5( serialize( $envelope ) )
			);
		}
		dailyos_envelope_cache_put( $handle, $envelope );
		return $handle;
	}
}

if ( ! function_exists( 'dailyos_envelope_normalize_surface_payload' ) ) {
	/**
	 * Normalize runtime DTO aliases to the section keys used by WP inner blocks.
	 *
	 * @param array<string,mixed> $envelope EntityIntelligenceEnvelope payload.
	 * @return array<string,mixed>
	 */
	function dailyos_envelope_normalize_surface_payload( array $envelope ): array {
		$aliases = [
			'recordEntries'     => [ 'record', 'record_entries' ],
			'metadataProposals' => [ 'metadata_proposals' ],
			'openLoops'         => [ 'open_loops' ],
			'healthStory'       => [ 'health_story' ],
		];
		foreach ( $aliases as $source_key => $target_keys ) {
			if ( ! array_key_exists( $source_key, $envelope ) ) {
				continue;
			}
			foreach ( $target_keys as $target_key ) {
				if ( ! array_key_exists( $target_key, $envelope ) ) {
					$envelope[ $target_key ] = $envelope[ $source_key ];
				}
			}
		}
		return $envelope;
	}
}

if ( ! function_exists( 'dailyos_resolve_envelope' ) ) {
	/**
	 * Resolve an envelope for an inner block. Hits the request-scoped cache
	 * by handle first; on miss invokes the producer with the 3-arg signature
	 * (which itself short-circuits via the DOS-477 producer cache when keyed
	 * the same way). Returns null when no envelope is reachable — inner
	 * blocks render empty-state chips per §10 invariant.
	 *
	 * @param string|null $handle     Envelope handle from block context.
	 * @param string      $entity_type Entity kind (account/project/person/meeting).
	 * @param string      $entity_id   Entity id from block context.
	 * @param array       $scope_set   Resolved scope set for the surface client.
	 * @return array<string,mixed>|null
	 */
	function dailyos_resolve_envelope( ?string $handle, string $entity_type, string $entity_id, array $scope_set ): ?array {
		if ( null !== $handle && '' !== $handle ) {
			$cached = dailyos_envelope_cache_get( $handle );
			if ( null !== $cached ) {
				return $cached;
			}
		}
		if ( '' === $entity_id ) {
			return null;
		}
		$runtime_client = apply_filters( 'dailyos_runtime_client_for_block', null );
		if ( ! is_object( $runtime_client ) || ! method_exists( $runtime_client, 'invoke_ability' ) ) {
			return null;
		}
		$response = $runtime_client->invoke_ability(
			'get_entity_intelligence',
			[
				'entity_type' => $entity_type,
				'entity_id'   => $entity_id,
				'depth'       => 'Full',
				'sections'    => null,
			],
			$scope_set
		);
		if ( ! is_array( $response ) ) {
			return null;
		}
		$resolved_handle = dailyos_envelope_handle_from_response( $response, $entity_type, $entity_id );
		if ( '' === $resolved_handle ) {
			return null;
		}
		return dailyos_envelope_cache_get( $resolved_handle );
	}
}

if ( ! function_exists( 'dailyos_envelope_section' ) ) {
	/**
	 * Return a surface-runtime wire reason when the cached envelope is an
	 * error envelope rather than a sectioned ability payload.
	 *
	 * @param array<string,mixed>|null $envelope Envelope payload.
	 * @return string
	 */
	function dailyos_envelope_surface_wire_reason( ?array $envelope ): string {
		if ( null === $envelope ) {
			return '';
		}
		$error = $envelope['error'] ?? null;
		$code  = is_array( $error ) ? (string) ( $error['code'] ?? '' ) : '';
		if ( '' === $code && isset( $envelope['code'] ) ) {
			$code = (string) $envelope['code'];
		}
		$known_reasons = [
			'ability_not_registered' => true,
			'input_schema_invalid'   => true,
			'producer_unavailable'   => true,
			'ownership_denied'       => true,
		];
		return isset( $known_reasons[ $code ] ) ? $code : '';
	}

	/**
	 * Lookup a named section in the envelope's sections map, return a flat
	 * descriptor: [ 'present' => bool, 'item_count' => int, 'reason' => string ].
	 *
	 * @param array<string,mixed>|null $envelope Envelope payload.
	 * @param string                   $section Section key (snake_case).
	 * @return array{present:bool,item_count:int,reason:string}
	 */
	function dailyos_envelope_section( ?array $envelope, string $section ): array {
		$wire_reason = dailyos_envelope_surface_wire_reason( $envelope );
		$default = [
			'present'    => false,
			'item_count' => 0,
			'reason'     => '' !== $wire_reason ? $wire_reason : 'not_available',
		];
		if ( null === $envelope ) {
			return $default;
		}
		$sections = $envelope['sections'] ?? [];
		if ( ! is_array( $sections ) ) {
			return $default;
		}
		$state = $sections[ $section ] ?? null;
		if ( ! is_array( $state ) ) {
			return $default;
		}
		$kind = $state['kind'] ?? '';
		if ( 'present' === $kind ) {
			return [
				'present'    => true,
				'item_count' => (int) ( $state['item_count'] ?? $state['itemCount'] ?? 0 ),
				'reason'     => '',
			];
		}
		$reason = '';
		if ( isset( $state['reason'] ) ) {
			$reason = is_array( $state['reason'] )
				? (string) ( $state['reason']['kind'] ?? 'unknown' )
				: (string) $state['reason'];
		}
		return [
			'present'    => false,
			'item_count' => 0,
			'reason'     => '' === $reason ? 'empty' : $reason,
		];
	}
}

if ( ! function_exists( 'dailyos_empty_chip' ) ) {
	/**
	 * Human-readable labels for runtime wire reasons.
	 *
	 * @param string $reason Empty-state reason.
	 * @param string $fallback Caller-provided label.
	 * @return string
	 */
	function dailyos_empty_reason_label( string $reason, string $fallback ): string {
		switch ( $reason ) {
			case 'ability_not_registered':
				return __( 'Ability not registered', 'dailyos' );
			case 'input_schema_invalid':
				return __( 'Input schema invalid', 'dailyos' );
			case 'producer_unavailable':
				return __( 'Producer unavailable', 'dailyos' );
			case 'ownership_denied':
				return __( 'Ownership denied', 'dailyos' );
			default:
				return $fallback;
		}
	}

	/**
	 * Render the canonical empty-state chip mandated by V1.1 §10 invariant
	 * "every inner block renders empty as quiet chip with data-empty-reason;
	 * no silent hidden states; inherits envelope Empty { reason }".
	 *
	 * @param string $reason Empty-state reason (snake_case).
	 * @param string $label  Localized human label.
	 * @param string $block_class Block-scoped CSS class (e.g.,
	 *                            "dailyos-stakeholder-grid").
	 * @return string
	 */
	function dailyos_empty_chip( string $reason, string $label, string $block_class ): string {
		$safe_reason = esc_attr( '' === $reason ? 'empty' : $reason );
		$safe_label  = esc_html( dailyos_empty_reason_label( $reason, $label ) );
		$safe_class  = esc_attr( $block_class );
		return '<div class="' . $safe_class . ' ' . $safe_class . '--empty">'
			. '<span class="dailyos-empty-chip" data-empty-reason="' . $safe_reason . '">'
			. $safe_label
			. '</span>'
			. '</div>';
	}
}

if ( ! function_exists( 'dailyos_inner_block_wrapper_attrs' ) ) {
	/**
	 * Build wrapper attrs for an inner block. Per V1.1 §10 invariant
	 * "check_no_inline_style_exception.sh", the only permitted inline-style
	 * value is a `--dailyos-*: var(--dailyos-*);` custom-property assignment.
	 * This helper returns either the standard wrapper attrs or, when the
	 * outer block context carries a CSS custom property (e.g.,
	 * --dailyos-project-tint), forwards it onto inner blocks via
	 * get_block_wrapper_attributes — never raw inline styles.
	 *
	 * @param string                 $block_class Block-scoped CSS class.
	 * @param array<string,string>   $custom_properties Optional inline-style
	 *                                                 custom-property pairs.
	 * @return string Rendered attribute string suitable for echo.
	 */
	function dailyos_inner_block_wrapper_attrs( string $block_class, array $custom_properties = [] ): string {
		$args = [
			'class' => $block_class,
		];
		if ( ! empty( $custom_properties ) ) {
			$style_segments = [];
			foreach ( $custom_properties as $key => $value ) {
				if ( 0 !== strpos( $key, '--dailyos-' ) ) {
					continue; // discipline per §10 inline-style allowlist
				}
				$style_segments[] = $key . ': ' . $value;
			}
			if ( ! empty( $style_segments ) ) {
				$args['style'] = implode( '; ', $style_segments );
			}
		}
		if ( function_exists( 'get_block_wrapper_attributes' ) ) {
			return get_block_wrapper_attributes( $args );
		}
		$out = 'class="' . esc_attr( $args['class'] ) . '"';
		if ( isset( $args['style'] ) ) {
			$out .= ' style="' . esc_attr( $args['style'] ) . '"';
		}
		return $out;
	}
}

if ( ! function_exists( 'dailyos_envelope_consume_claim' ) ) {
	/**
	 * Pass a claim reference through claim_receipt via the runtime client
	 * (AgentMcp audience filter is enforced server-side by
	 * build_receipt_for_audience per DOS-341). Returns the receipt payload
	 * or null on failure / unavailable runtime. Inner blocks invoke this
	 * for the rendered claim-row receipts called out in AC-462.3.
	 *
	 * @param array<string,mixed> $claim_ref { claim_id, audience_key, ... }.
	 * @param array<string|int,mixed> $scope_set Resolved scope set.
	 * @return array<string,mixed>|null
	 */
	function dailyos_envelope_consume_claim( array $claim_ref, array $scope_set ): ?array {
		// get_entity_intelligence now carries display-safe renderedText on
		// claim-backed envelope rows. Use that first so WordPress does not
		// fan out into dozens of claim_receipt calls and trip loopback limits.
		if ( '' !== dailyos_receipt_rendered_text( $claim_ref, '' ) ) {
			return $claim_ref;
		}

		$claim_id = isset( $claim_ref['claim_id'] ) ? (string) $claim_ref['claim_id'] : '';
		if ( '' !== $claim_id ) {
			$cached_item = dailyos_envelope_claim_item_cache_get( $claim_id );
			if ( is_array( $cached_item ) ) {
				$claim_ref = array_merge( $cached_item, $claim_ref );
				if ( '' !== dailyos_receipt_rendered_text( $claim_ref, '' ) ) {
					return $claim_ref;
				}
			}
		}

		$runtime_client = apply_filters( 'dailyos_runtime_client_for_block', null );
		if ( ! is_object( $runtime_client ) || ! method_exists( $runtime_client, 'invoke_ability' ) ) {
			return null;
		}

		// Shape the claim_ref into the registered claim_receipt ability's
		// input contract: { schemaVersion, target: { kind, claimId, ... },
		// surface }. Without this shaping the ability decoder rejects the
		// payload as a contract violation rather than rendering the receipt.
		if ( '' === $claim_id ) {
			return null;
		}
		$subject_ref = $claim_ref['subject_ref'] ?? null;
		if ( ! is_array( $subject_ref ) ) {
			return null;
		}
		$target = [
			'kind'    => 'claim',
			'claimId' => $claim_id,
			'subject' => $subject_ref,
		];
		if ( ! empty( $claim_ref['field_path'] ) ) {
			$target['fieldPath'] = (string) $claim_ref['field_path'];
		}
		// Envelope readers run inside the WP block render path, which
		// corresponds to entity_detail for the account-detail / project-detail
		// inner blocks unless the caller overrides via claim_ref['surface'].
		$surface = isset( $claim_ref['surface'] ) ? (string) $claim_ref['surface'] : 'entity_detail';
		$payload = [
			'schemaVersion' => 1,
			'target'        => $target,
			'surface'       => $surface,
		];

		$response = $runtime_client->invoke_ability( 'claim_receipt', $payload, $scope_set );
		if ( ! is_array( $response ) ) {
			return null;
		}

		return '' !== dailyos_receipt_rendered_text( $response, '' ) ? $response : null;
	}
}

if ( ! function_exists( 'dailyos_receipt_payload' ) ) {
	/**
	 * Unwrap a claim_receipt ability response into its ClaimReceiptSnapshot.
	 *
	 * @param array<string,mixed> $receipt Raw runtime response or unwrapped receipt.
	 * @return array<string,mixed>
	 */
	function dailyos_receipt_payload( array $receipt ): array {
		$ability = $receipt['ability'] ?? null;
		$data    = is_array( $ability ) ? ( $ability['data'] ?? null ) : null;
		if ( is_array( $data ) ) {
			return $data;
		}
		if ( isset( $receipt['data'] ) && is_array( $receipt['data'] ) ) {
			return $receipt['data'];
		}
		if ( isset( $receipt['receipt'] ) && is_array( $receipt['receipt'] ) ) {
			return $receipt['receipt'];
		}
		return $receipt;
	}

	/**
	 * Read display-safe claim text from a receipt.
	 *
	 * @param array<string,mixed> $receipt Raw runtime response or unwrapped receipt.
	 * @param string              $fallback Fallback when no rendered text is available.
	 * @return string
	 */
	function dailyos_receipt_rendered_text( array $receipt, string $fallback = '' ): string {
		$payload       = dailyos_receipt_payload( $receipt );
		$rendered_text = $payload['renderedText'] ?? $payload['rendered_text'] ?? null;
		if ( is_array( $rendered_text ) ) {
			$text = $rendered_text['text'] ?? '';
			return is_string( $text ) && '' !== trim( $text ) ? $text : $fallback;
		}
		if ( is_string( $rendered_text ) && '' !== trim( $rendered_text ) ) {
			return $rendered_text;
		}
		$text = $payload['text'] ?? '';
		return is_string( $text ) && '' !== trim( $text ) ? $text : $fallback;
	}

	/**
	 * Read trust band from a receipt.
	 *
	 * @param array<string,mixed> $receipt Raw runtime response or unwrapped receipt.
	 * @return string
	 */
	function dailyos_receipt_trust_band( array $receipt ): string {
		$payload = dailyos_receipt_payload( $receipt );
		$trust   = $payload['trust'] ?? null;
		if ( is_array( $trust ) ) {
			$band = $trust['band'] ?? '';
			if ( is_string( $band ) && '' !== $band ) {
				return $band;
			}
		}
		$band = $payload['trustBand'] ?? $payload['trust_band'] ?? '';
		return is_string( $band ) && '' !== $band ? $band : 'unscored';
	}
}

if ( ! function_exists( 'dailyos_envelope_collect_claim_refs' ) ) {
	/**
	 * Collect claim refs from envelope sections with optional claim/field filters.
	 *
	 * @param array<string,mixed>|null $envelope Envelope payload.
	 * @param array<int,string>        $section_keys Section keys to scan.
	 * @param array<string,mixed>      $filters Optional claim_types,
	 *                                          field_paths,
	 *                                          field_path_prefixes.
	 * @return array<int,array<string,mixed>>
	 */
	function dailyos_envelope_collect_claim_refs( ?array $envelope, array $section_keys, array $filters = [] ): array {
		if ( null === $envelope ) {
			return [];
		}
		$refs = [];
		foreach ( $section_keys as $section_key ) {
			$slice = $envelope[ $section_key ] ?? [];
			if ( ! is_array( $slice ) ) {
				continue;
			}
			$items = $slice['items'] ?? ( is_array( reset( $slice ) ) ? $slice : [] );
			if ( ! is_array( $items ) ) {
				continue;
			}
			foreach ( $items as $item ) {
				if ( ! is_array( $item ) || ! dailyos_envelope_claim_ref_matches( $item, $filters ) ) {
					continue;
				}
				$claim_id = $item['claimId'] ?? $item['claim_id'] ?? '';
				if ( '' === $claim_id ) {
					continue;
				}
				$ref = [
					'claim_id'     => (string) $claim_id,
					'audience_key' => $item['audienceKey'] ?? $item['audience_key'] ?? 'user',
					'subject_ref'  => $item['subjectRef'] ?? $item['subject_ref'] ?? null,
					'field_path'   => $item['fieldPath'] ?? $item['field_path'] ?? null,
				];
				foreach ( [ 'renderedText', 'rendered_text', 'text', 'trust', 'trustBand', 'trust_band', 'provenance', 'sensitivity' ] as $key ) {
					if ( array_key_exists( $key, $item ) ) {
						$ref[ $key ] = $item[ $key ];
					}
				}
				$refs[] = $ref;
			}
		}
		return $refs;
	}

	/**
	 * Match an envelope item against selector filters.
	 *
	 * @param array<string,mixed> $item Envelope item.
	 * @param array<string,mixed> $filters Filter map.
	 * @return bool
	 */
	function dailyos_envelope_claim_ref_matches( array $item, array $filters ): bool {
		$claim_type = (string) ( $item['claimType'] ?? $item['claim_type'] ?? '' );
		$field_path = (string) ( $item['fieldPath'] ?? $item['field_path'] ?? '' );

		if ( isset( $filters['claim_types'] ) && is_array( $filters['claim_types'] ) && ! in_array( $claim_type, $filters['claim_types'], true ) ) {
			return false;
		}
		$has_field_filter = false;
		if ( isset( $filters['field_paths'] ) && is_array( $filters['field_paths'] ) && in_array( $field_path, $filters['field_paths'], true ) ) {
			return true;
		}
		$has_field_filter = $has_field_filter || ( isset( $filters['field_paths'] ) && is_array( $filters['field_paths'] ) && ! empty( $filters['field_paths'] ) );
		if ( isset( $filters['field_path_prefixes'] ) && is_array( $filters['field_path_prefixes'] ) ) {
			foreach ( $filters['field_path_prefixes'] as $prefix ) {
				if ( is_string( $prefix ) && 0 === strpos( $field_path, $prefix ) ) {
					return true;
				}
			}
			$has_field_filter = $has_field_filter || ! empty( $filters['field_path_prefixes'] );
		}
		return ! $has_field_filter;
	}
}
