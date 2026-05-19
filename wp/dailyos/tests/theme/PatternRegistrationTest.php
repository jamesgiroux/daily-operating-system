<?php
/**
 * Pattern registration tests for the DailyOS magazine theme.
 *
 * Verifies that both FSE patterns exist on disk, lint cleanly as PHP,
 * declare the expected `Slug:` header, and only reference block types
 * that actually exist in `wp/dailyos/blocks/<name>/` (for `dailyos/*`
 * blocks) — per L0 Packet E §5.4.
 *
 * @package DailyOS
 */

declare(strict_types=1);

use PHPUnit\Framework\TestCase;

/**
 * Asserts pattern file presence, header metadata, and block-reference
 * integrity for the magazine theme.
 */
final class DailyOS_PatternRegistrationTest extends TestCase {
	private string $theme_dir;
	private string $blocks_dir;

	/**
	 * Resolves theme + blocks directories once per test.
	 */
	protected function setUp(): void {
		parent::setUp();
		$this->theme_dir  = dirname( __DIR__, 2 ) . '/theme';
		$this->blocks_dir = dirname( __DIR__, 2 ) . '/blocks';
	}

	/**
	 * Asserts both pattern PHP files exist on disk.
	 *
	 * @return void
	 */
	public function test_pattern_files_exist(): void {
		$patterns = [ 'account-overview-page.php', 'briefing-page.php' ];

		foreach ( $patterns as $pattern ) {
			$path = $this->theme_dir . '/patterns/' . $pattern;
			$this->assertFileExists( $path, "Missing pattern: {$pattern}" );
		}
	}

	/**
	 * Asserts each pattern file parses as valid PHP (no syntax errors).
	 *
	 * @return void
	 */
	public function test_pattern_files_parse_as_php(): void {
		$patterns = [ 'account-overview-page.php', 'briefing-page.php' ];

		foreach ( $patterns as $pattern ) {
			$path   = $this->theme_dir . '/patterns/' . $pattern;
			$output = [];
			$status = 1;
			$cmd    = sprintf( 'php -l %s 2>&1', escapeshellarg( $path ) );
			exec( $cmd, $output, $status );

			$this->assertSame( 0, $status, "PHP lint failed for {$pattern}: " . implode( "\n", $output ) );
		}
	}

	/**
	 * Asserts each pattern declares the expected `Slug:` header.
	 *
	 * @return void
	 */
	public function test_pattern_headers_declare_expected_slugs(): void {
		$expected = [
			'account-overview-page.php' => 'dailyos/account-overview-page',
			'briefing-page.php'         => 'dailyos/briefing-page',
		];

		foreach ( $expected as $file => $slug ) {
			$path     = $this->theme_dir . '/patterns/' . $file;
			$contents = (string) file_get_contents( $path );

			$this->assertMatchesRegularExpression(
				'/^\s*\*\s*Slug:\s*' . preg_quote( $slug, '/' ) . '\s*$/m',
				$contents,
				"Pattern {$file} missing Slug header for {$slug}"
			);
			$this->assertMatchesRegularExpression(
				'/^\s*\*\s*Title:\s*\S+/m',
				$contents,
				"Pattern {$file} missing Title header"
			);
		}
	}

	/**
	 * Asserts every `dailyos/*` block referenced inside the pattern bodies
	 * exists as a `wp/dailyos/blocks/<name>/` directory. Guards against
	 * patterns referencing renamed or unbuilt blocks. Core blocks
	 * (`core/*`, `wp:heading`, `wp:paragraph`, `wp:separator`, etc.) are
	 * intentionally not validated against disk — they ship with WP.
	 *
	 * @return void
	 */
	public function test_pattern_bodies_reference_existing_dailyos_blocks(): void {
		$patterns = [ 'account-overview-page.php', 'briefing-page.php' ];

		foreach ( $patterns as $pattern ) {
			$path     = $this->theme_dir . '/patterns/' . $pattern;
			$contents = (string) file_get_contents( $path );

			// Match `wp:dailyos/<name>` mentions (self-closing or open).
			preg_match_all( '/wp:dailyos\/([a-z0-9][a-z0-9-]*)/i', $contents, $matches );
			$referenced = array_unique( $matches[1] ?? [] );

			foreach ( $referenced as $block_name ) {
				$block_dir = $this->blocks_dir . '/' . $block_name;
				$this->assertDirectoryExists(
					$block_dir,
					"Pattern {$pattern} references dailyos/{$block_name}, but {$block_dir} does not exist."
				);
			}
		}
	}
}
