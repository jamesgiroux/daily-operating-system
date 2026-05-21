<?php
/**
 * Recommended Actions inner-block server-side render (W2 L1 — DOS-484).
 *
 * Projects from envelope sections:
 *   - OpenLoops (recommended subset) — per-action trust band.
 *   - MetadataProposals — `merge_intent` proposals surface the
 *     Suggest-merge affordance (W2 §5.3 path α).
 *
 * Merge affordance contract (per AC-484.3 + ADR-0123 V1.1 §1):
 *   When the envelope's MetadataProposals section contains one or more
 *   proposals of kind `merge_intent`, this block emits a server-rendered
 *   "Suggest merge" button per proposal. Clicking the button (handled by
 *   the view-script when present, or via form-submission fallback) emits
 *   a `FeedbackAction::MergeIntent` claim-feedback through
 *   `record_claim_feedback` via the outer-block helper
 *   `dailyos_person_detail_claim_inner_read()`.
 *
 *   Canonical payload shape (sanitized by
 *   services::claim_receipt::feedback::validate_and_sanitize_metadata):
 *     - merge_target: SubjectRef   (canonical person to merge INTO)
 *     - supporting_evidence: Option<String> (≤ 500 chars, ADR-0108 §3
 *       sanitization)
 *
 *   The WP block NEVER calls `services::persons::merge` directly — it
 *   emits the typed feedback intent and Tauri-side executes the merge
 *   (AC-484.3, AC-8.13). The Agent actor is denied this affordance; only
 *   the User actor renders the button (AC-8.13).
 *
 * @package DailyOS
 */

declare(strict_types=1);

if ( ! defined( 'ABSPATH' ) ) {
	return '';
}

