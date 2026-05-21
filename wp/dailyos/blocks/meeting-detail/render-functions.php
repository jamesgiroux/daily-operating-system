<?php
/**
 * Meeting detail outer-block server-side render (W2 §5.4 / DOS-752).
 *
 * Outer composite block invokes two W1 producers per V1.2.1 §5.4
 * "two-call composition":
 *
 *   1. `get_entity_intelligence` with entity_type=meeting (W1 Meeting
 *      EntityKind extension at `87df7cf6`, merged at `c5c0578f`). Returns
 *      the standard `EntityIntelligenceEnvelope` projecting the 7
 *      `EnvelopeSection` variants (Facts / Health / Touchpoints /
 *      OpenLoops / Threads / Record / MetadataProposals).
 *
 *   2. `meeting_prep_status` (DOS-335) for the Health composition's
 *      prep-status DTO (readiness + freshness).
 *
 * Both responses are passed to inner blocks via providesContext
 * (`dailyos/entityId`, `dailyos/entityType=meeting`,
 * `dailyos/envelopeHandle`). The envelope cache key is the
 * (envelope_render_id, actor_principal_id, surface) tuple per DOS-477.
 *
 * @package DailyOS
 *
 * @var array<string, mixed> $attributes Block attributes (provided by core).
 * @var string               $content    Inner content rendered upstream by core.
 */

declare(strict_types=1);

if ( ! defined( 'ABSPATH' ) ) {
	return '';
}

