<?php
/**
 * V1.4.4 W2 L4 wiring: post-context auto-fill regression tests.
 *
 * Per the W2 L4 acceptance criteria: when the outer-block attribute is
 * empty AND we're rendering inside the matching CPT, the renderer falls
 * back to `dailyos_entity_id` post-meta first, then post slug. This
 * lets editors create a `dailyos_<entity>` post and have the W2 surface
 * render against the runtime without editor-side block-attribute wiring.
 *
 * Each of the four outer blocks (account/project/person/meeting) is
 * tested for:
 *  - meta-id wins when both meta + slug are present
 *  - slug fallback when meta is empty
 *  - no auto-fill when the post type doesn't match (negative case)
 *
 * No customer data in fixtures — generic IDs only per CLAUDE.md.
 *
 * @package DailyOS
 */

declare(strict_types=1);

use PHPUnit\Framework\TestCase;

require_once __DIR__ . '/../../blocks/_shared/envelope/envelope-resolver.php';
require_once __DIR__ . '/../../blocks/account-detail/render-functions.php';
require_once __DIR__ . '/../../blocks/project-detail/render-functions.php';
require_once __DIR__ . '/../../blocks/person-detail/render-functions.php';
require_once __DIR__ . '/../../blocks/meeting-detail/render-functions.php';

// WP post-context stubs. The W2 L4 auto-fill path probes `get_the_ID()`,
// `get_post_type()`, `get_post_meta()`, `get_post()` — none of which the
// PHPUnit bootstrap stubs by default. Each function reads from a test-only
// global so the test can pin the simulated post.
if ( ! function_exists( 'get_the_ID' ) ) {
	// phpcs:ignore WordPress.NamingConventions.ValidFunctionName.FunctionNameInvalid -- Mirror WP core function name.
	function get_the_ID(): int {
		return (int) ( $GLOBALS['dailyos_test_current_post_id'] ?? 0 );
	}
}
if ( ! function_exists( 'get_post_type' ) ) {
	function get_post_type( int|object|null $post = null ): string|false {
		$post_id = is_int( $post )
		? $post
		: (int) ( $GLOBALS['dailyos_test_current_post_id'] ?? 0 );
		$posts   = $GLOBALS['dailyos_test_posts'] ?? [];
		if ( ! isset( $posts[ $post_id ] ) ) {
			return false;
		}
		return (string) ( $posts[ $post_id ]['post_type'] ?? '' );
	}
}
if ( ! function_exists( 'get_post_meta' ) ) {
	function get_post_meta( int $post_id, string $key = '', bool $single = false ) {
		unset( $single );
		$meta = $GLOBALS['dailyos_test_post_meta'] ?? [];
		if ( '' === $key ) {
			return $meta[ $post_id ] ?? [];
		}
		return $meta[ $post_id ][ $key ] ?? '';
	}
}
if ( ! function_exists( 'get_post' ) ) {
	function get_post( int $post_id ): ?object {
		$posts = $GLOBALS['dailyos_test_posts'] ?? [];
		if ( ! isset( $posts[ $post_id ] ) ) {
			return null;
		}
		return (object) $posts[ $post_id ];
	}
}

/**
 * @covers dailyos_account_detail_render
 * @covers dailyos_project_detail_render
 * @covers dailyos_person_detail_render
 * @covers dailyos_meeting_detail_render
 */
final class DailyOS_EntityDetailAutoFillTest extends TestCase {

	/**
	 * Reset test globals between tests.
	 */
	protected function setUp(): void {
		parent::setUp();
		if ( function_exists( 'dailyos_test_reset_globals' ) ) {
			dailyos_test_reset_globals();
		}
		unset(
			$GLOBALS['dailyos_test_filters']['dailyos_runtime_client_for_block'],
			$GLOBALS['dailyos_test_current_post_id'],
			$GLOBALS['dailyos_test_posts'],
			$GLOBALS['dailyos_test_post_meta']
		);
	}

