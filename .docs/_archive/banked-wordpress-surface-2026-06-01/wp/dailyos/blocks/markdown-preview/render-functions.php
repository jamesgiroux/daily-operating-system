<?php
/**
 * Markdown Preview block server-side render helpers.
 *
 * @package DailyOS
 */

declare(strict_types=1);

if ( ! defined( 'ABSPATH' ) ) {
	return '';
}

require_once dirname( __DIR__, 2 ) . '/includes/class-dailyos-markdown-sanitizer.php';

if ( ! function_exists( 'dailyos_markdown_preview_render' ) ) {
	/**
	 * Render the Markdown Preview block.
	 *
	 * @param array<string, mixed> $attributes Block attributes.
	 * @param string               $content Inner content.
	 * @param \WP_Block|null       $block Parsed block.
	 * @return string
	 */
	function dailyos_markdown_preview_render( array $attributes, string $content = '', $block = null ): string {
		unset( $content, $block );

		$source_handle = isset( $attributes['source_handle'] ) && is_string( $attributes['source_handle'] )
			? trim( $attributes['source_handle'] )
			: '';
		if ( '' === $source_handle || ! dailyos_markdown_preview_valid_handle( $source_handle ) ) {
			return dailyos_markdown_preview_render_state(
				'not_ready',
				__( 'Preview appears when a source is selected.', 'dailyos' )
			);
		}

		$runtime_client = apply_filters( 'dailyos_runtime_client_for_block', null );
		if ( is_object( $runtime_client ) && is_callable( [ $runtime_client, 'read_markdown_preview' ] ) ) {
			$response = $runtime_client->read_markdown_preview( $source_handle );
			return dailyos_markdown_preview_render_from_response( $response );
		}

		return dailyos_markdown_preview_render_state(
			'unavailable',
			__( 'Preview is temporarily unavailable.', 'dailyos' )
		);
	}

	/**
	 * Validate an opaque source handle without rendering it.
	 *
	 * @param string $source_handle Source handle.
	 * @return bool
	 */
	function dailyos_markdown_preview_valid_handle( string $source_handle ): bool {
		return 1 === preg_match( '/^[A-Za-z0-9:_-]{1,160}$/', $source_handle );
	}

	/**
	 * Render a runtime response.
	 *
	 * @param mixed $response Runtime response.
	 * @return string
	 */
	function dailyos_markdown_preview_render_from_response( mixed $response ): string {
		if ( is_wp_error( $response ) || ! is_array( $response ) ) {
			return dailyos_markdown_preview_render_state(
				'unavailable',
				__( 'Preview is temporarily unavailable.', 'dailyos' )
			);
		}

		if ( isset( $response['error'] ) ) {
			return dailyos_markdown_preview_render_state(
				'unavailable',
				__( 'Preview is temporarily unavailable.', 'dailyos' )
			);
		}

		$payload = isset( $response['data'] ) && is_array( $response['data'] ) ? $response['data'] : $response;
		return dailyos_markdown_preview_render_payload( $payload );
	}

	/**
	 * Render a safe markdown-preview payload.
	 *
	 * @param array<string, mixed> $payload Runtime payload.
	 * @return string
	 */
	function dailyos_markdown_preview_render_payload( array $payload ): string {
		$preview_html = isset( $payload['preview_html'] ) && is_string( $payload['preview_html'] )
			? $payload['preview_html']
			: ( isset( $payload['previewHtml'] ) && is_string( $payload['previewHtml'] ) ? $payload['previewHtml'] : '' );

		if ( '' === trim( $preview_html ) ) {
			return dailyos_markdown_preview_render_state(
				'empty',
				__( 'No preview is available for this source.', 'dailyos' )
			);
		}

		$sanitizer = new \DailyOS\DailyOS_Markdown_Sanitizer();
		$allow_asset_images = ! empty( $payload['asset_resolver_available'] ) || ! empty( $payload['assetResolverAvailable'] );
		$body = $sanitizer->sanitize(
			$preview_html,
			[
				'allow_dailyos_asset_images' => $allow_asset_images,
			]
		);

		$out  = '<section ' . dailyos_markdown_preview_wrapper_attrs( 'ready' ) . ' aria-labelledby="dailyos-markdown-preview-heading">';
		$out .= dailyos_markdown_preview_render_header( $payload );
		$out .= '<div class="dailyos-markdown-preview__body">';
		$out .= $body;
		$out .= '</div>';
		$out .= '</section>';

		return $out;
	}

	/**
	 * Render the block header with safe metadata.
	 *
	 * @param array<string, mixed> $payload Runtime payload.
	 * @return string
	 */
	function dailyos_markdown_preview_render_header( array $payload ): string {
		$label = dailyos_markdown_preview_safe_text(
			dailyos_markdown_preview_first_string( $payload, [ 'source_label', 'sourceLabel', 'label' ], '' ),
			__( 'Workspace source', 'dailyos' ),
			72
		);
		$lifecycle = dailyos_markdown_preview_lifecycle_label(
			dailyos_markdown_preview_first_string( $payload, [ 'lifecycle_state', 'lifecycleState' ], 'active' )
		);
		$trust = dailyos_markdown_preview_trust_label(
			dailyos_markdown_preview_first_string( $payload, [ 'trust_band', 'trustBand', 'trust_summary', 'trustSummary' ], 'needs_verification' )
		);
		$date = dailyos_markdown_preview_source_date( $payload );

		$out  = '<div class="dailyos-markdown-preview__header">';
		$out .= '<div>';
		$out .= '<p class="dailyos-markdown-preview__eyebrow">' . esc_html( $label ) . '</p>';
		$out .= '<h2 id="dailyos-markdown-preview-heading" class="dailyos-markdown-preview__title">' . esc_html__( 'Markdown preview', 'dailyos' ) . '</h2>';
		$out .= '</div>';
		$out .= '<dl class="dailyos-markdown-preview__meta">';
		$out .= dailyos_markdown_preview_meta_item( __( 'State', 'dailyos' ), $lifecycle );
		$out .= dailyos_markdown_preview_meta_item( __( 'Trust', 'dailyos' ), $trust );
		$out .= dailyos_markdown_preview_meta_item( __( 'Date', 'dailyos' ), $date );
		$out .= '</dl>';
		$out .= '</div>';

		return $out;
	}

	/**
	 * Render a whole-block state.
	 *
	 * @param string $state State key.
	 * @param string $message Public message.
	 * @return string
	 */
	function dailyos_markdown_preview_render_state( string $state, string $message ): string {
		$out  = '<section ' . dailyos_markdown_preview_wrapper_attrs( $state ) . ' aria-labelledby="dailyos-markdown-preview-heading">';
		$out .= '<div class="dailyos-markdown-preview__header">';
		$out .= '<div>';
		$out .= '<p class="dailyos-markdown-preview__eyebrow">' . esc_html__( 'Workspace source', 'dailyos' ) . '</p>';
		$out .= '<h2 id="dailyos-markdown-preview-heading" class="dailyos-markdown-preview__title">' . esc_html__( 'Markdown preview', 'dailyos' ) . '</h2>';
		$out .= '</div>';
		$out .= '</div>';
		$out .= '<p class="dailyos-markdown-preview__empty">' . esc_html( $message ) . '</p>';
		$out .= '</section>';
		return $out;
	}

	/**
	 * Build wrapper attributes without exposing source handles.
	 *
	 * @param string $state Public state key.
	 * @return string
	 */
	function dailyos_markdown_preview_wrapper_attrs( string $state ): string {
		return sprintf(
			'class="%s" data-dailyos-surface="markdown-preview" data-dailyos-state="%s"',
			esc_attr( 'wp-block-dailyos-markdown-preview dailyos-markdown-preview' ),
			esc_attr( dailyos_markdown_preview_safe_key( $state, 'unavailable' ) )
		);
	}

	/**
	 * Render one metadata item.
	 *
	 * @param string $term Metadata term.
	 * @param string $description Metadata value.
	 * @return string
	 */
	function dailyos_markdown_preview_meta_item( string $term, string $description ): string {
		return '<div class="dailyos-markdown-preview__metaItem">'
			. '<dt>' . esc_html( $term ) . '</dt>'
			. '<dd>' . esc_html( $description ) . '</dd>'
			. '</div>';
	}

	/**
	 * Pick the first scalar string value.
	 *
	 * @param array<string, mixed> $payload Payload.
	 * @param array<int, string>   $keys Candidate keys.
	 * @param string               $fallback Fallback.
	 * @return string
	 */
	function dailyos_markdown_preview_first_string( array $payload, array $keys, string $fallback = '' ): string {
		foreach ( $keys as $key ) {
			if ( isset( $payload[ $key ] ) && is_scalar( $payload[ $key ] ) && '' !== trim( (string) $payload[ $key ] ) ) {
				return trim( (string) $payload[ $key ] );
			}
		}
		return $fallback;
	}

	/**
	 * Sanitize display text and reject path-like values.
	 *
	 * @param mixed  $value Candidate value.
	 * @param string $fallback Fallback.
	 * @param int    $max_length Maximum output length.
	 * @return string
	 */
	function dailyos_markdown_preview_safe_text( mixed $value, string $fallback, int $max_length = 120 ): string {
		if ( ! is_scalar( $value ) ) {
			return $fallback;
		}
		$text = preg_replace( '/\s+/', ' ', trim( strip_tags( (string) $value ) ) ) ?? '';
		if ( '' === $text || str_contains( $text, '/' ) || str_contains( $text, '\\' ) ) {
			return $fallback;
		}
		if ( strlen( $text ) > $max_length ) {
			return rtrim( substr( $text, 0, $max_length - 1 ) ) . '...';
		}
		return $text;
	}

	/**
	 * Convert a token to a safe key.
	 *
	 * @param string $value Candidate key.
	 * @param string $fallback Fallback.
	 * @return string
	 */
	function dailyos_markdown_preview_safe_key( string $value, string $fallback ): string {
		$key = strtolower( trim( $value ) );
		return 1 === preg_match( '/^[a-z][a-z0-9_-]{0,63}$/', $key ) ? $key : $fallback;
	}

	/**
	 * Public lifecycle label.
	 *
	 * @param string $lifecycle Lifecycle key.
	 * @return string
	 */
	function dailyos_markdown_preview_lifecycle_label( string $lifecycle ): string {
		$key = dailyos_markdown_preview_safe_key( $lifecycle, 'active' );
		$labels = [
			'active'      => __( 'Available', 'dailyos' ),
			'pending'     => __( 'Pending', 'dailyos' ),
			'ingested'    => __( 'Available', 'dailyos' ),
			'quarantined' => __( 'Needs review', 'dailyos' ),
			'rejected'    => __( 'Ignored', 'dailyos' ),
			'deleted'     => __( 'Removed', 'dailyos' ),
			'tombstoned'  => __( 'Removed', 'dailyos' ),
		];
		return $labels[ $key ] ?? $labels['active'];
	}

	/**
	 * Public trust label.
	 *
	 * @param string $trust Trust key.
	 * @return string
	 */
	function dailyos_markdown_preview_trust_label( string $trust ): string {
		$key = dailyos_markdown_preview_safe_key( $trust, 'needs_verification' );
		$labels = [
			'likely_current'     => __( 'Likely current', 'dailyos' ),
			'use_with_caution'   => __( 'Use with caution', 'dailyos' ),
			'needs_verification' => __( 'Needs verification', 'dailyos' ),
		];
		return $labels[ $key ] ?? $labels['needs_verification'];
	}

	/**
	 * Format a source date.
	 *
	 * @param array<string, mixed> $payload Payload.
	 * @return string
	 */
	function dailyos_markdown_preview_source_date( array $payload ): string {
		$raw = dailyos_markdown_preview_first_string(
			$payload,
			[ 'source_asof', 'sourceAsof', 'last_observed_at', 'lastObservedAt' ],
			''
		);
		$timestamp = '' !== $raw ? strtotime( $raw ) : false;
		return false === $timestamp ? __( 'Date unknown', 'dailyos' ) : gmdate( 'M j, Y', (int) $timestamp );
	}
}
