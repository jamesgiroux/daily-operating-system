<?php
/**
 * W3 magazine theme — baseline token shim enqueue tests.
 *
 * Asserts that the plugin enqueues `dailyos-baseline-tokens` on both the
 * front-end (`wp_enqueue_scripts`) and the block editor
 * (`enqueue_block_editor_assets`), and that the shim file declares the
 * `--wp--preset--color--*` custom properties block CSS depends on.
 *
 * Full computed-style fidelity (does the var actually resolve in the
 * browser under TwentyTwentyFive?) is verified hands-on in L4 — not
 * tractable from PHPUnit without a headless browser harness.
 *
 * Spec: L0 Packet E V1.4 §5.7 + §8.7 + invariant #7.
 *
 * @package DailyOS
 */

declare(strict_types=1);

use DailyOS\DailyOS_Plugin;
use PHPUnit\Framework\TestCase;

if ( ! function_exists( 'wp_enqueue_style' ) ) {
	/**
	 * Minimal stub mirroring core's `wp_enqueue_style()`.
	 *
	 * @param string             $handle Style handle.
	 * @param string             $src    Source URL.
	 * @param array<int, string> $deps   Dependencies.
	 * @param string|bool|null   $ver    Version.
	 * @param string             $media  Media target.
	 */
	function wp_enqueue_style( string $handle, string $src = '', array $deps = [], $ver = false, string $media = 'all' ): void {
		$GLOBALS['dailyos_test_enqueued_styles'][ $handle ] = [
			'src'   => $src,
			'deps'  => $deps,
			'ver'   => $ver,
			'media' => $media,
		];
	}
}

if ( ! function_exists( 'wp_style_is' ) ) {
	/**
	 * Minimal stub mirroring core's `wp_style_is()` for the enqueued list.
	 *
	 * @param string $handle Style handle.
	 * @param string $list   Queue name (only `enqueued` is honored).
	 */
	function wp_style_is( string $handle, string $list = 'enqueued' ): bool {
		unset( $list );
		return isset( $GLOBALS['dailyos_test_enqueued_styles'][ $handle ] );
	}
}

/**
 * Stock-theme fallback: baseline tokens shim covers every preset color
 * referenced by plugin block CSS, and is enqueued on both surfaces.
 */
final class DailyOS_StockThemeFallbackTest extends TestCase {
	/**
	 * Reset the enqueued-style registry before every test.
	 */
	protected function setUp(): void {
		parent::setUp();

		dailyos_test_reset_globals();
		$GLOBALS['dailyos_test_enqueued_styles'] = [];
	}

	/**
	 * The shim is enqueued on the public front-end (`wp_enqueue_scripts`).
	 */
	public function test_enqueue_on_wp_enqueue_scripts_registers_baseline_tokens(): void {
		DailyOS_Plugin::instance()->enqueue_baseline_tokens();

		$this->assertTrue( wp_style_is( 'dailyos-baseline-tokens' ) );
	}

	/**
	 * The shim is enqueued in the block editor (`enqueue_block_editor_assets`).
	 *
	 * The handler is hook-agnostic — the assertion is that calling the same
	 * registered handler results in the style being enqueued. The plugin's
	 * `init()` wires the same callback into both hooks at priority 9.
	 */
	public function test_enqueue_on_block_editor_assets_registers_baseline_tokens(): void {
		DailyOS_Plugin::instance()->enqueue_baseline_tokens();

		$this->assertTrue( wp_style_is( 'dailyos-baseline-tokens' ) );
		$this->assertArrayHasKey( 'dailyos-baseline-tokens', $GLOBALS['dailyos_test_enqueued_styles'] );
		$this->assertStringContainsString( 'dailyos-baseline-tokens.css', (string) $GLOBALS['dailyos_test_enqueued_styles']['dailyos-baseline-tokens']['src'] );
	}

	/**
	 * The shim CSS file exists in the plugin asset tree.
	 */
	public function test_baseline_tokens_shim_file_exists(): void {
		$path = dirname( __DIR__, 2 ) . '/assets/dailyos-baseline-tokens.css';

		$this->assertFileExists( $path );
	}

	/**
	 * The shim declares every `--wp--preset--color--*` var referenced by
	 * block CSS. If any block introduces a new preset var, the matching
	 * fallback must land in the shim before merge.
	 *
	 * @dataProvider provide_required_preset_vars
	 */
	public function test_baseline_tokens_shim_declares_required_preset_var( string $var ): void {
		$contents = (string) file_get_contents(
			dirname( __DIR__, 2 ) . '/assets/dailyos-baseline-tokens.css'
		);

		$this->assertStringContainsString( $var . ':', $contents, "shim missing declaration for $var" );
	}

	/**
	 * Required preset vars derived from `wp/dailyos/blocks/*\/style.css`.
	 *
	 * @return array<int, array<int, string>>
	 */
	public static function provide_required_preset_vars(): array {
		$required = [
			'--wp--preset--color--account',
			'--wp--preset--color--account-8',
			'--wp--preset--color--account-12',
			'--wp--preset--color--desk-charcoal-4',
			'--wp--preset--color--garden-larkspur',
			'--wp--preset--color--garden-larkspur-15',
			'--wp--preset--color--garden-rosemary',
			'--wp--preset--color--garden-rosemary-12',
			'--wp--preset--color--garden-sage',
			'--wp--preset--color--garden-sage-10',
			'--wp--preset--color--garden-sage-15',
			'--wp--preset--color--meeting',
			'--wp--preset--color--meeting-8',
			'--wp--preset--color--paper-linen',
			'--wp--preset--color--person',
			'--wp--preset--color--person-8',
			'--wp--preset--color--project',
			'--wp--preset--color--project-8',
			'--wp--preset--color--spice-chili',
			'--wp--preset--color--spice-saffron',
			'--wp--preset--color--spice-saffron-10',
			'--wp--preset--color--spice-terracotta',
			'--wp--preset--color--spice-terracotta-10',
			'--wp--preset--color--spice-terracotta-15',
			'--wp--preset--color--spice-turmeric',
			'--wp--preset--color--spice-turmeric-15',
			'--wp--preset--color--text-primary',
			'--wp--preset--color--text-quaternary',
			'--wp--preset--color--text-secondary',
			'--wp--preset--color--text-tertiary',
			'--wp--preset--color--trust-likely-current',
			'--wp--preset--color--trust-likely-current-12',
			'--wp--preset--color--trust-likely-current-15',
			'--wp--preset--color--trust-needs-verification',
			'--wp--preset--color--trust-needs-verification-12',
			'--wp--preset--color--trust-needs-verification-15',
			'--wp--preset--color--trust-use-with-caution',
			'--wp--preset--color--trust-use-with-caution-12',
			'--wp--preset--color--trust-use-with-caution-15',
		];

		return array_map( static fn( string $var ): array => [ $var ], $required );
	}
}
