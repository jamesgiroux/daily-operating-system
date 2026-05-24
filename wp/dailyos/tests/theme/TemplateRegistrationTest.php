<?php
/**
 * Template registration tests for the DailyOS magazine theme.
 *
 * Verifies that the FSE block-theme templates exist on disk, that
 * the three required template parts (header, footer, sidebar-account-summary)
 * are present, and that theme.json keeps the `customTemplates` array empty —
 * per L0 Packet E §5.2 / §14 AC N. Custom templates are CPT-bound via
 * `single-dailyos_account.html`, `archive-dailyos_account.html`, and
 * `single-dailyos_briefing.html` filenames (underscore matches the
 * `dailyos_account` / `dailyos_briefing` post_type slug — WP template
 * hierarchy uses the literal post_type, not a hyphenated alias), not via
 * `customTemplates` entries.
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
	 * Asserts the FSE templates used by mock parity surfaces are present on disk.
	 *
	 * @return void
	 */
	public function test_required_templates_exist_on_disk(): void {
		$templates = [
			'index.html',
			'front-page.html',
			'single-dailyos_account.html',
			'single-dailyos_project.html',
			'single-dailyos_person.html',
			'single-dailyos_meeting.html',
			'archive-dailyos_account.html',
			'single-dailyos_briefing.html',
			'page-actions.html',
			'page-emails.html',
			'page-mock-surfaces.html',
		];

		foreach ( $templates as $template ) {
			$path = $this->theme_dir . '/templates/' . $template;
			$this->assertFileExists( $path, "Missing template: {$template}" );
			$this->assertNotSame( '', trim( (string) file_get_contents( $path ) ), "Empty template: {$template}" );
		}
	}

	/**
	 * Asserts the mock-surface review page points at every seeded surface.
	 *
	 * @return void
	 */
	public function test_mock_surfaces_template_links_seeded_review_targets(): void {
		$template_path = $this->theme_dir . '/templates/page-mock-surfaces.html';
		$this->assertFileExists( $template_path );

		$template = (string) file_get_contents( $template_path );
		foreach (
			[
				'/accounts/acme-corp/',
				'/entities/projects/beta-migration/',
				'/entities/people/priya-raman/',
				'/entities/meetings/mtg-acme-renewal-checkpoint/',
				'/briefings/briefing-today/',
				'/actions/',
				'/emails/',
			] as $href
		) {
			$this->assertStringContainsString( 'href="' . $href . '"', $template );
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
