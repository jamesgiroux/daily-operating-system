<?php
/**
 * Account Hero (account-hero) — W2 L1 inner block render-functions.
 *
 * Projection rule: Facts (identity).
 *
 * Canonical class names mirror `.docs/design/reference/surfaces/account.html`
 * lines 84–125 (headline section). The lifted reference CSS modules under
 * `wp/dailyos/theme/assets/styles/reference/AccountHero.module.css` +
 * `EntityHeroBase.module.css` + `IntelligenceQualityBadge.module.css` +
 * `EditableText.module.css` + `EditableVitalsStrip.module.css` are
 * auto-enqueued by `enqueue_styles_dir('dailyos-ref', '/styles/reference/')`
 * in `wp/dailyos/theme/functions.php`. Outputting the canonical class names
 * — multi-class form as in the reference render — lets the lifted CSS apply
 * directly. No per-block style.css needed.
 *
 * Per L0-packet-W2-entity-surfaces.md V1.2.1 §5.1 + wave-plan §10 invariant
 * "empty-state pattern": every inner block renders a quiet
 * dailyos-empty-chip with data-empty-reason on absent projection — NEVER
 * silent-hidden. Empty chip nests inside the canonical hero shell so the
 * surface keeps its editorial chrome even when claims aren't yet wired.
 *
 * Inner blocks declare usesContext for dailyos/envelopeHandle and consume
 * the cached envelope via dailyos_resolve_envelope(), which short-circuits
 * to the outer block's single producer invocation.
 *
 * @package DailyOS
 *
 * @var array<string, mixed> $attributes Block attributes (provided by core).
 * @var string               $content    Inner content (empty for dynamic blocks).
 * @var \WP_Block|null       $block      Parsed block (carries usesContext).
 */

declare(strict_types=1);

if ( ! defined( 'ABSPATH' ) ) {
	return '';
}

if ( ! function_exists( 'dailyos_resolve_envelope' ) ) {
	require_once dirname( __DIR__, 3 ) . '/_shared/envelope/envelope-resolver.php';
}

