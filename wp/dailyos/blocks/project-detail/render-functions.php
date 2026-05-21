<?php
/**
 * Project detail outer-block server-side render (W2 V1.2.1 §5.2 — DOS-483).
 *
 * Per W2 V1.2.1 §5.2 (Path B locked V1.1): the outer block invokes the W1
 * producer `get_entity_intelligence` (entity_type=project) once via the
 * paired DailyOS runtime, caches the resulting envelope under an opaque
 * `envelope_handle` so the 15 inner blocks (project-hero, vitals-strip,
 * portfolio-chapter, trajectory-chapter, horizon-chapter, watch-list,
 * watch-list-milestones, stakeholder-gallery, the-work, touchpoints-feed,
 * open-loops-feed, linear-issues-chapter, unified-timeline,
 * recommended-actions, project-appendix) project named slices of the
 * composed envelope rather than each re-invoking the producer (DOS-477
 * envelope cache contract).
 *
 * DOS-725 tint (V1.1 lock — wave §10 invariant "Per-project tint via CSS
 * custom property"): the outer wrapper carries
 * `style="--dailyos-project-tint: var(--color-garden-olive);"` — the
 * canonical olive token (ADR-0077 amendment). The
 * `check_no_inline_style_exception.sh` CI gate enforces that the only
 * permitted style-attribute body matches
 * `^--dailyos-[a-z-]+:\s*var\(--[a-z-]+\);?$` — narrow exception to
 * memory `feedback_no_inline_css`.
 *
 * @package DailyOS
 */

declare(strict_types=1);

if ( ! defined( 'ABSPATH' ) ) {
	return '';
}

if ( ! function_exists( 'dailyos_resolve_envelope' ) ) {
	$dailyos_project_detail_shared_resolver = dirname( __DIR__, 1 ) . '/_shared/envelope/envelope-resolver.php';
	if ( file_exists( $dailyos_project_detail_shared_resolver ) ) {
		require_once $dailyos_project_detail_shared_resolver;
	}
}

