<?php
/**
 * Meeting Header inner-block server-side render.
 *
 * Translation of the meeting record header from
 * `.docs/design/reference/surfaces/meeting.html` lines 53-58 — the
 * "Meeting Record" overline + headline + metadata row that sits at the
 * top of every meeting briefing page.
 *
 * Reads `dailyos/entityId` from outer-block context (provided by
 * `dailyos/meeting-detail`), invokes `get_entity_intelligence`
 * (entity_type=meeting) via the runtime client, and projects the Facts
 * section. The DOS-477 envelope cache de-duplicates the call within a
 * single request — sibling inner blocks reuse the same response.
 *
 * Emitted CSS classes match the canonical modules at
 * `.docs/design/reference/_shared/styles/meeting-intel.module.css` so
 * the magazine shell + design tokens apply without theme overrides.
 *
 * @package DailyOS
 *
 * @var array<string, mixed> $attributes Block attributes.
 */

declare(strict_types=1);

if ( ! defined( 'ABSPATH' ) ) {
	return '';
}

if ( ! function_exists( 'dailyos_meeting_header_render' ) ) {
	/**
	 * Render the meeting-header inner block.
	 *
	 * @param array<string, mixed> $attributes Block attributes.
	 * @param mixed                $block      WP_Block instance (provides context).
	 * @return string Rendered HTML.
	 */
	function dailyos_meeting_header_render( array $attributes, $block = null ): string {
		$meeting_id = '';
		if ( is_object( $block ) && isset( $block->context ) && is_array( $block->context ) ) {
			$meeting_id = isset( $block->context['dailyos/entityId'] )
				? (string) $block->context['dailyos/entityId']
				: '';
		}

		if ( '' === $meeting_id ) {
			return dailyos_meeting_header_empty_chip( 'missing_meeting_context', __( 'No meeting context.', 'dailyos' ) );
		}

		$runtime_client = apply_filters( 'dailyos_runtime_client_for_block', null );
		if ( ! is_object( $runtime_client ) || ! method_exists( $runtime_client, 'invoke_ability' ) ) {
			return dailyos_meeting_header_empty_chip( 'runtime_unavailable', __( 'Runtime unavailable.', 'dailyos' ) );
		}

		$scope_set = apply_filters( 'dailyos_surfaceclient_resolved_scopes', [] );
		if ( ! is_array( $scope_set ) ) {
			$scope_set = [];
		}

		$response = $runtime_client->invoke_ability(
			'get_entity_intelligence',
			[
				'entity_type' => 'meeting',
				'entity_id'   => $meeting_id,
			],
			$scope_set
		);

		if ( is_wp_error( $response ) ) {
			return dailyos_meeting_header_empty_chip( 'envelope_error', __( 'Header unavailable.', 'dailyos' ) );
		}

		$facts = dailyos_meeting_header_extract_facts( $response );
		if ( null === $facts ) {
			return dailyos_meeting_header_empty_chip( 'no_facts', __( 'Meeting facts unavailable.', 'dailyos' ) );
		}

		$title           = $facts['title'] ?? '';
		$time_local      = $facts['time_local'] ?? '';
		$meeting_type    = $facts['meeting_type'] ?? '';
		$primary_account = $facts['primary_account'] ?? '';

		if ( '' === $title ) {
			return dailyos_meeting_header_empty_chip( 'no_title', __( 'Meeting title unavailable.', 'dailyos' ) );
		}

		$wrapper_attrs = function_exists( 'get_block_wrapper_attributes' )
			? get_block_wrapper_attributes(
				[
					'class'        => 'wp-block-dailyos-meeting-header meeting-intel_outcomesWrap',
					'data-ds-tier' => 'pattern',
					'data-ds-name' => 'MeetingHeader',
					'data-ds-spec' => 'patterns/MeetingHeader.md',
				]
			)
			: 'class="wp-block-dailyos-meeting-header meeting-intel_outcomesWrap" data-ds-tier="pattern" data-ds-name="MeetingHeader"';

		$metadata_parts = array_filter(
			[ $time_local, $meeting_type, $primary_account ],
			static fn( $value ) => '' !== (string) $value
		);
		$metadata_text = implode( ' · ', array_map( 'esc_html', $metadata_parts ) );

		$out  = '<header ' . $wrapper_attrs . '>';
		$out .= '<p class="meeting-intel_recordOverline">'
			. esc_html__( 'Meeting Record', 'dailyos' )
			. '</p>';
		$out .= '<h1 class="meeting-intel_recordHeadline">'
			. esc_html( $title )
			. '</h1>';
		if ( '' !== $metadata_text ) {
			$out .= '<p class="meeting-intel_metadataText">' . $metadata_text . '</p>';
		}
		$out .= '</header>';

		return $out;
	}
}

if ( ! function_exists( 'dailyos_meeting_header_extract_facts' ) ) {
	/**
	 * Pull the facts section from an EntityIntelligenceEnvelope response
	 * envelope. Returns an associative array keyed by fact key (title /
	 * time_local / etc), or null when facts can't be read.
	 *
	 * @param array<string, mixed> $response Raw runtime client response.
	 * @return array<string, string>|null
	 */
	function dailyos_meeting_header_extract_facts( array $response ): ?array {
		$envelope = null;
		$ability  = $response['ability'] ?? null;
		if ( is_array( $ability ) && isset( $ability['data'] ) && is_array( $ability['data'] ) ) {
			$envelope = $ability['data'];
		} elseif ( isset( $response['data'] ) && is_array( $response['data'] ) ) {
			$envelope = $response['data'];
		} elseif ( isset( $response['envelope'] ) && is_array( $response['envelope'] ) ) {
			$envelope = $response['envelope'];
		} else {
			$envelope = $response;
		}
		if ( ! is_array( $envelope ) ) {
			return null;
		}
		$facts_section = $envelope['facts'] ?? null;
		if ( ! is_array( $facts_section ) ) {
			return null;
		}
		$items = $facts_section['items'] ?? [];
		if ( ! is_array( $items ) || empty( $items ) ) {
			return null;
		}
		$by_key = [];
		foreach ( $items as $item ) {
			if ( ! is_array( $item ) ) {
				continue;
			}
			$key   = isset( $item['key'] ) ? (string) $item['key'] : '';
			$value = isset( $item['value'] ) ? (string) $item['value'] : '';
			if ( '' === $key ) {
				continue;
			}
			$by_key[ $key ] = $value;
		}
		return $by_key;
	}
}

if ( ! function_exists( 'dailyos_meeting_header_empty_chip' ) ) {
	/**
	 * Visible empty-state chip per §10 invariant — never silent-hidden.
	 *
	 * @param string $reason Machine-readable reason for diagnostics.
	 * @param string $label  Human-readable label.
	 * @return string Rendered HTML chip.
	 */
	function dailyos_meeting_header_empty_chip( string $reason, string $label ): string {
		return sprintf(
			'<header class="wp-block-dailyos-meeting-header wp-block-dailyos-meeting-header--empty is-empty"><span class="dailyos-empty-chip" data-empty-reason="%s">%s</span></header>',
			esc_attr( $reason ),
			esc_html( $label )
		);
	}
}
