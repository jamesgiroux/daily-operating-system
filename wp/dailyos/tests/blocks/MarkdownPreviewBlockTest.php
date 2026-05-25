<?php
/**
 * Markdown Preview block tests.
 *
 * @package DailyOS
 */

declare(strict_types=1);

use PHPUnit\Framework\TestCase;

require_once __DIR__ . '/../../blocks/markdown-preview/render-functions.php';

/**
 * Tests for the dailyos/markdown-preview WP block render path.
 */
final class DailyOS_MarkdownPreviewBlockTest extends TestCase {
	protected function setUp(): void {
		parent::setUp();
		if ( function_exists( 'dailyos_test_reset_globals' ) ) {
			dailyos_test_reset_globals();
		}
	}

	public function test_block_json_uses_dailyos_category_and_dynamic_render(): void {
		$block_json = json_decode(
			(string) file_get_contents( __DIR__ . '/../../blocks/markdown-preview/block.json' ),
			true
		);

		$this->assertSame( 'dailyos/markdown-preview', $block_json['name'] );
		$this->assertSame( 'dailyos', $block_json['category'] );
		$this->assertSame( 'file:./render.php', $block_json['render'] );
		$this->assertArrayHasKey( 'source_handle', $block_json['attributes'] );
		$this->assertArrayNotHasKey( 'raw_markdown', $block_json['attributes'] );
		$this->assertArrayNotHasKey( 'preview_html', $block_json['attributes'] );
	}

	public function test_default_render_is_unavailable_without_runtime_projection(): void {
		$html = dailyos_markdown_preview_render(
			[
				'source_handle' => 'src_handle_123',
			]
		);

		$this->assertStringContainsString( 'data-dailyos-surface="markdown-preview"', $html );
		$this->assertStringContainsString( 'data-dailyos-state="unavailable"', $html );
		$this->assertStringContainsString( 'Preview is temporarily unavailable.', $html );
		$this->assertStringNotContainsString( 'src_handle_123', $html );
	}

	public function test_payload_render_sanitizes_preview_html_and_redacts_raw_source_values(): void {
		$html = dailyos_markdown_preview_render_payload(
			[
				'source_handle'     => 'src_handle_123',
				'file_id'           => 'file_123',
				'path'              => '/Users/example/workspace/source.md',
				'source_label'      => 'workspace/source/private.md',
				'preview_html'      => '<h1>Preview</h1><p>Body</p><script>alert(1)</script><img src="http://example.invalid/track.png">',
				'lifecycle_state'   => 'quarantined',
				'trust_band'        => 'use_with_caution',
				'source_asof'       => '2026-05-24T10:00:00Z',
			]
		);

		$this->assertStringContainsString( '<h1>Preview</h1>', $html );
		$this->assertStringContainsString( 'Workspace source', $html );
		$this->assertStringContainsString( 'Needs review', $html );
		$this->assertStringContainsString( 'Use with caution', $html );
		$this->assertStringContainsString( 'May 24, 2026', $html );
		$this->assertStringContainsString( '[remote asset blocked]', $html );
		$this->assertStringNotContainsString( '<script', $html );
		$this->assertStringNotContainsString( 'alert(1)', $html );
		$this->assertStringNotContainsString( 'http://example.invalid', $html );
		$this->assertStringNotContainsString( 'src_handle_123', $html );
		$this->assertStringNotContainsString( 'file_123', $html );
		$this->assertStringNotContainsString( '/Users/example/workspace/source.md', $html );
		$this->assertStringNotContainsString( 'workspace/source/private.md', $html );
	}
}