if ( ! function_exists( 'dailyos_project_detail_render' ) ) {
	/**
	 * Render the project-detail outer block. Invokes `get_entity_intelligence`
	 * via the runtime client (no direct DB reads from PHP), publishes the
	 * envelope into the shared request-scoped cache so inner blocks can
	 * resolve it via `dailyos/envelopeHandle` context, then renders the
	 * default template (or any caller-supplied inner content).
	 *
	 * @param array<string, mixed> $attributes Block attributes.
	 * @param string               $content    Pre-rendered inner-block content from core.
	 * @return string Rendered HTML.
	 */
	function dailyos_project_detail_render( array $attributes, string $content = '' ): string {
		$project_id = isset( $attributes['project_id'] ) ? (string) $attributes['project_id'] : '';

		// Auto-fill from post context when attribute is empty AND we're
		// rendering inside the matching CPT. L4 quick-setup path: create a
		// `dailyos_project` post, set the `dailyos_entity_id` post-meta (or
		// fall back to post slug), and the W2 surface composes automatically.
		if ( '' === $project_id && function_exists( 'get_the_ID' ) && function_exists( 'get_post_type' ) ) {
			$post_id = get_the_ID();
			if ( $post_id && 'dailyos_project' === get_post_type( $post_id ) ) {
				$meta_id = function_exists( 'get_post_meta' )
					? get_post_meta( $post_id, 'dailyos_entity_id', true )
					: '';
				if ( is_string( $meta_id ) && '' !== $meta_id ) {
					$project_id = $meta_id;
				} else {
					$post_obj = function_exists( 'get_post' ) ? get_post( $post_id ) : null;
					if ( $post_obj && is_object( $post_obj ) && isset( $post_obj->post_name ) ) {
						$project_id = (string) $post_obj->post_name;
					}
				}
			}
		}

		if ( '' === $project_id ) {
			return '<div class="wp-block-dailyos-project-detail is-empty">'
				. esc_html__( 'No project to show here.', 'dailyos' )
				. '</div>';
		}

		$runtime_client = apply_filters( 'dailyos_runtime_client_for_block', null );
		if ( ! is_object( $runtime_client ) || ! method_exists( $runtime_client, 'invoke_ability' ) ) {
			return '<div class="wp-block-dailyos-project-detail is-unavailable">'
				. esc_html__( 'Runtime unavailable.', 'dailyos' )
				. '</div>';
		}

		// W1 producer: get_entity_intelligence (entity_type=project).
		// Runtime client signature (class-dailyos-runtime-client.php:85) requires
		// (name, payload, scope_set). Scope set resolves from the surface client's
		// granted scopes via the canonical filter.
		$scope_set = apply_filters( 'dailyos_surfaceclient_resolved_scopes', [] );
		if ( ! is_array( $scope_set ) ) {
			$scope_set = [];
		}
		$response = $runtime_client->invoke_ability(
			'get_entity_intelligence',
			[
				'entity_type' => 'project',
				'entity_id'   => $project_id,
			],
			$scope_set
		);

		if ( is_wp_error( $response ) ) {
			return '<div class="wp-block-dailyos-project-detail is-unavailable">'
				. esc_html__( 'Runtime unavailable.', 'dailyos' )
				. '</div>';
		}

		// Publish the envelope into the shared request-scoped cache under
		// an opaque handle (envelope_render_id when DOS-477 surfaces it on
		// the response, else a deterministic per-request hash). Inner
		// blocks resolve via `dailyos_resolve_envelope( $handle, 'project',
		// $entity_id, $scope_set )` — short-circuits to this same payload.
		$envelope_handle = '';
		if ( is_array( $response ) && function_exists( 'dailyos_envelope_handle_from_response' ) ) {
			$envelope_handle = dailyos_envelope_handle_from_response( $response, 'project', $project_id );
		}
		// Expose the handle to inner blocks that fall through block context
		// (e.g. direct programmatic render outside the editor's context path).
		$GLOBALS['dailyos_envelope_handle_for_request'] = $envelope_handle;

		// DOS-725: per-project tint via CSS custom property on outer wrapper.
		// Narrow exception to `feedback_no_inline_css`; gated by
		// `check_no_inline_style_exception.sh`. Format constraint:
		// `^--dailyos-[a-z-]+:\s*var\(--[a-z-]+\);?$` — `--dailyos-*` namespace only.
		$wrapper_attrs = function_exists( 'get_block_wrapper_attributes' )
			? get_block_wrapper_attributes(
				[
					'class'                  => 'wp-block-dailyos-project-detail',
					'style'                  => '--dailyos-project-tint: var(--color-garden-olive);',
					'data-ds-tier'           => 'pattern',
					'data-ds-name'           => 'ProjectDetail',
					'data-dailyos-surface'   => 'project_detail',
					'data-dailyos-entity-id' => $project_id,
				]
			)
			: 'class="wp-block-dailyos-project-detail" style="--dailyos-project-tint: var(--color-garden-olive);" data-dailyos-surface="project_detail"';

		// If the caller passed no inner content (e.g. direct programmatic
		// render outside the block editor's template path), render the
		// default template so the surface composes the 15 inner blocks.
		$inner = $content;
		if ( '' === trim( $inner ) && function_exists( 'do_blocks' ) ) {
			$inner = do_blocks( dailyos_project_detail_default_template_markup() );
		}

		$out  = '<section ' . $wrapper_attrs . '>';
		$out .= '<div class="dailyos-inner-blocks-slot" data-dailyos-envelope-handle="' . esc_attr( $envelope_handle ) . '">';
		$out .= $inner;
		$out .= '</div>';
		$out .= '</section>';

		return $out;
	}

	/**
	 * Default-template block markup for the project-detail surface. Mirrors
	 * the `template` array in block.json so a direct programmatic render
	 * (no editor inner-content path) still produces the canonical 15-inner-
	 * block composition.
	 *
	 * Per W2 V1.2.1 §5.2 cycle-2 F1 fix: `linear-issues-chapter` is
	 * 1-to-1 with `src/pages/ProjectDetailEditorial.tsx:526`.
	 *
	 * @return string Block markup for the default template.
	 */
	function dailyos_project_detail_default_template_markup(): string {
		$blocks = [
			'dailyos/project-hero',
			'dailyos/vitals-strip',
			'dailyos/portfolio-chapter',
			'dailyos/trajectory-chapter',
			'dailyos/horizon-chapter',
			'dailyos/watch-list',
			'dailyos/watch-list-milestones',
			'dailyos/stakeholder-gallery',
			'dailyos/the-work',
			'dailyos/touchpoints-feed',
			'dailyos/open-loops-feed',
			'dailyos/linear-issues-chapter',
			'dailyos/unified-timeline',
			'dailyos/recommended-actions',
			'dailyos/project-appendix',
		];
		$out = '';
		foreach ( $blocks as $name ) {
			$out .= '<!-- wp:' . $name . ' /-->';
		}
		return $out;
	}

	/**
	 * Inner-block READ helper (W1W2 L2 cycle-2 split): claim-row inner blocks
	 * invoke `claim_receipt` (audience-keyed receipt builder) through this
	 * hook to fetch the audience-scoped receipt during render. The
	 * consumer-skeleton CI gate (AC-W1.9) lints the 3-arg invocation here.
	 *
	 * **Render-path only.** Read does NOT emit `record_claim_feedback`; the
	 * feedback write path is `_claim_inner_write` and runs explicitly from
	 * the feedback affordance handler.
	 *
	 * @param array<string, mixed> $claim_ref Claim reference
	 *                                        ({ claim_id, audience_key, ... }).
	 * @return array<string, mixed> Receipt envelope.
	 */
	function dailyos_project_detail_claim_inner_read( array $claim_ref ): array {
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

		return [
			'receipt' => $receipt_response,
		];
	}

	/**
	 * Inner-block WRITE helper (W1W2 L2 cycle-2 split): explicit feedback
	 * write surface for the project-detail subtree. Called from feedback
	 * affordance handlers — NEVER from render. AC-W1.9 consumer-skeleton
	 * gate verifies the 3-arg `record_claim_feedback` invocation here.
	 *
	 * @param array<string, mixed> $claim_ref Claim reference.
	 * @param string               $action    Feedback action variant.
	 * @param array<string, mixed> $metadata  Optional metadata.
	 * @return array<string, mixed> Feedback runtime response.
	 */
	function dailyos_project_detail_claim_inner_write(
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
					'message' => 'DailyOS runtime client not bound for inner consumer.',
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