if ( ! function_exists( 'dailyos_account_hero_render' ) ) {
	/**
	 * Render the account-hero inner block.
	 *
	 * @param array<string, mixed> $attributes Block attributes.
	 * @param string               $content    Inner content (empty).
	 * @param \WP_Block|null       $block      Parsed block carrying usesContext.
	 * @return string
	 */
	function dailyos_account_hero_render( array $attributes, string $content = '', $block = null ): string {
		unset( $attributes, $content );

		$handle    = null;
		$entity_id = '';
		if ( null !== $block && isset( $block->context ) && is_array( $block->context ) ) {
			$handle    = isset( $block->context['dailyos/envelopeHandle'] ) ? (string) $block->context['dailyos/envelopeHandle'] : null;
			$entity_id = isset( $block->context['dailyos/entityId'] ) ? (string) $block->context['dailyos/entityId'] : '';
		}
		if ( ( null === $handle || '' === $handle ) && isset( $GLOBALS['dailyos_envelope_handle_for_request'] ) ) {
			$handle = (string) $GLOBALS['dailyos_envelope_handle_for_request'];
		}

		$scope_set = apply_filters( 'dailyos_surfaceclient_resolved_scopes', [] );
		if ( ! is_array( $scope_set ) ) {
			$scope_set = [];
		}

		$envelope = dailyos_resolve_envelope( $handle, 'account', $entity_id, $scope_set );

		// Pull subject identity + facts projection. Fields fall back to
		// envelope.subject when the typed facts section is absent so the hero
		// shell still renders the account name during partial-state renders.
		$subject = ( is_array( $envelope ) && isset( $envelope['subject'] ) && is_array( $envelope['subject'] ) )
			? $envelope['subject']
			: [];
		$subject_name = '';
		foreach ( [ 'name', 'displayName', 'display_name', 'displayLabel', 'display_label', 'label' ] as $name_key ) {
			if ( isset( $subject[ $name_key ] ) && is_string( $subject[ $name_key ] ) && '' !== $subject[ $name_key ] ) {
				$subject_name = $subject[ $name_key ];
				break;
			}
		}
		$account_type = '';
		foreach ( [ 'accountType', 'account_type', 'type', 'lifecycle' ] as $type_key ) {
			if ( isset( $subject[ $type_key ] ) && is_string( $subject[ $type_key ] ) && '' !== $subject[ $type_key ] ) {
				$account_type = $subject[ $type_key ];
				break;
			}
		}

		// Section projection: Facts. If the section has no items, fall back
		// to subject-only render (still renders the editorial hero shell with
		// the empty chip nested inside).
		$projected_sections = [ 'facts' ];
		$any_present = false;
		$first_empty_reason = '';
		foreach ( $projected_sections as $section_key ) {
			$state = dailyos_envelope_section( $envelope, $section_key );
			if ( $state['present'] && $state['item_count'] > 0 ) {
				$any_present = true;
				break;
			}
			if ( '' === $first_empty_reason && '' !== $state['reason'] ) {
				$first_empty_reason = $state['reason'];
			}
		}

		// Compose the canonical hero shell. Class names mirror
		// .docs/design/reference/surfaces/account.html headline section.
		$out  = '<section id="headline" class="entity-detail_chapterSection" data-ds-tier="pattern" data-ds-name="AccountHero" data-ds-spec="patterns/AccountHero.md">';
		$out .= '<div class="AccountHero_hero EntityHeroBase_hero">';

		// Hero date + freshness + type badge row.
		$out .= '<div class="AccountHero_heroDate EntityHeroBase_heroDate AccountHero_heroDateLayout">';
		$quality_level = dailyos_account_hero_quality_level_from_envelope( $envelope );
		$out .= '<span class="IntelligenceQualityBadge_root" data-quality-level="' . esc_attr( $quality_level ) . '">';
		$out .= '<span class="IntelligenceQualityBadge_dot"></span>';
		$out .= '</span>';
		if ( '' !== $account_type ) {
			$badge_modifier = 'AccountHero_' . sanitize_html_class( strtolower( $account_type ) ) . 'Badge';
			$out .= '<div class="AccountHero_typeBadgeWrapper">';
			$out .= '<span class="AccountHero_badge EntityHeroBase_heroBadge ' . esc_attr( $badge_modifier ) . '">';
			$out .= esc_html( ucfirst( $account_type ) );
			$out .= '</span>';
			$out .= '</div>';
		}
		$out .= '</div>';

		// Headline name. Empty subject still renders the EditableText shell so
		// the editorial typography is visible even before envelope resolves.
		$out .= '<h1 class="AccountHero_name EntityHeroBase_heroTitle">';
		if ( '' !== $subject_name ) {
			$out .= '<span class="EditableText_editable">' . esc_html( $subject_name ) . '</span>';
		} elseif ( '' !== $entity_id ) {
			$out .= '<span class="EditableText_editable">' . esc_html( $entity_id ) . '</span>';
		} else {
			$out .= '<span class="EditableText_editable EditableText_empty">' . esc_html__( 'Account', 'dailyos' ) . '</span>';
		}
		$out .= '</h1>';

		// Facts projection. When facts are present, render them as
		// EditableVitalsStrip items. When absent, fall back to the empty chip
		// nested inside the hero shell so the editorial chrome still renders.
		if ( $any_present ) {
			$out .= '<div class="EditableVitalsStrip_strip">';
			$out .= '<div class="EditableVitalsStrip_stripItems">';
			$projected_claim_refs = dailyos_account_hero_select_claim_refs( $envelope );
			foreach ( $projected_claim_refs as $claim_ref ) {
				$receipt = dailyos_envelope_consume_claim( $claim_ref, $scope_set );
				if ( null === $receipt ) {
					continue;
				}
				$out .= dailyos_account_hero_render_row( $claim_ref, $receipt );
			}
			$out .= '</div>';
			$out .= '</div>';
		} else {
			$reason = '' !== $first_empty_reason ? $first_empty_reason : 'no_account_facts';
			$out .= dailyos_empty_chip(
				$reason,
				__( 'Account identity unavailable', 'dailyos' ),
				'AccountHero_facts'
			);
		}

		$out .= '</div>'; // .AccountHero_hero
		$out .= '</section>';
		return $out;
	}
}


