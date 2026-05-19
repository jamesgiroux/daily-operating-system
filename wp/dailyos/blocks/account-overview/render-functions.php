<?php
/**
 * Account overview block server-side render.
 *
 * Per W4-A V3 §6.3 / AC §13: every render calls the runtime through
 * `DailyOS_Runtime_Client::project_composition_for_surface(...)`. The
 * substrate side owns the cache and the scope-identity authority; PHP
 * never derives or inspects scopes, never invokes W4-D directly, and
 * never serializes the projection body into block attributes.
 *
 * @package DailyOS
 *
 * @var array<string, mixed> $attributes Block attributes (provided by core).
 * @var string               $content    Inner content (empty for dynamic blocks).
 */

declare(strict_types=1);

if ( ! defined( 'ABSPATH' ) ) {
	return '';
}

if ( ! function_exists( 'dailyos_account_overview_render' ) ) {
	/**
	 * Render the account-overview block from attributes. The wrapper keeps
	 * the block-registration render path's single runtime fetch behavior.
	 *
	 * @param array<string, mixed> $attributes Block attributes.
	 * @return string
	 */
	function dailyos_account_overview_render( array $attributes ): string {
		$response = dailyos_account_overview_fetch_projection( $attributes );
		return dailyos_account_overview_render_from_projection( $response, $attributes );
	}

	/**
	 * Fetch the projected composition for the wrapper render path.
	 *
	 * @param array<string, mixed> $attributes Block attributes.
	 * @return array<string, mixed>|\WP_Error|string Empty-state HTML when no fetch can be made.
	 */
	function dailyos_account_overview_fetch_projection( array $attributes ): array|\WP_Error|string {
		$composition_id      = isset( $attributes['composition_id'] ) ? (string) $attributes['composition_id'] : '';
		$composition_version = isset( $attributes['composition_version'] ) ? (int) $attributes['composition_version'] : 0;
		$cache_hint_token    = isset( $attributes['cache_hint_token'] ) ? (string) $attributes['cache_hint_token'] : '';

		if ( '' === $composition_id ) {
			return '<div class="wp-block-dailyos-account-overview is-empty">'
				. esc_html__( 'No account context to show here.', 'dailyos' )
				. '</div>';
		}

		$runtime_client = apply_filters( 'dailyos_runtime_client_for_block', null );
		// Duck-typed acceptance so PHPUnit can inject lightweight fakes
		// without subclassing the final transport class. Production code
		// passes a real DailyOS_Runtime_Client through the filter from
		// class-dailyos-plugin.php.
		if ( ! is_object( $runtime_client ) || ! method_exists( $runtime_client, 'project_composition_for_surface' ) ) {
			return '<div class="wp-block-dailyos-account-overview is-empty">'
				. esc_html__( 'No account context to show here.', 'dailyos' )
				. '</div>';
		}

		$cache_hint_param = '' !== $cache_hint_token ? $cache_hint_token : null;
		$response         = $runtime_client->project_composition_for_surface(
			$composition_id,
			$composition_version,
			$cache_hint_param
		);

		return $response;
	}

	/**
	 * Render the account-overview block from an already-fetched projection.
	 *
	 * @param array<string, mixed>|\WP_Error|string $response Runtime projection response or transport error.
	 * @param array<string, mixed>                  $attributes Block attributes.
	 * @return string
	 */
	function dailyos_account_overview_render_from_projection( array|\WP_Error|string $response, array $attributes ): string {
		if ( is_string( $response ) ) {
			return $response;
		}

		if ( is_wp_error( $response ) ) {
			return dailyos_account_overview_render_runtime_unavailable_notice();
		}

		if ( isset( $response['ok'] ) && false === $response['ok'] ) {
			$code = isset( $response['error']['code'] ) ? (string) $response['error']['code'] : 'runtime_request_failed';
			switch ( $code ) {
				case 'rate_limited':
				case 'transport_abuse_limited':
					return dailyos_account_overview_render_throttled_notice();
				// Session/pairing-repair-shaped codes — user reconnects from settings.
				case 'session_requires_repair':
				case 'session_not_found':
				case 'session_expired':
				case 'session_throttled':
				case 'session_invalid':
				case 'identity_mismatch':
				case 'wp_user_mismatch':
				case 'pairing_code_invalid':
				case 'pairing_code_expired':
				case 'pairing_code_consumed':
				case 'pairing_code_limited':
				case 'pairing_suspended':
				case 'pairing_revoked':
				case 'pairing_expired':
				case 'pairing_authority_unavailable':
				case 'site_binding_mismatch':
				case 'restored_stale_pairing':
				case 'unknown_runtime_anchor':
				case 'scope_denied':
				case 'auth_missing':
				case 'signature_invalid':
				case 'canonicalization_mismatch':
				case 'timestamp_stale':
				case 'timestamp_future':
				case 'key_not_found':
				case 'key_rotated':
				case 'token_invalid':
				case 'nonce_replay':
					return dailyos_account_overview_render_session_repair_notice();
				// Runtime-unavailable: transient infrastructure problem; retry.
				case 'runtime_unavailable':
				case 'runtime_request_failed':
				case 'runtime_invalid_json':
				case 'runtime_http_error':
				case 'host_invalid':
				case 'browser_origin_forbidden':
				case 'route_not_found':
					return dailyos_account_overview_render_runtime_unavailable_notice();
				// Renderer-input-invalid: defensive — runtime can't process the request shape.
				case 'request_body_too_large':
				case 'request_body_unreadable':
				case 'handshake_body_invalid':
				case 'session_refresh_body_invalid':
				case 'surface_invoke_invalid':
				case 'event_log_id_invalid':
				case 'project_composition_invalid':
				case 'project_composition_unknown_producer':
				case 'project_composition_invalid_id':
					return dailyos_account_overview_render_invalid_request_notice();
				// Projection-consistency failures — verification banner correct.
				case 'projection_tampered':
				case 'projection_version_rollback':
				case 'stale_composition_watermark':
				case 'missing_expected_claim_version':
				case 'mid_flight_mutation':
				case 'composition_version_overflow':
					return dailyos_account_overview_render_verification_banner();
				default:
					// Fail-safe — unknown code → verification banner. Operator
					// adds a typed mapping when a new code appears.
					return dailyos_account_overview_render_verification_banner();
			}
		}

		$projection = isset( $response['projection'] ) && is_array( $response['projection'] )
			? $response['projection']
			: null;
		if ( null === $projection ) {
			return dailyos_account_overview_render_verification_banner();
		}

		$composition_id          = isset( $attributes['composition_id'] ) ? (string) $attributes['composition_id'] : '';
		$composition_version     = isset( $attributes['composition_version'] ) ? (int) $attributes['composition_version'] : 0;
		$delivered_state         = function_exists( 'get_option' )
			? get_option( 'dailyos_composition_versions', [] )
			: [];
		$delivered_state_version = is_array( $delivered_state ) && isset( $delivered_state[ $composition_id ] )
			? (int) $delivered_state[ $composition_id ]
			: (int) ( $projection['composition_version'] ?? 0 );
		$is_stale                = $delivered_state_version > $composition_version;

		$wrapper_attrs = function_exists( 'get_block_wrapper_attributes' )
			? get_block_wrapper_attributes(
				[
					'class'                => 'wp-block-dailyos-account-overview',
					'data-ds-tier'         => 'pattern',
					'data-ds-name'         => 'AccountOverview',
					'data-dailyos-surface' => 'account_overview',
				]
			)
			: 'class="wp-block-dailyos-account-overview" data-dailyos-surface="account_overview"';

		$blocks = isset( $projection['blocks'] ) && is_array( $projection['blocks'] )
			? $projection['blocks']
			: [];

		$out = '<section ' . $wrapper_attrs . '>';

		if ( $is_stale ) {
			$out .= dailyos_account_overview_render_stale_banner();
		}

		foreach ( $blocks as $block ) {
			if ( ! is_array( $block ) ) {
				continue;
			}
			$out .= dailyos_account_overview_render_block(
				$block,
				[
					'composition_id'      => $composition_id,
					'composition_version' => $composition_version,
					'surface'             => 'account_overview',
				]
			);
		}

		if ( empty( $blocks ) ) {
			$out .= '<p class="dailyos-empty">' . esc_html__( 'No account context to show here.', 'dailyos' ) . '</p>';
		}

		$out .= '<hr class="dailyos-finis-marker" aria-hidden="true" />';
		$out .= '</section>';

		return $out;
	}

	/**
	 * Render a single projected block.
	 *
	 * @param array<string, mixed> $block Projected block payload.
	 * @param array<string, mixed> $render_context Optional render context carrying composition_id / composition_version / surface for feedback affordance mounting.
	 * @return string Rendered HTML.
	 */
	function dailyos_account_overview_render_block( array $block, array $render_context = [] ): string {
		// The runtime serializes ProjectedBlock with a structured shape:
		// the chosen rule lives under selected_known_type_id, the data the
		// producer emitted lives under payload, and trust_band is hoisted
		// to the top level. Read those rather than the flat block_type /
		// title / summary keys, which the runtime does not emit.
		$type_full = isset( $block['selected_known_type_id'] ) ? (string) $block['selected_known_type_id'] : '';
		if ( '' === $type_full && isset( $block['original_type_id'] ) ) {
			$type_full = (string) $block['original_type_id'];
		}
		$type    = '' !== $type_full ? (string) preg_replace( '#^dailyos/#', '', $type_full ) : 'unknown';
		$payload = isset( $block['payload'] ) && is_array( $block['payload'] ) ? $block['payload'] : [];

		$trust         = isset( $block['trust_band'] ) ? (string) $block['trust_band'] : 'needs_verification';
		$visible_bands = [ 'likely_current', 'use_with_caution', 'needs_verification' ];
		if ( ! in_array( $trust, $visible_bands, true ) ) {
			$trust = 'needs_verification';
		}

		// Header label. AccountOverview emits an explicit title; claim
		// blocks don't, so fall back to the block-type label.
		$label = isset( $payload['title'] ) && is_string( $payload['title'] )
			? (string) $payload['title']
			: ucfirst( str_replace( '_', ' ', $type ) );

		$claim_rows = dailyos_account_overview_claim_rows( $payload );
		$body       = dailyos_account_overview_body_text( $payload );

		$out  = '<article class="dailyos-block dailyos-block-' . esc_attr( $type ) . '">';
		$out .= '<header><h3>' . esc_html( $label ) . '</h3>';
		$out .= '<span data-ds-tier="primitive" data-ds-name="TrustBandBadge" data-ds-spec="primitives/TrustBandBadge.md" data-ds-trust-band="' . esc_attr( $trust ) . '">';
		$out .= esc_html( dailyos_trust_band_label( $trust ) );
		$out .= '</span></header>';

		if ( ! empty( $claim_rows ) ) {
			$out .= '<div class="dailyos-claim-list">';
			foreach ( $claim_rows as $row ) {
				$out .= dailyos_account_overview_render_claim_row( $row, $block, $render_context );
			}
			$out .= '</div>';
		} elseif ( '' !== $body ) {
			$out .= '<p>' . esc_html( $body ) . '</p>';
		}
		$out .= '</article>';
		return $out;
	}

	/**
	 * Extract renderable claim rows from known projected payload shapes.
	 *
	 * @param array<string, mixed> $payload Projected block payload.
	 * @return array<int, array<string, mixed>>
	 */
	function dailyos_account_overview_claim_rows( array $payload ): array {
		$rows = [];

		if ( isset( $payload['text'] ) && is_string( $payload['text'] ) && '' !== trim( $payload['text'] ) ) {
			$rows[] = dailyos_account_overview_claim_row_from_item( $payload, '/text' );
		}

		foreach ( [ 'items', 'nodes', 'context' ] as $collection_key ) {
			if ( ! isset( $payload[ $collection_key ] ) || ! is_array( $payload[ $collection_key ] ) ) {
				continue;
			}
			foreach ( $payload[ $collection_key ] as $index => $item ) {
				if ( ! is_array( $item ) || ! isset( $item['text'] ) || ! is_string( $item['text'] ) || '' === trim( $item['text'] ) ) {
					continue;
				}
				$rows[] = dailyos_account_overview_claim_row_from_item(
					$item,
					'/' . $collection_key . '/' . (int) $index . '/text'
				);
			}
		}

		return $rows;
	}

	/**
	 * Build one claim row from a payload item.
	 *
	 * @param array<string, mixed> $item Payload item.
	 * @param string               $field_path Payload field path.
	 * @return array<string, mixed>
	 */
	function dailyos_account_overview_claim_row_from_item( array $item, string $field_path ): array {
		$claim_id = isset( $item['claim_id'] ) && is_string( $item['claim_id'] ) ? (string) $item['claim_id'] : '';
		return [
			'text'          => isset( $item['text'] ) && is_string( $item['text'] ) ? (string) $item['text'] : '',
			'claim_id'      => $claim_id,
			'field_path'    => $field_path,
			'sources'       => dailyos_account_overview_sources_from_item( $item ),
			'invocation_id' => isset( $item['invocation_id'] ) && is_string( $item['invocation_id'] ) ? (string) $item['invocation_id'] : '',
		];
	}

	/**
	 * Preserve legacy body rendering for unknown payload variants.
	 *
	 * @param array<string, mixed> $payload Projected block payload.
	 * @return string
	 */
	function dailyos_account_overview_body_text( array $payload ): string {
		$parts = [];
		foreach ( dailyos_account_overview_claim_rows( $payload ) as $row ) {
			if ( isset( $row['text'] ) && is_string( $row['text'] ) ) {
				$parts[] = $row['text'];
			}
		}
		return implode( ' · ', $parts );
	}

	/**
	 * Render a claim row plus its client-side feedback affordance mount.
	 *
	 * @param array<string, mixed> $row Renderable claim row.
	 * @param array<string, mixed> $block Projected block payload.
	 * @param array<string, mixed> $render_context Block render context.
	 * @return string
	 */
	function dailyos_account_overview_render_claim_row( array $row, array $block, array $render_context ): string {
		$text      = isset( $row['text'] ) && is_string( $row['text'] ) ? (string) $row['text'] : '';
		$claim_ref = dailyos_account_overview_claim_ref_for_row( $row, $block );
		$claim_id  = isset( $row['claim_id'] ) && is_string( $row['claim_id'] ) ? (string) $row['claim_id'] : '';
		if ( '' === $claim_id && isset( $claim_ref['claim_id'] ) && is_string( $claim_ref['claim_id'] ) ) {
			$claim_id = (string) $claim_ref['claim_id'];
		}

		$out  = '<div class="dailyos-claim-row">';
		$out .= '<p class="dailyos-claim-text">' . esc_html( $text ) . '</p>';

		if ( '' !== $claim_id ) {
			$invocation_ids = dailyos_account_overview_invocation_ids_for_block( $block );
			$invocation_id  = isset( $row['invocation_id'] ) && is_string( $row['invocation_id'] ) && '' !== $row['invocation_id']
				? (string) $row['invocation_id']
				: ( $invocation_ids[0] ?? '' );

			// V4-W4 binding tuple: the runtime's IssueNonceRequest::parse
			// requires field_path + claim_version + composition_id +
			// composition_version on every nonce mint. claim_ref carries
			// per-claim values; composition_* comes from render_context.
			$claim_version       = isset( $claim_ref['claim_version'] ) ? (int) $claim_ref['claim_version'] : 0;
			$field_path          = isset( $row['field_path'] ) && is_string( $row['field_path'] )
				? (string) $row['field_path']
				: ( isset( $claim_ref['field_path'] ) && is_string( $claim_ref['field_path'] ) ? (string) $claim_ref['field_path'] : '' );
			$composition_id      = isset( $render_context['composition_id'] ) && is_string( $render_context['composition_id'] )
				? (string) $render_context['composition_id']
				: '';
			$composition_version = isset( $render_context['composition_version'] ) ? (int) $render_context['composition_version'] : 0;

			$out .= '<span class="dailyos-claim-feedback">';
			$out .= dailyos_account_overview_feedback_mount(
				[
					'claimId'            => $claim_id,
					'claimVersion'       => $claim_version,
					'fieldPath'          => $field_path,
					'compositionId'      => $composition_id,
					'compositionVersion' => $composition_version,
					'sources'            => isset( $row['sources'] ) && is_array( $row['sources'] ) ? $row['sources'] : [],
					'surface'            => isset( $render_context['surface'] ) && is_string( $render_context['surface'] ) ? (string) $render_context['surface'] : 'account_overview',
					'invocationId'       => $invocation_id,
					'invocationIds'      => $invocation_ids,
				]
			);
			$out .= '</span>';
		}

		$out .= '</div>';
		return $out;
	}

	/**
	 * Find the most specific claim ref for a row.
	 *
	 * @param array<string, mixed> $row Renderable claim row.
	 * @param array<string, mixed> $block Projected block.
	 * @return array<string, mixed>
	 */
	function dailyos_account_overview_claim_ref_for_row( array $row, array $block ): array {
		$claim_id   = isset( $row['claim_id'] ) && is_string( $row['claim_id'] ) ? (string) $row['claim_id'] : '';
		$claim_refs = isset( $block['claim_refs'] ) && is_array( $block['claim_refs'] ) ? $block['claim_refs'] : [];

		foreach ( $claim_refs as $claim_ref ) {
			if ( is_array( $claim_ref ) && '' !== $claim_id && isset( $claim_ref['claim_id'] ) && $claim_id === (string) $claim_ref['claim_id'] ) {
				return $claim_ref;
			}
		}

		if ( 1 === count( $claim_refs ) && isset( $claim_refs[0] ) && is_array( $claim_refs[0] ) ) {
			return $claim_refs[0];
		}

		return [];
	}

	/**
	 * Normalize row source references into the JS component contract.
	 *
	 * @param array<string, mixed> $item Payload item.
	 * @return array<int, array<string, string>>
	 */
	function dailyos_account_overview_sources_from_item( array $item ): array {
		foreach ( [ 'sources', 'source_refs', 'sourceRefs' ] as $key ) {
			if ( isset( $item[ $key ] ) && is_array( $item[ $key ] ) ) {
				return dailyos_account_overview_normalize_sources( $item[ $key ] );
			}
		}

		foreach ( [ 'source_ref', 'sourceRef', 'source_id', 'sourceId', 'source' ] as $key ) {
			if ( isset( $item[ $key ] ) && is_string( $item[ $key ] ) && '' !== trim( $item[ $key ] ) ) {
				return dailyos_account_overview_normalize_sources( [ (string) $item[ $key ] ] );
			}
		}

		return [];
	}

	/**
	 * Normalize arbitrary source arrays.
	 *
	 * @param array<mixed> $sources Raw source list.
	 * @return array<int, array<string, string>>
	 */
	function dailyos_account_overview_normalize_sources( array $sources ): array {
		$normalized = [];
		$values     = dailyos_account_overview_is_list( $sources ) ? $sources : [ $sources ];

		foreach ( $values as $index => $source ) {
			if ( is_string( $source ) && '' !== trim( $source ) ) {
				$normalized[] = [
					'id'         => (string) $source,
					'source_ref' => (string) $source,
					'label'      => (string) $source,
				];
				continue;
			}

			if ( ! is_array( $source ) ) {
				continue;
			}

			$source_ref = '';
			foreach ( [ 'source_ref', 'sourceRef', 'ref', 'id' ] as $key ) {
				if ( isset( $source[ $key ] ) && is_string( $source[ $key ] ) && '' !== trim( $source[ $key ] ) ) {
					$source_ref = (string) $source[ $key ];
					break;
				}
			}

			$label = '';
			foreach ( [ 'label', 'title', 'name' ] as $key ) {
				if ( isset( $source[ $key ] ) && is_string( $source[ $key ] ) && '' !== trim( $source[ $key ] ) ) {
					$label = (string) $source[ $key ];
					break;
				}
			}

			if ( '' === $source_ref && '' === $label ) {
				continue;
			}

			$normalized[] = [
				'id'         => '' !== $source_ref ? $source_ref : 'source-' . (int) $index,
				'source_ref' => $source_ref,
				'label'      => '' !== $label ? $label : $source_ref,
				'type'       => isset( $source['type'] ) && is_string( $source['type'] ) ? (string) $source['type'] : '',
			];
		}

		return $normalized;
	}

	/**
	 * Portable list check for supported PHP versions.
	 *
	 * @param array<mixed> $value Candidate array.
	 * @return bool
	 */
	function dailyos_account_overview_is_list( array $value ): bool {
		if ( [] === $value ) {
			return true;
		}
		return array_keys( $value ) === range( 0, count( $value ) - 1 );
	}

	/**
	 * Extract provenance invocation ids for invocation-scoped feedback.
	 *
	 * @param array<string, mixed> $block Projected block.
	 * @return array<int, string>
	 */
	function dailyos_account_overview_invocation_ids_for_block( array $block ): array {
		$ids        = [];
		$provenance = isset( $block['provenance'] ) && is_array( $block['provenance'] ) ? $block['provenance'] : [];

		foreach ( $provenance as $item ) {
			if ( is_array( $item ) && isset( $item['invocation_id'] ) && is_string( $item['invocation_id'] ) && '' !== trim( $item['invocation_id'] ) ) {
				$ids[] = (string) $item['invocation_id'];
			}
		}

		return array_values( array_unique( $ids ) );
	}

	/**
	 * Render the React feedback mount.
	 *
	 * @param array<string, mixed> $props Component props.
	 * @return string
	 */
	function dailyos_account_overview_feedback_mount( array $props ): string {
		$json = function_exists( 'wp_json_encode' ) ? wp_json_encode( $props ) : json_encode( $props );
		if ( ! is_string( $json ) || '' === $json ) {
			return '';
		}
		return '<span data-dailyos-feedback-affordance data-dailyos-feedback-props="' . esc_attr( $json ) . '"></span>';
	}

	/**
	 * Gets the display label for a trust band.
	 *
	 * @param string $band Trust band.
	 * @return string Trust-band label.
	 */
	function dailyos_trust_band_label( string $band ): string {
		switch ( $band ) {
			case 'likely_current':
				return __( 'Likely current', 'dailyos' );
			case 'use_with_caution':
				return __( 'Use with caution', 'dailyos' );
			case 'needs_verification':
			default:
				return __( 'Needs verification', 'dailyos' );
		}
	}

	/**
	 * Renders the stale report banner.
	 *
	 * @return string Rendered banner HTML.
	 */
	function dailyos_account_overview_render_stale_banner(): string {
		return '<aside data-ds-tier="pattern" data-ds-name="StaleReportBanner" data-ds-spec="patterns/StaleReportBanner.md" class="dailyos-stale-banner">'
			. '<p>' . esc_html__( 'Newer context has arrived. Refresh to bring this in.', 'dailyos' ) . '</p>'
			. '</aside>';
	}

	/**
	 * Renders a throttled runtime notice.
	 *
	 * @return string Rendered notice HTML.
	 */
	function dailyos_account_overview_render_throttled_notice(): string {
		return '<aside data-ds-tier="pattern" data-ds-name="RuntimeThrottledNotice" data-ds-spec="patterns/RuntimeNotice.md" class="dailyos-runtime-notice dailyos-runtime-notice-throttled">'
			. '<p>' . esc_html__( 'Runtime is throttling; retry shortly.', 'dailyos' ) . '</p>'
			. '</aside>';
	}

	/**
	 * Renders a session repair notice.
	 *
	 * @return string Rendered notice HTML.
	 */
	function dailyos_account_overview_render_session_repair_notice(): string {
		return '<aside data-ds-tier="pattern" data-ds-name="SurfaceSessionRepairNotice" data-ds-spec="patterns/RuntimeNotice.md" class="dailyos-runtime-notice dailyos-runtime-notice-session-repair">'
			. '<p>' . esc_html__( 'Surface session needs repair; reconnect from DailyOS settings.', 'dailyos' ) . '</p>'
			. '</aside>';
	}

	/**
	 * Renders a runtime unavailable notice.
	 *
	 * @return string Rendered notice HTML.
	 */
	function dailyos_account_overview_render_runtime_unavailable_notice(): string {
		return '<aside data-ds-tier="pattern" data-ds-name="RuntimeUnavailableNotice" data-ds-spec="patterns/RuntimeNotice.md" class="dailyos-runtime-notice dailyos-runtime-notice-unavailable">'
			. '<p>' . esc_html__( 'Runtime unavailable; retry.', 'dailyos' ) . '</p>'
			. '</aside>';
	}

	/**
	 * Renders an invalid request notice.
	 *
	 * @return string Rendered notice HTML.
	 */
	function dailyos_account_overview_render_invalid_request_notice(): string {
		return '<aside data-ds-tier="pattern" data-ds-name="InvalidRuntimeRequestNotice" data-ds-spec="patterns/RuntimeNotice.md" class="dailyos-runtime-notice dailyos-runtime-notice-invalid-request">'
			. '<p>' . esc_html__( "Editor sent a request the runtime couldn't process. Reload the editor.", 'dailyos' ) . '</p>'
			. '</aside>';
	}

	/**
	 * Renders the verification banner.
	 *
	 * @return string Rendered banner HTML.
	 */
	function dailyos_account_overview_render_verification_banner(): string {
		return '<aside data-ds-tier="pattern" data-ds-name="ConsistencyFindingBanner" data-ds-spec="patterns/ConsistencyFindingBanner.md" class="dailyos-verification-banner">'
			. '<p>' . esc_html__( "Something about this account doesn't line up. Verify before acting.", 'dailyos' ) . '</p>'
			. '</aside>';
	}
}
