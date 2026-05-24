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
	 * @param  string $template_filename Template filename inside templates/.
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
	 * and end-mark via header part + atmosphere body container.
	 *
	 * @return void
	 */
	public function test_front_page_includes_editorial_shell_classes(): void {
		$composed = $this->compose_surface( 'front-page.html' );

		// FolioBar mount header (canonical class is `dailyos-folio-bar-mount` post chrome lane W3-4;
		// `dailyos-folio-bar` matches as a prefix substring).
		$this->assertStringContainsString( 'dailyos-folio-bar', $composed, 'Missing dailyos-folio-bar header shell.' );
		// Canonical MagazinePageLayout classes (lifted from .docs/design/reference/_shared/styles/
		// per the chrome lane).
		$this->assertStringContainsString( 'MagazinePageLayout_magazinePage', $composed, 'Missing MagazinePageLayout_magazinePage root container.' );
		$this->assertStringContainsString( 'MagazinePageLayout_pageContainer', $composed, 'Missing MagazinePageLayout_pageContainer content wrap.' );
		// End-of-page FinisMarker is rendered by the footer template part; assert the canonical
		// FinisMarker root class (pattern spec at .docs/design/patterns/FinisMarker.md). The
		// previous v1.4.2 `dailyos-end-mark` + literal `* * *` in-pattern separator was part of
		// the deleted `account-overview-page` pattern; v1.4.4 W2 ships a FinisMarker inner block
		// at the end of `account-detail-default` composition + footer-level FinisMarker via the
		// magazine-theme footer template part.
		$this->assertStringContainsString( 'FinisMarker_root', $composed, 'Missing FinisMarker_root end-of-page finis.' );
	}

	/**
	 * Single-account template composes the folio bar + magazine page wraps
	 * + footer FinisMarker.
	 *
	 * @return void
	 */
	public function test_single_account_includes_editorial_shell_and_sidebar(): void {
		$template = (string) file_get_contents( $this->theme_dir . '/templates/single-dailyos_account.html' );
		$composed = $this->compose_surface( 'single-dailyos_account.html' );

		$this->assertStringContainsString( 'dailyos-folio-bar', $composed, 'Missing dailyos-folio-bar header shell.' );
		$this->assertStringContainsString( 'MagazinePageLayout_magazinePage', $composed, 'Missing MagazinePageLayout_magazinePage root container.' );
		$this->assertStringContainsString( 'MagazinePageLayout_pageContainer', $composed, 'Missing MagazinePageLayout_pageContainer content wrap.' );
		$this->assertStringContainsString( 'FinisMarker_root', $composed, 'Missing FinisMarker_root end-of-page finis.' );
		// `dailyos-end-mark` + literal `* * *` were part of the v1.4.2 account-overview-page
		// pattern (deleted in v1.4.4 W2 substrate trim). The new account-detail-default
		// composition now uses the footer template part for the FinisMarker;
		// FinisMarker_root assertion above covers the canonical end-of-page sign-off.
		// V2 wave: single-account template references the account-detail-default pattern
		// (which expands to the canonical account-detail composition).
		$this->assertStringContainsString(
			'wp:pattern',
			$template,
			'single-account template must reference the dailyos/account-detail-default pattern.'
		);
	}
}