if ( ! function_exists( 'dailyos_account_hero_select_claim_refs' ) ) {
	/**
	 * Select claim references from the envelope for the account-hero projection.
	 * Pure projection — does not invoke any abilities; receipts fan out in
	 * the renderer via dailyos_envelope_consume_claim().
	 *
	 * Projection rule: Facts (identity).
	 *
	 * @param array<string,mixed>|null $envelope Envelope payload.
	 * @return array<int,array<string,mixed>>
	 */
	function dailyos_account_hero_select_claim_refs( ?array $envelope ): array {
		if ( null === $envelope ) {
			return [];
		}
		$refs = [];
		foreach ( [ 'facts' ] as $section_key ) {
			$slice = $envelope[ $section_key ] ?? [];
			if ( ! is_array( $slice ) ) {
				continue;
			}
			$items = $slice['items'] ?? ( is_array( reset( $slice ) ) ? $slice : [] );
			if ( ! is_array( $items ) ) {
				continue;
			}
			foreach ( $items as $item ) {
				if ( ! is_array( $item ) ) {
					continue;
				}
				$claim_id = $item['claimId'] ?? $item['claim_id'] ?? '';
				if ( '' === $claim_id ) {
					continue;
				}
				$refs[] = [
					'claim_id'     => (string) $claim_id,
					'audience_key' => $item['audienceKey'] ?? 'user',
					'subject_ref'  => $item['subjectRef'] ?? null,
					'field_path'   => $item['fieldPath'] ?? null,
				];
			}
		}
		return $refs;
	}
}

if ( ! function_exists( 'dailyos_account_hero_quality_level_from_envelope' ) ) {
	/**
	 * Derive the IntelligenceQualityBadge data-quality-level attribute from
	 * the envelope's intelligence quality summary. Falls back to "unknown"
	 * when the envelope hasn't provided a quality signal yet.
	 *
	 * @param array<string,mixed>|null $envelope
	 * @return string
	 */
	function dailyos_account_hero_quality_level_from_envelope( ?array $envelope ): string {
		if ( ! is_array( $envelope ) ) {
			return 'unknown';
		}
		$quality = $envelope['intelligence_quality'] ?? $envelope['intelligenceQuality'] ?? null;
		if ( is_array( $quality ) ) {
			$level = $quality['level'] ?? $quality['band'] ?? '';
			if ( is_string( $level ) && '' !== $level ) {
				return $level;
			}
		}
		if ( is_string( $quality ) && '' !== $quality ) {
			return $quality;
		}
		return 'unknown';
	}
}

if ( ! function_exists( 'dailyos_account_hero_render_row' ) ) {
	/**
	 * Render a single fact row inside the EditableVitalsStrip. Receipt was
	 * already resolved server-side through claim_receipt; this function
	 * shapes the typed display (vitals item with field stack + source
	 * attribution) mirroring .docs/design/reference/surfaces/account.html
	 * vitals-strip items.
	 *
	 * @param array<string,mixed> $claim_ref The claim_ref passed to the consumer.
	 * @param array<string,mixed> $receipt   Receipt payload returned by claim_receipt.
	 * @return string
	 */
	function dailyos_account_hero_render_row( array $claim_ref, array $receipt ): string {
		$claim_id   = isset( $claim_ref['claim_id'] ) ? (string) $claim_ref['claim_id'] : '';
		$trust_band = dailyos_receipt_trust_band( $receipt );
		$display    = dailyos_receipt_rendered_text( $receipt, $claim_id );
		$source     = dailyos_receipt_source_label( $receipt );

		$out  = '<span class="EditableVitalsStrip_itemWithSeparator" data-claim-id="' . esc_attr( $claim_id ) . '" data-trust-band="' . esc_attr( $trust_band ) . '">';
		$out .= '<span class="EditableVitalsStrip_separatorDot"></span>';
		$out .= '<span class="EditableVitalsStrip_fieldStack">';
		$out .= '<span class="EditableVitalsStrip_fieldRow">';
		$out .= '<span class="EditableVitalsStrip_clickableFieldValue">' . esc_html( $display ) . '</span>';
		$out .= '</span>';
		if ( '' !== $source ) {
			$out .= '<span class="EditableVitalsStrip_sourceAttribution">' . esc_html( $source ) . '</span>';
		}
		$out .= '</span>';
		$out .= '</span>';
		return $out;
	}
}
