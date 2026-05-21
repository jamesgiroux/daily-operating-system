<?php
/**
 * Person detail outer-block server-side render (W2 L1 — DOS-484).
 *
 * Per W2 V1.2.1 §5.3 (Path B locked V1.1): the outer block invokes the W1
 * producer `get_entity_intelligence` (entity_type=person) once via the
 * paired DailyOS runtime, then caches the resulting envelope under an
 * `envelope_handle` so the 12 inner blocks (person-hero, vitals-strip,
 * person-insight-chapter, person-network, person-relationships,
 * watch-list, the-work, touchpoints-feed, open-loops-feed,
 * unified-timeline, recommended-actions, person-appendix) project named
 * slices of the composed envelope rather than each re-invoking the
 * producer (DOS-477 / DOS-690 contract).
 *
 * The merge affordance (path α per AC-484.3 + ADR-0123 V1.1 §1) is
 * implemented inside the `dailyos/recommended-actions` inner block: when
 * the envelope's MetadataProposals section contains a `merge_intent`
 * proposal, the inner block surfaces a "Suggest merge" button that
 * emits `FeedbackAction::MergeIntent` (unit variant; payload_json:
 * `{ merge_target: SubjectRef, supporting_evidence?: String }`) via
 * `record_claim_feedback`. The WP block never calls
 * `services::persons::merge` directly — actual merge stays Tauri-side.
 *
 * @package DailyOS
 */

declare(strict_types=1);

if ( ! defined( 'ABSPATH' ) ) {
	return '';
}

