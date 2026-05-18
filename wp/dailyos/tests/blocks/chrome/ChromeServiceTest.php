<?php
/**
 * Primitive chrome service — PHPUnit coverage per v1.4.3 W2 L0
 * Packet D §5.8 + AC #15 (DOS-682).
 *
 * Asserts each chrome partial:
 *  - renders a `data-chrome="<state>"` marker matching the partial name
 *  - emits NO inline `style=` attribute (cardinal rule per memory
 *    feedback_no_inline_css.md)
 *  - escapes the label per WP esc_html() discipline
 *
 * Does NOT depend on per-primitive `wp/dailyos/blocks/<slug>/` dirs (which
 * land in PR-D2/D3/D4). Tests own the fixture data per V1.4 §5.8 surgical
 * fold.
 *
 * @package DailyOS
 */

declare(strict_types=1);

use PHPUnit\Framework\TestCase;

require_once dirname( __DIR__, 4 ) . '/blocks/_shared/chrome/render-empty.php';
require_once dirname( __DIR__, 4 ) . '/blocks/_shared/chrome/render-loading.php';
require_once dirname( __DIR__, 4 ) . '/blocks/_shared/chrome/render-error.php';

/**
 * Cover all three chrome partials + the no-inline-style invariant.
 */
final class DailyOS_ChromeServiceTest extends TestCase {
	/**
	 * Polyfill esc_html() if PHPUnit isn't running under WP's runtime.
	 */
	protected function setUp(): void {
		parent::setUp();
		if ( ! function_exists( 'esc_html' ) ) {
			function esc_html( $text ) {
				return htmlspecialchars( (string) $text, ENT_QUOTES | ENT_HTML5, 'UTF-8' );
			}
		}
	}

	/**
	 * Empty partial emits the `data-chrome="empty"` marker + canonical label.
	 */
	public function test_render_empty_default_label(): void {
		$html = dailyos_chrome_render_empty();
		$this->assertStringContainsString( 'data-chrome="empty"', $html );
		$this->assertStringContainsString( '—', $html );
		$this->assertStringContainsString( 'data-ds-name="PrimitiveChrome"', $html );
	}

	/**
	 * Empty partial accepts a label override and escapes it.
	 */
	public function test_render_empty_label_override_is_escaped(): void {
		$html = dailyos_chrome_render_empty( '<script>x</script>' );
		$this->assertStringContainsString( 'data-chrome="empty"', $html );
		$this->assertStringNotContainsString( '<script>', $html );
		$this->assertStringContainsString( '&lt;script&gt;', $html );
	}

	/**
	 * Loading partial emits the `data-chrome="loading"` marker + canonical label.
	 */
	public function test_render_loading_default_label(): void {
		$html = dailyos_chrome_render_loading();
		$this->assertStringContainsString( 'data-chrome="loading"', $html );
		$this->assertStringContainsString( 'Loading', $html );
	}

	/**
	 * Error partial emits the `data-chrome="error"` marker + canonical label.
	 */
	public function test_render_error_default_label(): void {
		$html = dailyos_chrome_render_error();
		$this->assertStringContainsString( 'data-chrome="error"', $html );
		$this->assertStringContainsString( 'Error', $html );
	}

	/**
	 * Every chrome partial MUST emit zero inline `style=` attributes.
	 *
	 * Cardinal no-inline-CSS rule per memory feedback_no_inline_css.md +
	 * V1.4 §5.8 chrome contract item #3 (MUST NOT consume editorial/EmptyState).
	 */
	public function test_no_partial_emits_inline_style_attribute(): void {
		$partials = [
			'empty'   => dailyos_chrome_render_empty(),
			'loading' => dailyos_chrome_render_loading(),
			'error'   => dailyos_chrome_render_error( 'Boom' ),
		];
		foreach ( $partials as $name => $html ) {
			$this->assertStringNotContainsString(
				'style=',
				$html,
				"chrome partial '{$name}' emitted an inline style attribute; primitive chrome MUST consume tokens via CSS classes only"
			);
		}
	}

	/**
	 * Chrome marker classes are disjoint — empty/loading/error MUST NOT
	 * collide in the rendered HTML data-chrome attribute.
	 */
	public function test_chrome_state_markers_are_disjoint(): void {
		$empty   = dailyos_chrome_render_empty();
		$loading = dailyos_chrome_render_loading();
		$error   = dailyos_chrome_render_error();

		$this->assertStringNotContainsString( 'data-chrome="loading"', $empty );
		$this->assertStringNotContainsString( 'data-chrome="error"', $empty );
		$this->assertStringNotContainsString( 'data-chrome="empty"', $loading );
		$this->assertStringNotContainsString( 'data-chrome="error"', $loading );
		$this->assertStringNotContainsString( 'data-chrome="empty"', $error );
		$this->assertStringNotContainsString( 'data-chrome="loading"', $error );
	}
}
