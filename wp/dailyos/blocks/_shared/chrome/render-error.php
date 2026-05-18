<?php
/**
 * Primitive chrome — error state partial.
 *
 * Consumed by per-primitive render-functions.php files when the projection
 * returns an error state. v1.4.3 W2 L0 Packet D §5.8.
 *
 * @package DailyOS
 * @param string|null $label Optional label override (default "Error").
 */

declare(strict_types=1);

if ( ! function_exists( 'dailyos_chrome_render_error' ) ) {
	/**
	 * Render the canonical primitive-tier error chrome.
	 *
	 * @param string|null $label Optional label override.
	 * @return string Rendered HTML.
	 */
	function dailyos_chrome_render_error( ?string $label = null ): string {
		$label_text = $label ?? 'Error';
		return sprintf(
			'<span class="dailyos-chrome-error" data-chrome="error" data-ds-name="PrimitiveChrome" data-ds-spec="primitives/_chrome/README.md">%s</span>',
			esc_html( $label_text )
		);
	}
}