	// ---- account-detail -------------------------------------------------

	/**
	 * Asserts account detail uses post meta when attribute empty.
	 */
	public function test_account_detail_uses_post_meta_when_attribute_empty(): void {
		$this->seed_post( 101, 'dailyos_account', 'acct-from-slug' );
		$GLOBALS['dailyos_test_post_meta'][101] = [
			'dailyos_entity_id' => 'acct-from-meta',
		];
		$client                                 = $this->fake_runtime_client();
		$this->register_runtime_client_filter( $client );

		dailyos_account_detail_render( [], '' );

		$this->assertSame( 1, $client->calls );
		$this->assertSame( 'acct-from-meta', $client->requests[0]['payload']['entity_id'] );
	}

	/**
	 * Asserts account detail falls back to slug when meta empty.
	 */
	public function test_account_detail_falls_back_to_slug_when_meta_empty(): void {
		$this->seed_post( 102, 'dailyos_account', 'acct-from-slug' );
		$client = $this->fake_runtime_client();
		$this->register_runtime_client_filter( $client );

		dailyos_account_detail_render( [], '' );

		$this->assertSame( 1, $client->calls );
		$this->assertSame( 'acct-from-slug', $client->requests[0]['payload']['entity_id'] );
	}

	/**
	 * Asserts account detail does not auto fill on wrong post type.
	 */
	public function test_account_detail_does_not_auto_fill_on_wrong_post_type(): void {
		// post type is project, not account — auto-fill must NOT apply.
		$this->seed_post( 103, 'dailyos_project', 'proj-slug' );
		$client = $this->fake_runtime_client();
		$this->register_runtime_client_filter( $client );

		$html = dailyos_account_detail_render( [], '' );

		$this->assertStringContainsString( 'data-empty-reason="no_account_id"', $html );
		$this->assertSame( 0, $client->calls, 'producer must NOT be invoked when attribute empty + post type mismatched' );
	}

	/**
	 * Asserts account detail attribute wins over post context.
	 */
	public function test_account_detail_attribute_wins_over_post_context(): void {
		$this->seed_post( 104, 'dailyos_account', 'acct-from-slug' );
		$GLOBALS['dailyos_test_post_meta'][104] = [
			'dailyos_entity_id' => 'acct-from-meta',
		];
		$client                                 = $this->fake_runtime_client();
		$this->register_runtime_client_filter( $client );

		dailyos_account_detail_render( [ 'account_id' => 'acct-from-attr' ], '' );

		$this->assertSame( 'acct-from-attr', $client->requests[0]['payload']['entity_id'] );
	}

	// ---- project-detail -------------------------------------------------

	/**
	 * Asserts project detail uses post meta when attribute empty.
	 */
	public function test_project_detail_uses_post_meta_when_attribute_empty(): void {
		$this->seed_post( 201, 'dailyos_project', 'proj-from-slug' );
		$GLOBALS['dailyos_test_post_meta'][201] = [
			'dailyos_entity_id' => 'proj-from-meta',
		];
		$client                                 = $this->fake_runtime_client();
		$this->register_runtime_client_filter( $client );

		dailyos_project_detail_render( [], '' );

		$this->assertSame( 'proj-from-meta', $client->requests[0]['payload']['entity_id'] );
	}

	/**
	 * Asserts project detail falls back to slug when meta empty.
	 */
	public function test_project_detail_falls_back_to_slug_when_meta_empty(): void {
		$this->seed_post( 202, 'dailyos_project', 'proj-from-slug' );
		$client = $this->fake_runtime_client();
		$this->register_runtime_client_filter( $client );

		dailyos_project_detail_render( [], '' );

		$this->assertSame( 'proj-from-slug', $client->requests[0]['payload']['entity_id'] );
	}

	// ---- person-detail --------------------------------------------------

