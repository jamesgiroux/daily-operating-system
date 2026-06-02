<?php
/**
 * Block pipeline render regression test.
 *
 * Asserts that every registered `dailyos/*` block produces non-empty
 * output when rendered through the WP block pipeline (`do_blocks`),
 * not just when its render function is called directly.
 *
 * Catches the render.php contract bug where files end with
 * `return dailyos_*_render( $attributes );` — WP core's render
 * callback (`register_block_type_from_metadata` at
 * wp-includes/blocks.php:569) uses `ob_start(); require $path;
 * return ob_get_clean();` which captures echo output, not the
 * file's return value. Every render.php must `echo` the rendered
 * HTML, not `return` it.
 *
 * @package DailyOS
 */

declare(strict_types=1);

use PHPUnit\Framework\TestCase;

/**
 * Smoke-tests the block render pipeline for every dailyos/* block.
 */
final class DailyOS_BlockPipelineRenderTest extends TestCase {
	/**
	 * Minimal attribute payloads for each block. Empty-state output is
	 * acceptable; the test only asserts non-empty render.
	 *
	 * @return array<string, array{0: string, 1: array<string, mixed>}>
	 */
	public static function block_render_provider(): array {
		return [
			'avatar'                     => [
				'dailyos/avatar',
				[],
			],
			'entity-chip'                => [
				'dailyos/entity-chip',
				[ 'entityType' => 'account' ],
			],
			'freshness-indicator'        => [
				'dailyos/freshness-indicator',
				[],
			],
			'health-badge'               => [
				'dailyos/health-badge',
				[
					'score' => 0,
					'band'  => 'green',
				],
			],
			'intelligence-quality-badge' => [
				'dailyos/intelligence-quality-badge',
				[],
			],
			'pill'                       => [
				'dailyos/pill',
				[],
			],
			'provenance-tag'             => [
				'dailyos/provenance-tag',
				[],
			],
			'score-band'                 => [
				'dailyos/score-band',
				[],
			],
			'status-dot'                 => [
				'dailyos/status-dot',
				[],
			],
			'trust-band-badge'           => [
				'dailyos/trust-band-badge',
				[],
			],
			'type-badge'                 => [
				'dailyos/type-badge',
				[ 'accountType' => 'customer' ],
			],
		];
	}

	/**
	 * Asserts each block produces non-empty output through the WP
	 * block pipeline. The empty-state HTML is allowed — what's banned
	 * is a literally empty render result (which is what a broken
	 * render.php produces).
	 *
	 * @dataProvider block_render_provider
	 *
	 * @param string               $block_name Block name (e.g. dailyos/pill).
	 * @param array<string, mixed> $attributes Block attributes.
	 *
	 * @return void
	 */
	public function test_block_renders_non_empty_through_pipeline(
		string $block_name,
		array $attributes
	): void {
		if ( ! function_exists( 'do_blocks' ) || ! class_exists( 'WP_Block_Type_Registry' ) ) {
			$this->markTestSkipped( 'WP block API not loaded — run under wp test harness.' );
		}

		$registered = WP_Block_Type_Registry::get_instance()->get_registered( $block_name );
		$this->assertNotNull( $registered, "Block not registered: {$block_name}" );
		$this->assertIsCallable( $registered->render_callback, "Block has no render callback: {$block_name}" );

		$attrs_json = wp_json_encode( $attributes );
		$markup     = sprintf(
			'<!-- wp:%s %s /-->',
			$block_name,
			false === $attrs_json ? '{}' : $attrs_json
		);

		$rendered = do_blocks( $markup );

		$this->assertNotSame(
			'',
			trim( $rendered ),
			"Block {$block_name} rendered empty through the pipeline. Check render.php: it must `echo` the rendered HTML, not `return` it. WP core's render callback uses ob_start()/require/ob_get_clean() — return values are discarded."
		);
	}
}
