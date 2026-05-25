<?php
/**
 * Markdown sanitizer tests.
 *
 * @package DailyOS
 */

declare(strict_types=1);

use DailyOS\DailyOS_Markdown_Sanitizer;
use PHPUnit\Framework\TestCase;

require_once __DIR__ . '/../includes/class-dailyos-markdown-sanitizer.php';

/**
 * Tests for the DailyOS markdown preview sanitizer.
 */
final class DailyOS_MarkdownSanitizerTest extends TestCase {
	private DailyOS_Markdown_Sanitizer $sanitizer;

	protected function setUp(): void {
		parent::setUp();
		$this->sanitizer = new DailyOS_Markdown_Sanitizer();
	}

	/**
	 * @return array<string, array{0:string,1:array<int,string>}>
	 */
	public static function dangerous_html_provider(): array {
		return [
			'script'       => [ '<script>alert(1)</script>', [ '<script', 'alert(1)' ] ],
			'event'        => [ '<img src="dailyos-asset://safe" onerror="alert(1)">', [ 'onerror', 'alert(1)' ] ],
			'javascript'   => [ '<a href="javascript:alert(1)">bad</a>', [ 'javascript:', 'alert(1)' ] ],
			'data_href'    => [ '<a href="data:text/html,<script>alert(1)</script>">bad</a>', [ 'data:text/html', '<script', 'alert(1)' ] ],
			'data_img'     => [ '<img src="data:image/svg+xml,<svg onload=alert(1)>">', [ 'data:image', 'onload', 'alert(1)' ] ],
			'svg'          => [ '<svg><script>alert(1)</script></svg>', [ '<svg', '<script', 'alert(1)' ] ],
			'math'         => [ '<math><mtext><script>alert(1)</script></mtext></math>', [ '<math', '<script', 'alert(1)' ] ],
			'foreign'      => [ '<foreignObject><body onload=alert(1)></body></foreignObject>', [ 'foreignObject', 'onload', 'alert(1)' ] ],
			'style_attr'   => [ '<p style="background:url(javascript:alert(1))">x</p>', [ 'style=', 'javascript:', 'alert(1)' ] ],
			'style_tag'    => [ '<style>@import url(http://example.invalid/style.css)</style>', [ '<style', '@import', 'example.invalid' ] ],
			'link'         => [ '<link rel="stylesheet" href="http://example.invalid/style.css">', [ '<link', 'example.invalid' ] ],
			'object'       => [ '<object data="http://example.invalid/payload"></object>', [ '<object', 'example.invalid' ] ],
			'embed'        => [ '<embed src="http://example.invalid/payload">', [ '<embed', 'example.invalid' ] ],
			'iframe'       => [ '<iframe src="http://example.invalid/payload"></iframe>', [ '<iframe', 'example.invalid' ] ],
			'comment'      => [ '<!--[if IE]><script>alert(1)</script><![endif]-->', [ '<!--', '<script', 'alert(1)' ] ],
			'remote_image' => [ '<img src="http://example.invalid/track.png">', [ 'http://example.invalid', '<img' ] ],
		];
	}

	/**
	 * @dataProvider dangerous_html_provider
	 *
	 * @param string            $html Input HTML.
	 * @param array<int,string> $forbidden Forbidden output fragments.
	 */
	public function test_sanitizer_removes_dangerous_html( string $html, array $forbidden ): void {
		$output = $this->sanitizer->sanitize( $html );

		foreach ( $forbidden as $needle ) {
			$this->assertStringNotContainsString( $needle, $output );
		}
	}

	public function test_sanitizer_preserves_safe_markdown_html_and_link_policy(): void {
		$output = $this->sanitizer->sanitize(
			'<h1>Title</h1><p>A <strong>safe</strong> paragraph with <a href="https://example.invalid/page">a link</a>.</p><pre><code>code</code></pre><table><thead><tr><th>H</th></tr></thead><tbody><tr><td>C</td></tr></tbody></table>'
		);

		$this->assertStringContainsString( '<h1>Title</h1>', $output );
		$this->assertStringContainsString( '<strong>safe</strong>', $output );
		$this->assertStringContainsString( 'href="https://example.invalid/page"', $output );
		$this->assertStringContainsString( 'rel="nofollow noopener noreferrer"', $output );
		$this->assertStringContainsString( '<table>', $output );
	}

	public function test_dailyos_asset_image_is_blocked_until_resolver_is_available(): void {
		$output = $this->sanitizer->sanitize( '<img src="dailyos-asset://asset-1" alt="Preview">' );

		$this->assertSame( DailyOS_Markdown_Sanitizer::LOCAL_ASSET_PLACEHOLDER, $output );
		$this->assertStringNotContainsString( 'dailyos-asset://asset-1', $output );
	}

	public function test_dailyos_asset_image_can_be_preserved_when_resolver_is_available(): void {
		$output = $this->sanitizer->sanitize(
			'<img src="dailyos-asset://asset-1" alt="Preview">',
			[ 'allow_dailyos_asset_images' => true ]
		);

		$this->assertStringContainsString( 'src="dailyos-asset://asset-1"', $output );
		$this->assertStringContainsString( 'alt="Preview"', $output );
	}

	public function test_unknown_tags_drop_their_full_subtree(): void {
		$output = $this->sanitizer->sanitize( '<p>Before</p><section><strong>Hidden</strong></section><p>After</p>' );

		$this->assertStringContainsString( '<p>Before</p>', $output );
		$this->assertStringContainsString( '<p>After</p>', $output );
		$this->assertStringNotContainsString( 'Hidden', $output );
		$this->assertStringNotContainsString( '<section', $output );
	}

	public function test_fail_closed_escapes_the_entire_fragment(): void {
		$output = $this->sanitizer->sanitize(
			'<p>Safe</p><script>alert(1)</script>',
			[ 'force_fail_closed' => true ]
		);

		$this->assertStringContainsString( 'data-sanitizer-state="fail_closed"', $output );
		$this->assertStringContainsString( '&lt;script&gt;alert(1)&lt;/script&gt;', $output );
		$this->assertStringNotContainsString( '<script>', $output );
	}
}