if ( ! function_exists( 'dailyos_person_detail_render' ) ) {
	/**
	 * Render the person-detail outer block. Invokes `get_entity_intelligence`
	 * via the runtime client (no direct DB reads from PHP), publishes the
	 * envelope into a per-request handle store so inner blocks can resolve
	 * it via `dailyos/envelopeHandle` context, then renders the default
	 * template (or any caller-supplied inner content).
	 *
	 * @param array<string, mixed> $attributes Block attributes.
	 * @param string               $content    Pre-rendered inner-block content from core.
	 * @return string Rendered HTML.
	 */
	function dailyos_person_detail_render( array $attributes, string $content = '' ): string {
		$person_id = isset( $attributes['person_id'] ) ? (string) $attributes['person_id'] : '';

		if ( '' === $person_id ) {
			return '<div class="wp-block-dailyos-person-detail is-empty">'
				. esc_html__( 'No person to show here.', 'dailyos' )
				. '</div>';
		}

		$runtime_client = apply_filters( 'dailyos_runtime_client_for_block', null );
		if ( ! is_object( $runtime_client ) || ! method_exists( $runtime_client, 'invoke_ability' ) ) {
			return '<div class="wp-block-dailyos-person-detail is-unavailable">'
				. esc_html__( 'Runtime unavailable.', 'dailyos' )
				. '</div>';
		}

		// W1 producer: get_entity_intelligence (entity_type=person).
		// Runtime client signature (class-dailyos-runtime-client.php:85) requires
		// (name, payload, scope_set). The runtime authoritatively enforces
		// required scopes; passing the surface-resolved set scopes the
		// invocation to what this paired site has been granted.
		$scope_set = apply_filters( 'dailyos_surfaceclient_resolved_scopes', [] );
		if ( ! is_array( $scope_set ) ) {
			$scope_set = [];
		}
		$response = $runtime_client->invoke_ability(
			'get_entity_intelligence',
			[
				'entity_type' => 'person',
				'entity_id'   => $person_id,
			],
			$scope_set
		);

		if ( is_wp_error( $response ) ) {
			return '<div class="wp-block-dailyos-person-detail is-unavailable">'
				. esc_html__( 'Runtime unavailable.', 'dailyos' )
				. '</div>';
		}

		// Cache envelope under a handle keyed by entity, so inner blocks
		// reading `dailyos/envelopeHandle` context can resolve the same
		// envelope without re-invoking the producer (DOS-477 cache key).
		$envelope_handle = 'person:' . $person_id;
		dailyos_person_detail_envelope_store( $envelope_handle, $response );

		$wrapper_attrs = function_exists( 'get_block_wrapper_attributes' )
			? get_block_wrapper_attributes(
				[
					'class'                  => 'wp-block-dailyos-person-detail',
					'data-ds-tier'           => 'pattern',
					'data-ds-name'           => 'PersonDetail',
					'data-dailyos-surface'   => 'person_detail',
					'data-dailyos-entity-id' => $person_id,
				]
			)
			: 'class="wp-block-dailyos-person-detail" data-dailyos-surface="person_detail"';

		// If the caller passed no inner content (e.g. direct programmatic
		// render outside the block editor's template path), render the
		// default template so the surface composes the 12 inner blocks.
		$inner = $content;
		if ( '' === trim( $inner ) && function_exists( 'do_blocks' ) ) {
			$inner = do_blocks( dailyos_person_detail_default_template_markup() );
		}

		$out  = '<section ' . $wrapper_attrs . '>';
		$out .= '<div class="dailyos-inner-blocks-slot" data-dailyos-envelope-handle="' . esc_attr( $envelope_handle ) . '">';
		$out .= $inner;
		$out .= '</div>';
		$out .= '</section>';

		return $out;
	}

	/**
	 * Per-request envelope store. Inner blocks resolve their envelope by
	 * handle so the outer block invokes `get_entity_intelligence` exactly
	 * once per render (DOS-477 cache contract). Static-scoped to the
	 * request lifetime; no cross-request persistence.
	 *
	 * @param string                    $handle   Envelope handle (e.g. "person:$id").
	 * @param array<string, mixed>|null $envelope Envelope payload to store, or null
	 *                                            to fetch the current value.
	 * @return array<string, mixed>|null Stored envelope or null if not set.
	 */
	function dailyos_person_detail_envelope_store( string $handle, $envelope = null ) {
		static $store = [];
		if ( null !== $envelope ) {
			$store[ $handle ] = $envelope;
		}
		return isset( $store[ $handle ] ) ? $store[ $handle ] : null;
	}

	/**
	 * Default-template block markup for the person-detail surface. Mirrors
	 * the `template` array in block.json so a direct programmatic render
	 * (no editor inner-content path) still produces the canonical 12-inner-
	 * block composition.
	 *
	 * @return string Block markup for the default template.
	 */
	function dailyos_person_detail_default_template_markup(): string {
		$blocks = [
			'dailyos/person-hero',
			'dailyos/vitals-strip',
			'dailyos/person-insight-chapter',
			'dailyos/person-network',
			'dailyos/person-relationships',
			'dailyos/watch-list',
			'dailyos/the-work',
			'dailyos/touchpoints-feed',
			'dailyos/open-loops-feed',
			'dailyos/unified-timeline',
			'dailyos/recommended-actions',
			'dailyos/person-appendix',
		];
		$out = '';
		foreach ( $blocks as $name ) {
			$out .= '<!-- wp:' . $name . ' /-->';
		}
		return $out;
	}

	/**
	 * Inner-block consumer hook (W2 L1): claim-row inner blocks invoke
	 * `claim_receipt` (audience-keyed receipt builder) and
	 * `record_claim_feedback` (feedback affordance) through this hook so the
	 * outer block remains the single wiring authority. Per W2 §5.3 path α,
	 * the merge affordance also flows through `record_claim_feedback` —
	 * recommended-actions block calls this helper with a
	 * `FeedbackAction::MergeIntent` claim_ref to emit the typed feedback.
	 *
	 * @param array<string, mixed> $claim_ref Claim reference from projection
	 *                                         ({ claim_id, audience_key, action?, ... }).
	 * @return array<string, mixed> Receipt + feedback affordance envelope.
	 */
	function dailyos_person_detail_claim_inner_consumer( array $claim_ref ): array {
		$runtime_client = apply_filters( 'dailyos_runtime_client_for_block', null );
		if ( ! is_object( $runtime_client ) || ! method_exists( $runtime_client, 'invoke_ability' ) ) {
			return [
				'ok'    => false,
				'error' => [
					'code'    => 'runtime_unavailable',
					'message' => 'DailyOS runtime client not bound for inner consumer.',
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

		// W1 producer: record_claim_feedback — feedback affordance mount for
		// the same claim_ref. Per W2 §5.3 path α, the recommended-actions
		// inner block forwards a `FeedbackAction::MergeIntent` claim_ref
		// (unit variant; payload_json carries `merge_target` SubjectRef +
		// optional `supporting_evidence` ≤500 chars, validated by
		// services::claim_receipt::feedback::validate_and_sanitize_metadata
		// per ADR-0123 V1.1 §1). The WP block emits the typed intent; the
		// actual merge runs Tauri-side.
		$feedback_response = $runtime_client->invoke_ability(
			'record_claim_feedback',
			$claim_ref,
			$scope_set
		);

		return [
			'receipt'  => $receipt_response,
			'feedback' => $feedback_response,
		];
	}
}
