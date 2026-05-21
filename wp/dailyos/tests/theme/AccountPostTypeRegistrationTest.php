<?php
/**
 * W3 magazine theme — DailyOS CPT registration tests.
 *
 * Covers all five entity CPTs registered by `register_post_types()`:
 * dailyos_account, dailyos_project, dailyos_person, dailyos_meeting,
 * dailyos_briefing.
 *
 * @package DailyOS
 */

declare(strict_types=1);

use DailyOS\DailyOS_Plugin;
use PHPUnit\Framework\TestCase;

if ( ! function_exists( 'register_post_type' ) ) {
	/**
	 * Minimal stub mirroring core's `register_post_type()` so unit tests
	 * can observe the registration without a live WordPress.
	 *
	 * @param string               $post_type Post type slug.
	 * @param array<string, mixed> $args      Registration arguments.
	 */
	function register_post_type( string $post_type, array $args = [] ): object {
		$registered = (object) array_merge(
			[
				'name'         => $post_type,
				'public'       => false,
				'has_archive'  => false,
				'rewrite'      => false,
				'show_in_rest' => false,
				'rest_base'    => '',
				'supports'     => [],
			],
			$args
		);

		$GLOBALS['dailyos_test_registered_post_types'][ $post_type ] = $registered;

		return $registered;
	}
}

if ( ! function_exists( 'get_post_types' ) ) {
	/**
	 * Minimal stub mirroring core's `get_post_types()`.
	 *
	 * @param array<string, mixed> $args     Lookup filter (only `name` is honored).
	 * @param string               $output   Output format (ignored).
	 * @param string               $operator Logical operator (ignored).
	 * @return array<string, mixed>
	 */
	function get_post_types( array $args = [], string $output = 'names', string $operator = 'and' ): array {
		unset( $output, $operator );

		$registry = $GLOBALS['dailyos_test_registered_post_types'] ?? [];

		if ( isset( $args['name'] ) ) {
			$name = (string) $args['name'];
			return isset( $registry[ $name ] ) ? [ $name => $registry[ $name ] ] : [];
		}

		return $registry;
	}
}

/**
 * Asserts that every DailyOS entity CPT registers with the shape templates
 * and rewrite rules depend on. Each CPT gets the same four checks: registered,
 * public, archive + rewrite slug, REST-exposed at the expected base.
 *
 * Spec: L0 Packet E V1.4 §5.0 + §8.2; v1.4.4 W2 extends to project/person/
 * meeting; v1.4.4 W3 extends to briefing.
 */
final class DailyOS_AccountPostTypeRegistrationTest extends TestCase {
	/**
	 * Reset registries before every test.
	 */
	protected function setUp(): void {
		parent::setUp();

		dailyos_test_reset_globals();
		$GLOBALS['dailyos_test_registered_post_types'] = [];
	}

	/**
	 * @return array<string, array{0: string, 1: string}>
	 */
	public static function entity_cpt_provider(): array {
		return [
			'account'  => [ 'dailyos_account', 'accounts' ],
			'project'  => [ 'dailyos_project', 'projects' ],
			'person'   => [ 'dailyos_person', 'people' ],
			'meeting'  => [ 'dailyos_meeting', 'meetings' ],
			'briefing' => [ 'dailyos_briefing', 'briefings' ],
		];
	}

	/**
	 * Calling `register_post_types()` records every entity CPT.
	 *
	 * @dataProvider entity_cpt_provider
	 */
	public function test_register_post_types_records_cpt( string $cpt, string $rest_base ): void {
		unset( $rest_base );
		DailyOS_Plugin::instance()->register_post_types();

		$post_types = get_post_types( [ 'name' => $cpt ] );

		$this->assertArrayHasKey( $cpt, $post_types );
	}

	/**
	 * Every entity CPT is publicly queryable.
	 *
	 * @dataProvider entity_cpt_provider
	 */
	public function test_cpt_is_public( string $cpt, string $rest_base ): void {
		unset( $rest_base );
		DailyOS_Plugin::instance()->register_post_types();

		$registered = $GLOBALS['dailyos_test_registered_post_types'][ $cpt ] ?? null;

		$this->assertNotNull( $registered, "CPT {$cpt} not registered" );
		$this->assertTrue( (bool) $registered->public, "CPT {$cpt} must be public" );
	}

	/**
	 * Every entity CPT exposes a public archive and declares a rewrite slug.
	 * The account slug is `accounts`; project/person/meeting nest under
	 * `entities/`; briefing uses `briefings`.
	 *
	 * @dataProvider entity_cpt_provider
	 */
	public function test_cpt_has_archive_and_rewrite_slug( string $cpt, string $rest_base ): void {
		unset( $rest_base );
		DailyOS_Plugin::instance()->register_post_types();

		$registered = $GLOBALS['dailyos_test_registered_post_types'][ $cpt ] ?? null;

		$this->assertNotNull( $registered, "CPT {$cpt} not registered" );
		$this->assertTrue( (bool) $registered->has_archive, "CPT {$cpt} must declare has_archive" );
		$this->assertIsArray( $registered->rewrite, "CPT {$cpt} must declare rewrite array" );
		$this->assertArrayHasKey( 'slug', $registered->rewrite, "CPT {$cpt} rewrite missing slug" );
	}

	/**
	 * Every entity CPT is exposed through the REST API at its expected base.
	 *
	 * @dataProvider entity_cpt_provider
	 */
	public function test_cpt_is_exposed_in_rest_with_expected_base( string $cpt, string $rest_base ): void {
		DailyOS_Plugin::instance()->register_post_types();

		$registered = $GLOBALS['dailyos_test_registered_post_types'][ $cpt ] ?? null;

		$this->assertNotNull( $registered, "CPT {$cpt} not registered" );
		$this->assertTrue( (bool) $registered->show_in_rest, "CPT {$cpt} must show_in_rest" );
		$this->assertSame( $rest_base, $registered->rest_base, "CPT {$cpt} rest_base mismatch" );
	}
}
