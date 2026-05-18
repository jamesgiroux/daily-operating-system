<?php
/**
 * Primitive chrome — empty state partial.
 *
 * Consumed by per-primitive render-functions.php files when the projected
 * payload has no resolvable claim_refs. v1.4.3 W2 L0 Packet D §5.8.
 *
 * @package DailyOS
 * @param string|null $label Optional label override (default "—").
 */

declare(strict_types=1);

if ( ! function_exists( 'dailyos_chrome_render_empty' ) ) {
	/**
	 * Render the canonical primitive-tier empty chrome.
	 *
	 * @param string|null $label Optional label override.
	 * @return string Rendered HTML.
	 */
	function dailyos_chrome_render_empty( ?string $label = null ): string {
		$label_text = $label ?? '—';
		return sprintf(
			'<span class="dailyos-chrome-empty" data-chrome="empty" data-ds-name="PrimitiveChrome" data-ds-spec="primitives/_chrome/README.md">%s</span>',
			esc_html( $label_text )
		);
	}
}
