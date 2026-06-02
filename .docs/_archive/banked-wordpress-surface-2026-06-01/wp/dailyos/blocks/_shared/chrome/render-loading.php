<?php
/**
 * Primitive chrome — loading state partial.
 *
 * Consumed by per-primitive render-functions.php files when the projected
 * payload has no claim_refs and the projection result is unresolved/pending.
 * For v1.4.3 W2 the loading state is surface-derived (empty claim_refs +
 * pending projection); producer-side render_hints.chrome_state adoption
 * is deferred to v1.4.4 W4.
 *
 * v1.4.3 W2 L0 Packet D §5.8.
 *
 * @package DailyOS
 * @param string|null $label Optional label override (default "Loading").
 */

declare(strict_types=1);

if ( ! function_exists( 'dailyos_chrome_render_loading' ) ) {
	/**
	 * Render the canonical primitive-tier loading chrome.
	 *
	 * @param string|null $label Optional label override.
	 * @return string Rendered HTML.
	 */
	function dailyos_chrome_render_loading( ?string $label = null ): string {
		$label_text = $label ?? 'Loading';
		return sprintf(
			'<span class="dailyos-chrome-loading" data-chrome="loading" data-ds-name="PrimitiveChrome" data-ds-spec="primitives/_chrome/README.md">%s</span>',
			esc_html( $label_text )
		);
	}
}