if ( ! function_exists( 'dailyos_recommended_actions_render' ) ) {
	/**
	 * Render the recommended-actions inner block.
	 *
	 * @param array<string, mixed> $attributes Block attributes.
	 * @param array<string, mixed> $ctx        Block context (entityType, entityId,
	 *                                          envelopeHandle).
	 * @return string Rendered HTML.
	 */
	function dailyos_recommended_actions_render( array $attributes, array $ctx = [] ): string {
		$handle    = isset( $ctx['dailyos/envelopeHandle'] ) ? (string) $ctx['dailyos/envelopeHandle'] : '';
		$entity_id = isset( $ctx['dailyos/entityId'] ) ? (string) $ctx['dailyos/entityId'] : '';
		$envelope  = null;
		if ( '' !== $handle && function_exists( 'dailyos_person_detail_envelope_store' ) ) {
			$envelope = dailyos_person_detail_envelope_store( $handle );
		}

		$wrapper_attrs = function_exists( 'get_block_wrapper_attributes' )
			? get_block_wrapper_attributes(
				[
					'class'        => 'wp-block-dailyos-recommended-actions',
					'data-ds-tier' => 'pattern',
					'data-ds-name' => 'RecommendedActions',
				]
			)
			: 'class="wp-block-dailyos-recommended-actions"';

		if ( ! is_array( $envelope ) ) {
			return '<section ' . $wrapper_attrs . '>'
				. '<span class="dailyos-empty-chip" data-empty-reason="no_envelope">'
				. esc_html__( 'No data yet.', 'dailyos' )
				. '</span>'
				. '</section>';
		}

		$out  = '<section ' . $wrapper_attrs . '>';
		$out .= '<div class="dailyos-recommended-actions__body" data-dailyos-projection="recommended-actions" data-trust-band-source="per-action">';

		// Body of the recommended-actions list: per-OpenLoop rendering with
		// per-action trust band lands in subsequent L1 passes per the
		// AC-484.5 visible-QA matrix. The structural skeleton (envelope
		// resolution, empty-state chip, merge affordance) is the L1
		// deliverable.
		$out .= dailyos_recommended_actions_render_merge_affordance( $envelope, $entity_id );
		$out .= '</div>';
		$out .= '</section>';

		return $out;
	}

	/**
	 * Render the Suggest-merge affordance. Detects merge_intent proposals
	 * in the envelope's MetadataProposals section and emits one button per
	 * proposal. Only renders for User-actor surfaces (per AC-8.13 — Agent
	 * actor is denied this affordance).
	 *
	 * The button carries the claim_ref payload as data-attributes; the
	 * frontend handler (view-script when present) reads them and POSTs
	 * the typed feedback through the runtime client. Server-side, the
	 * `dailyos_person_detail_claim_inner_read` helper is the single
	 * wiring authority for `record_claim_feedback` invocations — the
	 * affordance click ultimately flows through that helper (see the
	 * outer block's render-functions.php).
	 *
	 * @param array<string, mixed> $envelope  Cached envelope from the outer block.
	 * @param string               $entity_id Subject person id (this person —
	 *                                         the merge SOURCE).
	 * @return string Rendered affordance HTML (empty string when no merge
	 *                intent proposals are present or actor is Agent).
	 */
	function dailyos_recommended_actions_render_merge_affordance( array $envelope, string $entity_id ): string {
		$audience = isset( $envelope['audience'] ) ? (string) $envelope['audience'] : 'user';
		// AC-8.13: Agent actor is denied the merge affordance.
		if ( 'agent' === strtolower( $audience ) || 'agent_mcp' === strtolower( $audience ) ) {
			return '';
		}

		$proposals = [];
		if ( isset( $envelope['metadata_proposals'] ) && is_array( $envelope['metadata_proposals'] ) ) {
			$proposals = $envelope['metadata_proposals'];
		} elseif ( isset( $envelope['MetadataProposals'] ) && is_array( $envelope['MetadataProposals'] ) ) {
			$proposals = $envelope['MetadataProposals'];
		}

		$merge_proposals = [];
		foreach ( $proposals as $proposal ) {
			if ( ! is_array( $proposal ) ) {
				continue;
			}
			$kind = isset( $proposal['kind'] ) ? (string) $proposal['kind'] : '';
			$kind = '' === $kind && isset( $proposal['proposal_kind'] ) ? (string) $proposal['proposal_kind'] : $kind;
			if ( 'merge_intent' === $kind ) {
				$merge_proposals[] = $proposal;
			}
		}

		if ( [] === $merge_proposals ) {
			return '';
		}

		$out = '<div class="dailyos-recommended-actions__merge-affordances" data-dailyos-affordance="merge-intent">';
		$out .= '<h3 class="dailyos-recommended-actions__heading">' . esc_html__( 'Suggest a merge', 'dailyos' ) . '</h3>';
		foreach ( $merge_proposals as $proposal ) {
			$out .= dailyos_recommended_actions_render_merge_button( $proposal, $entity_id );
		}
		$out .= '</div>';

		return $out;
	}

	/**
	 * Render a single Suggest-merge button for one merge_intent proposal.
	 *
	 * The button carries the canonical MergeIntent payload shape (per
	 * ADR-0123 V1.1 §1) as data-attributes:
	 *   - data-claim-id            — the source claim_ref id
	 *   - data-merge-source        — this person's entity id (merge FROM)
	 *   - data-merge-target-kind   — SubjectRef.kind (person)
	 *   - data-merge-target-id     — SubjectRef.id (canonical person to
	 *                                merge INTO)
	 *   - data-feedback-action     — `merge_intent`
	 *
	 * Click handler flow (Tauri-side actually executes; WP only signals):
	 *   1. Frontend collects the data-attrs + optional
	 *      `supporting_evidence` from a textarea (≤500 chars).
	 *   2. POST to a nonce-protected admin-ajax/REST endpoint that calls
	 *      `dailyos_person_detail_claim_inner_read( $claim_ref )` with
	 *      `action => 'merge_intent'` and `payload_json` carrying
	 *      `merge_target` + `supporting_evidence`.
	 *   3. `record_claim_feedback` validates + sanitizes the payload
	 *      (services::claim_receipt::feedback::validate_and_sanitize_metadata).
	 *   4. Tauri-side surfaces the typed intent and executes the actual
	 *      merge through `services::persons::merge` when the user
	 *      confirms.
	 *
	 * @param array<string, mixed> $proposal  A merge_intent metadata proposal.
	 * @param string               $entity_id Subject person id (merge source).
	 * @return string Rendered button HTML.
	 */
	function dailyos_recommended_actions_render_merge_button( array $proposal, string $entity_id ): string {
		$claim_id  = isset( $proposal['claim_id'] ) ? (string) $proposal['claim_id'] : '';
		$target    = isset( $proposal['merge_target'] ) && is_array( $proposal['merge_target'] ) ? $proposal['merge_target'] : [];
		$target_id = isset( $target['id'] ) ? (string) $target['id'] : '';
		$target_kind = isset( $target['kind'] ) ? (string) $target['kind'] : 'person';
		$target_label = isset( $proposal['merge_target_label'] ) ? (string) $proposal['merge_target_label'] : $target_id;

		// Quiet skip if the proposal lacks the canonical merge_target
		// shape — never silent-hidden: emit a quiet chip so the empty
		// reason is debuggable (W2 §10 invariant).
		if ( '' === $target_id ) {
			return '<span class="dailyos-empty-chip" data-empty-reason="merge_intent_missing_target">'
				. esc_html__( 'Merge target unavailable.', 'dailyos' )
				. '</span>';
		}

		$label = sprintf(
			/* translators: %s: name or id of the person to merge into. */
			esc_html__( 'Suggest merge into %s', 'dailyos' ),
			esc_html( $target_label )
		);

		$button  = '<button type="button" class="dailyos-button dailyos-button--suggest-merge"';
		$button .= ' data-dailyos-action="merge-intent"';
		$button .= ' data-claim-id="' . esc_attr( $claim_id ) . '"';
		$button .= ' data-feedback-action="merge_intent"';
		$button .= ' data-merge-source="' . esc_attr( $entity_id ) . '"';
		$button .= ' data-merge-target-kind="' . esc_attr( $target_kind ) . '"';
		$button .= ' data-merge-target-id="' . esc_attr( $target_id ) . '"';
		$button .= '>' . $label . '</button>';

		return $button;
	}
}