	/**
	 * Asserts person detail uses post meta when attribute empty.
	 */
	public function test_person_detail_uses_post_meta_when_attribute_empty(): void {
		$this->seed_post( 301, 'dailyos_person', 'person-from-slug' );
		$GLOBALS['dailyos_test_post_meta'][301] = [
			'dailyos_entity_id' => 'person-from-meta',
		];
		$client                                 = $this->fake_runtime_client();
		$this->register_runtime_client_filter( $client );

		dailyos_person_detail_render( [], '' );

		$this->assertSame( 'person-from-meta', $client->requests[0]['payload']['entity_id'] );
	}

	/**
	 * Asserts person detail falls back to slug when meta empty.
	 */
	public function test_person_detail_falls_back_to_slug_when_meta_empty(): void {
		$this->seed_post( 302, 'dailyos_person', 'person-from-slug' );
		$client = $this->fake_runtime_client();
		$this->register_runtime_client_filter( $client );

		dailyos_person_detail_render( [], '' );

		$this->assertSame( 'person-from-slug', $client->requests[0]['payload']['entity_id'] );
	}

	// ---- meeting-detail -------------------------------------------------

	/**
	 * Asserts meeting detail uses post meta when attribute empty.
	 */
	public function test_meeting_detail_uses_post_meta_when_attribute_empty(): void {
		$this->seed_post( 401, 'dailyos_meeting', 'meeting-from-slug' );
		$GLOBALS['dailyos_test_post_meta'][401] = [
			'dailyos_entity_id' => 'meeting-from-meta',
		];
		$client                                 = $this->fake_runtime_client();
		$this->register_runtime_client_filter( $client );

		dailyos_meeting_detail_render( [], '' );

		// Meeting outer makes 2 calls (envelope + prep status); both must
		// carry the auto-filled meeting_id.
		$this->assertGreaterThanOrEqual( 1, $client->calls );
		$this->assertSame( 'meeting-from-meta', $client->requests[0]['payload']['entity_id'] );
	}

	/**
	 * Asserts meeting detail falls back to slug when meta empty.
	 */
	public function test_meeting_detail_falls_back_to_slug_when_meta_empty(): void {
		$this->seed_post( 402, 'dailyos_meeting', 'meeting-from-slug' );
		$client = $this->fake_runtime_client();
		$this->register_runtime_client_filter( $client );

		dailyos_meeting_detail_render( [], '' );

		$this->assertGreaterThanOrEqual( 1, $client->calls );
		$this->assertSame( 'meeting-from-slug', $client->requests[0]['payload']['entity_id'] );
	}

	// ---- block.json contract: editorScript declared ---------------------

	/**
	 * Asserts w2 outer blocks declare editor script.
	 */
	public function test_w2_outer_blocks_declare_editor_script(): void {
		$outers = [
			'account-detail',
			'project-detail',
			'person-detail',
			'meeting-detail',
		];
		foreach ( $outers as $slug ) {
			$json_path = __DIR__ . '/../../blocks/' . $slug . '/block.json';
			$this->assertFileExists( $json_path );
			$json = json_decode( (string) file_get_contents( $json_path ), true );
			$this->assertSame( 3, $json['apiVersion'], $slug );
			$this->assertArrayHasKey( 'editorScript', $json, $slug . ' must declare editorScript for L4 inspector wiring' );
			$this->assertSame( 'file:./edit.js', $json['editorScript'], $slug );

			$edit_js = __DIR__ . '/../../blocks/' . $slug . '/edit.js';
			$this->assertFileExists( $edit_js, $slug . ' edit.js missing' );

			$edit_asset = __DIR__ . '/../../blocks/' . $slug . '/edit.asset.php';
			$this->assertFileExists( $edit_asset, $slug . ' edit.asset.php missing' );

			$asset = include $edit_asset;
			$this->assertIsArray( $asset, $slug . ' edit.asset.php must return array' );
			$this->assertContains( 'wp-block-editor', $asset['dependencies'], $slug );
			$this->assertContains( 'wp-components', $asset['dependencies'], $slug );
			$this->assertContains( 'wp-i18n', $asset['dependencies'], $slug );
			$this->assertContains( 'wp-element', $asset['dependencies'], $slug );
			$this->assertContains( 'wp-blocks', $asset['dependencies'], $slug );
		}
	}

