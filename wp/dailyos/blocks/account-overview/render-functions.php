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

		$composition_id      = isset( $attributes['composition_id'] ) ? (string) $attributes['composition_id'] : '';
		$composition_version = isset( $attributes['composition_version'] ) ? (int) $attributes['composition_version'] : 0;
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
					'class'        => 'wp-block-dailyos-account-overview',
					'data-ds-tier' => 'pattern',
					'data-ds-name' => 'AccountOverview',
				]
			)
			: 'class="wp-block-dailyos-account-overview"';

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
			$out .= dailyos_account_overview_render_block( $block );
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
	 * @return string Rendered HTML.
	 */
	function dailyos_account_overview_render_block( array $block ): string {
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

		// Body text. Producer emits claim text under /text for single-claim
		// blocks, /items/*/text for ActionList, /nodes/*/text for
		// RelationshipMap, and an array of overview claims under /context
		// for the AccountOverview summary block.
		$body = '';
		if ( isset( $payload['text'] ) && is_string( $payload['text'] ) ) {
			$body = (string) $payload['text'];
		} elseif ( isset( $payload['items'] ) && is_array( $payload['items'] ) ) {
			$parts = [];
			foreach ( $payload['items'] as $item ) {
				if ( is_array( $item ) && isset( $item['text'] ) && is_string( $item['text'] ) ) {
					$parts[] = (string) $item['text'];
				}
			}
			$body = implode( ' · ', $parts );
		} elseif ( isset( $payload['nodes'] ) && is_array( $payload['nodes'] ) ) {
			$parts = [];
			foreach ( $payload['nodes'] as $node ) {
				if ( is_array( $node ) && isset( $node['text'] ) && is_string( $node['text'] ) ) {
					$parts[] = (string) $node['text'];
				}
			}
			$body = implode( ' · ', $parts );
		} elseif ( isset( $payload['context'] ) && is_array( $payload['context'] ) ) {
			$parts = [];
			foreach ( $payload['context'] as $ctx ) {
				if ( is_array( $ctx ) && isset( $ctx['text'] ) && is_string( $ctx['text'] ) ) {
					$parts[] = (string) $ctx['text'];
				}
			}
			$body = implode( ' · ', $parts );
		}

		$out  = '<article class="dailyos-block dailyos-block-' . esc_attr( $type ) . '">';
		$out .= '<header><h3>' . esc_html( $label ) . '</h3>';
		$out .= '<span data-ds-tier="primitive" data-ds-name="TrustBandBadge" data-ds-spec="primitives/TrustBandBadge.md" data-ds-trust-band="' . esc_attr( $trust ) . '">';
		$out .= esc_html( dailyos_trust_band_label( $trust ) );
		$out .= '</span></header>';
		if ( '' !== $body ) {
			$out .= '<p>' . esc_html( $body ) . '</p>';
		}
		$out .= '</article>';
		return $out;
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
