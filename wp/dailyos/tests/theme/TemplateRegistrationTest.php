<?php
/**
 * Template registration tests for the DailyOS magazine theme.
 *
 * Verifies that the five FSE block-theme templates exist on disk, that
 * the three required template parts (header, footer, sidebar-account-summary)
 * are present, and that theme.json keeps the `customTemplates` array empty —
 * per L0 Packet E §5.2 / §14 AC N. Custom templates are CPT-bound via
 * `single-dailyos-account.html`, `archive-dailyos-account.html`, and
 * `single-dailyos-briefing.html` filenames, not via `customTemplates` entries.
 *
 * @package DailyOS
 */

declare(strict_types=1);

use PHPUnit\Framework\TestCase;

/**
 * Asserts theme template / template-part file presence and theme.json
 * customTemplates discipline.
 */
final class DailyOS_TemplateRegistrationTest extends TestCase {
	private string $theme_dir;

	/**
	 * Resolves the theme directory once per test.
	 */
	protected function setUp(): void {
		parent::setUp();
		$this->theme_dir = dirname( __DIR__, 2 ) . '/theme';
	}

	/**
	 * Asserts all five FSE templates are present on disk.
	 *
	 * @return void
	 */
	public function test_required_templates_exist_on_disk(): void {
		$templates = [
			'index.html',
			'front-page.html',
			'single-dailyos-account.html',
			'archive-dailyos-account.html',
			'single-dailyos-briefing.html',
		];

		foreach ( $templates as $template ) {
			$path = $this->theme_dir . '/templates/' . $template;
			$this->assertFileExists( $path, "Missing template: {$template}" );
			$this->assertNotSame( '', trim( (string) file_get_contents( $path ) ), "Empty template: {$template}" );
		}
	}

	/**
	 * Asserts the three required template parts are present.
	 *
	 * @return void
	 */
	public function test_required_template_parts_exist_on_disk(): void {
		$parts = [ 'header.html', 'footer.html', 'sidebar-account-summary.html' ];

		foreach ( $parts as $part ) {
			$path = $this->theme_dir . '/parts/' . $part;
			$this->assertFileExists( $path, "Missing template part: {$part}" );
			$this->assertNotSame( '', trim( (string) file_get_contents( $path ) ), "Empty template part: {$part}" );
		}
	}

	/**
	 * Asserts theme.json keeps customTemplates empty. CPT templates use
	 * filename binding (single-{cpt}.html / archive-{cpt}.html); the
	 * customTemplates array would only be needed for user-selectable
	 * alternative page templates, which DailyOS does not expose.
	 *
	 * @return void
	 */
	public function test_theme_json_custom_templates_is_empty(): void {
		$theme_json_path = $this->theme_dir . '/theme.json';
		$this->assertFileExists( $theme_json_path, 'theme.json missing' );

		$decoded = json_decode( (string) file_get_contents( $theme_json_path ), true );
		$this->assertIsArray( $decoded, 'theme.json failed to decode' );
		$this->assertArrayHasKey( 'customTemplates', $decoded, 'customTemplates key missing from theme.json' );
		$this->assertSame(
			[],
			$decoded['customTemplates'],
			'theme.json#customTemplates must remain [] — CPT templates bind by filename.'
		);
	}
}
