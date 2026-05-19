<?php
/**
 * Editorial shell presence tests for the DailyOS magazine theme.
 *
 * Per L0 Packet E §8.8: the magazine chrome (folio bar header,
 * atmosphere wrapper, magazine page layout, end-mark separator,
 * account summary sidebar) must be wired through templates + parts +
 * patterns. This test reads template + part + pattern source directly
 * and asserts the editorial-shell class hooks are referenced.
 *
 * The "render" step here is intentionally a flat `file_get_contents`
 * + string-presence assertion — full WP block-rendering through
 * `do_blocks()` requires the WP runtime and is deferred to L2 / L4
 * verification once Studio is wired.
 *
 * @package DailyOS
 */

declare(strict_types=1);

use PHPUnit\Framework\TestCase;

/**
 * Asserts the editorial shell classes are present across the surfaces
 * named in the spec (folio bar in header part, atmosphere on body
 * group, magazine-page on main column, end-mark in account pattern,
 * sidebar template-part on single-account).
 */
final class DailyOS_EditorialShellPresenceTest extends TestCase {
	private string $theme_dir;

	/**
	 * Resolves the theme dir once per test.
	 */
	protected function setUp(): void {
		parent::setUp();
		$this->theme_dir = dirname( __DIR__, 2 ) . '/theme';
	}

	/**
	 * Joins the full editorial render surface for a template by reading
	 * the template body plus every template-part / pattern it references.
	 * This avoids needing `do_blocks()` while still asserting the chrome
	 * actually composes from the parts it claims to compose from.
	 *
	 * @param string $template_filename Template filename inside templates/.
	 * @return string Concatenated source of template + referenced parts + patterns.
	 */
	private function compose_surface( string $template_filename ): string {
		$template_path = $this->theme_dir . '/templates/' . $template_filename;
		$this->assertFileExists( $template_path, "Missing template: {$template_filename}" );

		$body  = (string) file_get_contents( $template_path );
		$bag   = $body;
		$parts = [];

		// Resolve referenced template parts.
		if ( preg_match_all( '/wp:template-part\s*{\s*"slug":"([a-z0-9-]+)"/i', $body, $m ) ) {
			$parts = array_unique( $m[1] );
		}
		foreach ( $parts as $slug ) {
			$part_path = $this->theme_dir . '/parts/' . $slug . '.html';
			if ( is_file( $part_path ) ) {
				$bag .= "\n" . (string) file_get_contents( $part_path );
			}
		}

		// Resolve referenced patterns.
		$patterns = [];
		if ( preg_match_all( '/wp:pattern\s*{\s*"slug":"dailyos\/([a-z0-9-]+)"/i', $body, $pm ) ) {
			$patterns = array_unique( $pm[1] );
		}
		foreach ( $patterns as $slug ) {
			$pattern_path = $this->theme_dir . '/patterns/' . $slug . '.php';
			if ( is_file( $pattern_path ) ) {
				$bag .= "\n" . (string) file_get_contents( $pattern_path );
			}
		}

		return $bag;
	}

	/**
	 * Front-page composes the folio bar, atmosphere body, magazine page,
	 * and end-mark via header part + account-overview-page pattern.
	 *
	 * @return void
	 */
	public function test_front_page_includes_editorial_shell_classes(): void {
		$composed = $this->compose_surface( 'front-page.html' );

		$this->assertStringContainsString( 'dailyos-folio-bar', $composed, 'Missing dailyos-folio-bar header shell.' );
		$this->assertStringContainsString( 'dailyos-atmosphere', $composed, 'Missing dailyos-atmosphere body wrapper.' );
		$this->assertStringContainsString( 'dailyos-magazine-page', $composed, 'Missing dailyos-magazine-page layout class.' );
		$this->assertStringContainsString( 'dailyos-end-mark', $composed, 'Missing dailyos-end-mark separator.' );
		$this->assertStringContainsString( '* * *', $composed, 'Missing literal end-mark glyph.' );
	}

	/**
	 * Single-account template composes the folio bar + atmosphere +
	 * magazine page + end-mark AND mounts the account summary sidebar.
	 *
	 * @return void
	 */
	public function test_single_account_includes_editorial_shell_and_sidebar(): void {
		$template = (string) file_get_contents( $this->theme_dir . '/templates/single-dailyos-account.html' );
		$composed = $this->compose_surface( 'single-dailyos-account.html' );

		$this->assertStringContainsString( 'dailyos-folio-bar', $composed, 'Missing dailyos-folio-bar header shell.' );
		$this->assertStringContainsString( 'dailyos-atmosphere', $composed, 'Missing dailyos-atmosphere body wrapper.' );
		$this->assertStringContainsString( 'dailyos-magazine-page', $composed, 'Missing dailyos-magazine-page layout class.' );
		$this->assertStringContainsString( 'dailyos-end-mark', $composed, 'Missing dailyos-end-mark separator.' );
		$this->assertStringContainsString(
			'template-part {"slug":"sidebar-account-summary"}',
			$template,
			'single-account template must mount sidebar-account-summary part.'
		);
	}
}