	// ---- theme template contract: W2 outer block rendered --------------

	/**
	 * Asserts entity templates render w2 outer blocks.
	 */
	public function test_entity_templates_render_w2_outer_blocks(): void {
		// V2 wave templates can reference the outer block either directly
		// (`<!-- wp:dailyos/X-detail /-->`) or via the canonical filesystem
		// pattern (`<!-- wp:pattern {"slug":"dailyos/X-detail-default"} /-->`)
		// which expands to the full chapter composition at parse time. Both
		// forms route through the same outer-block renderer at render time.
		$cases = [
			'single-dailyos_account.html' => [ 'wp:dailyos/account-detail', 'wp:pattern' ],
			'single-dailyos_project.html' => [ 'wp:dailyos/project-detail', 'wp:pattern' ],
			'single-dailyos_person.html'  => [ 'wp:dailyos/person-detail', 'wp:pattern' ],
			'single-dailyos_meeting.html' => [ 'wp:dailyos/meeting-detail', 'wp:pattern' ],
		];
		foreach ( $cases as $template => $any_of_markers ) {
			$path = __DIR__ . '/../../theme/templates/' . $template;
			$this->assertFileExists( $path, $template . ' missing' );
			$contents     = (string) file_get_contents( $path );
			$found_marker = false;
			foreach ( $any_of_markers as $marker ) {
				if ( str_contains( $contents, $marker ) ) {
					$found_marker = true;
					break;
				}
			}
			$this->assertTrue(
				$found_marker,
				$template . ' must reference the outer block (direct or via wp:pattern slug)'
			);
			$this->assertStringContainsString( 'MagazinePageLayout_magazinePage', $contents, $template . ' must use the magazine shell.' );
			$this->assertStringNotContainsString( 'sidebar-account-summary', $contents, $template . ' must not use the legacy sidebar shell.' );
		}
	}

	// ---- helpers --------------------------------------------------------

	/**
	 * Seed a fake post in test globals + pin it as the current post.
	 */
	private function seed_post( int $id, string $post_type, string $slug ): void {
		$GLOBALS['dailyos_test_current_post_id'] = $id;
		$GLOBALS['dailyos_test_posts'][ $id ]    = [
			'ID'        => $id,
			'post_type' => $post_type,
			'post_name' => $slug,
		];
	}

	/**
	 * Build a fake runtime client whose invoke_ability captures the
	 * call shape and returns a present-envelope response.
	 */
	private function fake_runtime_client(): object {
		return new class() {
			public int $calls = 0;
			/**
			 * @var array<int,array<string,mixed>>
			 */
			public array $requests = [];
			public function invoke_ability( string $ability, array $payload, array $scope_set ) {
				++$this->calls;
				$this->requests[] = [
					'ability'   => $ability,
					'payload'   => $payload,
					'scope_set' => $scope_set,
				];
				// Return a minimal present envelope shape so each renderer
				// proceeds past its is_wp_error / shape checks.
				return [
					'ok'       => true,
					'envelope' => [
						'envelopeRenderId' => 'env-' . ( $payload['entity_id'] ?? 'x' ),
						'subject'          => [
							'kind'         => $payload['entity_type'] ?? 'account',
							'id'           => $payload['entity_id'] ?? '',
							'displayLabel' => 'Generic Test Subject',
						],
						'sections'         => [
							'facts' => [
								'kind'       => 'present',
								'item_count' => 1,
							],
						],
					],
				];
			}
		};
	}

	/**
	 * Register the runtime-client filter so each detail renderer picks up
	 * the fake.
	 */
	private function register_runtime_client_filter( object $client ): void {
		add_filter(
			'dailyos_runtime_client_for_block',
			static function () use ( $client ) {
				return $client;
			},
			10,
			1
		);
	}
}