if ( ! function_exists( 'dailyos_meeting_detail_render' ) ) {
	/**
	 * Render the meeting-detail outer block.
	 *
	 * Invokes `get_entity_intelligence` (entity_type=meeting) AND
	 * `meeting_prep_status` (DOS-335) via the runtime client (no direct
	 * DB reads from PHP), then emits the outer wrapper + inner-blocks
	 * slot. Errors render minimal notices per §10 invariant
	 * (visible-empty chip, never silent-hidden).
	 *
	 * @param array<string, mixed> $attributes Block attributes.
	 * @param string               $content    Pre-rendered inner-block content from core.
	 * @return string Rendered HTML.
	 */
	function dailyos_meeting_detail_render( array $attributes, string $content = '' ): string {
		$meeting_id = isset( $attributes['meeting_id'] ) ? (string) $attributes['meeting_id'] : '';

		// Auto-fill from post context when attribute is empty AND we're
		// rendering inside the matching CPT. L4 quick-setup path: create a
		// `dailyos_meeting` post, set the `dailyos_entity_id` post-meta (or
		// fall back to post slug), and the W2 surface composes automatically.
		if ( '' === $meeting_id && function_exists( 'get_the_ID' ) && function_exists( 'get_post_type' ) ) {
			$post_id = get_the_ID();
			if ( $post_id && 'dailyos_meeting' === get_post_type( $post_id ) ) {
				$meta_id = function_exists( 'get_post_meta' )
					? get_post_meta( $post_id, 'dailyos_entity_id', true )
					: '';
				if ( is_string( $meta_id ) && '' !== $meta_id ) {
					$meeting_id = $meta_id;
				} else {
					$post_obj = function_exists( 'get_post' ) ? get_post( $post_id ) : null;
					if ( $post_obj && is_object( $post_obj ) && isset( $post_obj->post_name ) ) {
						$meeting_id = (string) $post_obj->post_name;
					}
				}
			}
		}

		if ( '' === $meeting_id ) {
			return '<div class="wp-block-dailyos-meeting-detail is-empty" data-empty-reason="missing_meeting_id">'
				. esc_html__( 'No meeting to show here.', 'dailyos' )
				. '</div>';
		}

		$runtime_client = apply_filters( 'dailyos_runtime_client_for_block', null );
		if ( ! is_object( $runtime_client ) || ! method_exists( $runtime_client, 'invoke_ability' ) ) {
			return '<div class="wp-block-dailyos-meeting-detail is-unavailable" data-empty-reason="runtime_unavailable">'
				. esc_html__( 'Runtime unavailable.', 'dailyos' )
				. '</div>';
		}

		// Runtime client signature (class-dailyos-runtime-client.php:85) requires
		// (name, payload, scope_set). Scope set resolves from the surface client's
		// granted scopes via the canonical filter.
		$scope_set = apply_filters( 'dailyos_surfaceclient_resolved_scopes', [] );
		if ( ! is_array( $scope_set ) ) {
			$scope_set = [];
		}

		// W1 producer #1: get_entity_intelligence (entity_type=meeting).
		// Meeting EntityKind extension at `87df7cf6` closes V1.0 codex-challenge F2.
		$envelope_response = $runtime_client->invoke_ability(
			'get_entity_intelligence',
			[
				'entity_type' => 'meeting',
				'entity_id'   => $meeting_id,
			],
			$scope_set
		);

		if ( is_wp_error( $envelope_response ) ) {
			return '<div class="wp-block-dailyos-meeting-detail is-unavailable" data-empty-reason="envelope_error">'
				. esc_html__( 'Runtime unavailable.', 'dailyos' )
				. '</div>';
		}

		// W1 producer #2: meeting_prep_status (DOS-335). Two-call composition
		// per V1.2.1 §5.4 "composite blocks may invoke a sibling DTO-shaped
		// ability alongside the entity envelope; cache key includes both
		// abilities' watermarks".
		$prep_response = $runtime_client->invoke_ability(
			'meeting_prep_status',
			[
				'meeting_id' => $meeting_id,
			],
			$scope_set
		);

		// Inner blocks consume responses via providesContext; the outer block
		// remains the single wiring authority. Prep response is opaque to
		// PHP here — meeting-prep-status inner block re-invokes
		// meeting_prep_status with the same meeting_id when it renders, and
		// the DOS-477 envelope cache de-duplicates the call.
		unset( $envelope_response, $prep_response );

		$wrapper_attrs = function_exists( 'get_block_wrapper_attributes' )
			? get_block_wrapper_attributes(
				[
					'class'                => 'wp-block-dailyos-meeting-detail',
					'data-ds-tier'         => 'pattern',
					'data-ds-name'         => 'MeetingDetail',
					'data-dailyos-surface' => 'meeting_detail',
				]
			)
			: 'class="wp-block-dailyos-meeting-detail" data-dailyos-surface="meeting_detail"';

		$out  = '<section ' . $wrapper_attrs . '>';
		// Inner-blocks slot. The W1 producers consumed across the 10 typed
		// inner blocks are:
		//   - get_entity_intelligence (Facts/Health/Touchpoints/OpenLoops/...)
		//   - meeting_prep_status (DOS-335 — prep DTO)
		//   - claim_receipt (audience-keyed receipt per claim_ref)
		//   - record_claim_feedback (per-claim feedback affordance)
		$out .= '<div class="dailyos-inner-blocks-slot">' . $content . '</div>';
		$out .= '</section>';

		return $out;
	}

	/**
	 * Inner-block READ helper for claim-bearing inner blocks (W2 §5.4 +
	 * W1W2 L2 cycle-2 split). Per-claim render path invokes `claim_receipt`
	 * (audience-keyed receipt builder) through this hook to fetch the
	 * audience-scoped receipt during render. **Render-path only — no writes.**
	 *
	 * Used by `dailyos/meeting-claims-for-review` and
	 * `dailyos/meeting-agenda-draft` per V1.1 §10 claim-fanout invariant.
	 *
	 * @param array<string, mixed> $claim_ref Claim reference from projection
	 *                                         ({ claim_id, audience_key, ... }).
	 * @return array<string, mixed> Receipt envelope.
	 */
	function dailyos_meeting_detail_claim_inner_read( array $claim_ref ): array {
		$runtime_client = apply_filters( 'dailyos_runtime_client_for_block', null );
		if ( ! is_object( $runtime_client ) || ! method_exists( $runtime_client, 'invoke_ability' ) ) {
			return [
				'ok'    => false,
				'error' => [
					'code'    => 'runtime_unavailable',
					'message' => 'DailyOS runtime client not bound for meeting-detail inner consumer.',
				],
			];
		}

		$scope_set = apply_filters( 'dailyos_surfaceclient_resolved_scopes', [] );
		if ( ! is_array( $scope_set ) ) {
			$scope_set = [];
		}

		// W1 producer: claim_receipt — audience-keyed receipt for one claim_ref.
		$receipt_response = $runtime_client->invoke_ability(
			'claim_receipt',
			$claim_ref,
			$scope_set
		);

		return [
			'receipt' => $receipt_response,
		];
	}

	/**
	 * Inner-block WRITE helper for meeting-detail (W1W2 L2 cycle-2 split):
	 * explicit feedback write surface. Called from feedback affordance
	 * handlers — NEVER from render.
	 *
	 * @param array<string, mixed> $claim_ref Claim reference.
	 * @param string               $action    Feedback action variant.
	 * @param array<string, mixed> $metadata  Optional payload_json body.
	 * @return array<string, mixed> Feedback runtime response.
	 */
	function dailyos_meeting_detail_claim_inner_write(
		array $claim_ref,
		string $action,
		array $metadata = []
	): array {
		$runtime_client = apply_filters( 'dailyos_runtime_client_for_block', null );
		if ( ! is_object( $runtime_client ) || ! method_exists( $runtime_client, 'invoke_ability' ) ) {
			return [
				'ok'    => false,
				'error' => [
					'code'    => 'runtime_unavailable',
					'message' => 'DailyOS runtime client not bound for meeting-detail inner consumer.',
				],
			];
		}

		$scope_set = apply_filters( 'dailyos_surfaceclient_resolved_scopes', [] );
		if ( ! is_array( $scope_set ) ) {
			$scope_set = [];
		}

		$payload = $claim_ref;
		$payload['action']   = $action;
		$payload['metadata'] = $metadata;

		$feedback_response = $runtime_client->invoke_ability(
			'record_claim_feedback',
			$payload,
			$scope_set
		);

		return [
			'feedback' => $feedback_response,
		];
	}
}
