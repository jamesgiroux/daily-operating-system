<?php
/**
 * Token-mapping manifest translator and gate tests.
 *
 * @package DailyOS
 */

declare(strict_types=1);

use PHPUnit\Framework\TestCase;

/**
 * Covers token-mapping manifest emission and CI gate failure modes.
 */
final class DailyOS_TokenMappingManifestTest extends TestCase {
	/**
	 * Paths created by the test and removed in tearDown.
	 *
	 * @var array<int, string>
	 */
	private array $created_paths = [];

	/**
	 * Cleanup generated fixture files.
	 */
	protected function tearDown(): void {
		foreach ( array_reverse( $this->created_paths ) as $path ) {
			$this->remove_path( $path );
		}
		$this->created_paths = [];
		parent::tearDown();
	}

	/**
	 * The real translator emits a token manifest beside generated style.css.
	 */
	public function test_translate_tauri_emits_token_mapping_manifest_for_pill(): void {
		$this->require_command( 'node' );

		$repo_root = $this->repo_root();
		$block_dir = $repo_root . '/wp/dailyos/blocks/pill';
		if ( is_dir( $block_dir ) ) {
			$this->markTestSkipped( 'Pill block already exists; translator overwrite guard is working.' );
		}

		$this->created_paths[] = $block_dir;
		$result                = $this->run_command(
			[
				'node',
				'wp/dailyos/scripts/translate-tauri.mjs',
				'--primitive',
				'Pill',
			],
			$repo_root
		);

		$this->assertSame(
			0,
			$result['code'],
			"translator failed\nstdout:\n{$result['stdout']}\nstderr:\n{$result['stderr']}"
		);

		$manifest_path = $block_dir . '/.token-mapping.json';
		$style_path    = $block_dir . '/style.css';
		$this->assertFileExists( $manifest_path );
		$this->assertFileExists( $style_path );

		$manifest = json_decode( (string) file_get_contents( $manifest_path ), true );
		$this->assertIsArray( $manifest );
		$this->assertContains(
			[
				'source_token' => '--color-spice-turmeric',
				'target_token' => 'wp--preset--color--spice-turmeric',
			],
			$manifest
		);
		$this->assertContains(
			[
				'source_token' => '--space-xs',
				'target_token' => 'wp--preset--spacing--xs',
			],
			$manifest
		);

		$style = (string) file_get_contents( $style_path );
		$this->assertStringContainsString( 'var(--wp--preset--color--spice-turmeric)', $style );
		$this->assertStringNotContainsString( 'var(--color-', $style );
		$this->assertDoesNotMatchRegularExpression( '/#[0-9a-f]{3,8}\b|rgba?\(|hsla?\(/i', $style );
	}

	/**
	 * The gate passes when style.css, manifest, and theme.json agree.
	 */
	public function test_token_mapping_gate_passes_valid_fixture(): void {
		$style = '.wp-block-dailyos-token-fixture { '
			. 'color: var(--wp--preset--color--spice-turmeric); '
			. 'gap: var(--wp--preset--spacing--sm); '
			. '}';
		$root  = $this->create_gate_fixture(
			[
				[
					'source_token' => '--color-spice-turmeric',
					'target_token' => 'wp--preset--color--spice-turmeric',
				],
				[
					'source_token' => '--space-sm',
					'target_token' => 'wp--preset--spacing--sm',
				],
			],
			$style,
			[ 'spice-turmeric' ]
		);

		$result = $this->run_gate( $root );
		$this->assertSame(
			0,
			$result['code'],
			"gate failed\nstdout:\n{$result['stdout']}\nstderr:\n{$result['stderr']}"
		);
	}

	/**
	 * The gate fails when a manifest color target is absent from theme.json.
	 */
	public function test_token_mapping_gate_rejects_undefined_palette_target(): void {
		$style = '.wp-block-dailyos-token-fixture { '
			. 'color: var(--wp--preset--color--missing); '
			. '}';
		$root  = $this->create_gate_fixture(
			[
				[
					'source_token' => '--color-spice-turmeric',
					'target_token' => 'wp--preset--color--missing',
				],
			],
			$style,
			[ 'spice-turmeric' ]
		);

		$result = $this->run_gate( $root );
		$this->assertNotSame( 0, $result['code'] );
		$this->assertStringContainsString( 'not defined in theme.json settings.color.palette', $result['stderr'] );
	}

	/**
	 * The gate fails when style.css uses a WordPress var missing from manifest.
	 */
	public function test_token_mapping_gate_rejects_style_var_missing_from_manifest(): void {
		$style = '.wp-block-dailyos-token-fixture { '
			. 'color: var(--wp--preset--color--spice-turmeric); '
			. 'background: var(--wp--preset--color--garden-larkspur); '
			. '}';
		$root  = $this->create_gate_fixture(
			[
				[
					'source_token' => '--color-spice-turmeric',
					'target_token' => 'wp--preset--color--spice-turmeric',
				],
			],
			$style,
			[ 'spice-turmeric', 'garden-larkspur' ]
		);

		$result = $this->run_gate( $root );
		$this->assertNotSame( 0, $result['code'] );
		$this->assertStringContainsString( 'missing from .token-mapping.json', $result['stderr'] );
	}

	/**
	 * The gate fails on raw hex/rgb/hsl color literals outside the escape list.
	 */
	public function test_token_mapping_gate_rejects_raw_color_literals(): void {
		$style = '.wp-block-dailyos-token-fixture { '
			. 'color: var(--wp--preset--color--spice-turmeric); '
			. 'border-color: #ffffff; '
			. '}';
		$root  = $this->create_gate_fixture(
			[
				[
					'source_token' => '--color-spice-turmeric',
					'target_token' => 'wp--preset--color--spice-turmeric',
				],
			],
			$style,
			[ 'spice-turmeric' ]
		);

		$result = $this->run_gate( $root );
		$this->assertNotSame( 0, $result['code'] );
		$this->assertStringContainsString( 'raw color literal', $result['stderr'] );
	}

	/**
	 * Creates a temporary repo-shaped fixture for the bash gate.
	 *
	 * @param array<int, array{source_token: string, target_token: string}> $manifest Manifest entries.
	 * @param string                                                        $style    style.css contents.
	 * @param array<int, string>                                            $palette  Theme color slugs.
	 */
	private function create_gate_fixture( array $manifest, string $style, array $palette ): string {
		$this->require_command( 'bash' );
		$this->require_command( 'jq' );
		$root                  = sys_get_temp_dir() . '/dailyos-token-map-' . bin2hex( random_bytes( 6 ) );
		$this->created_paths[] = $root;

		$block_dir = $root . '/wp/dailyos/blocks/token-fixture';
		$theme_dir = $root . '/wp/dailyos/theme';
		mkdir( $block_dir, 0777, true );
		mkdir( $theme_dir, 0777, true );

		file_put_contents( $block_dir . '/style.css', $style . "\n" );
		file_put_contents(
			$block_dir . '/.token-mapping.json',
			json_encode( $manifest, JSON_PRETTY_PRINT | JSON_UNESCAPED_SLASHES ) . "\n"
		);
		file_put_contents(
			$theme_dir . '/theme.json',
			json_encode(
				[
					'version'  => 3,
					'settings' => [
						'color' => [
							'palette' => array_map(
								static fn ( string $slug ): array => [
									'slug'  => $slug,
									'name'  => ucwords( str_replace( '-', ' ', $slug ) ),
									'color' => '#000000',
								],
								$palette
							),
						],
					],
				],
				JSON_PRETTY_PRINT | JSON_UNESCAPED_SLASHES
			) . "\n"
		);

		return $root;
	}

	/**
	 * Runs the token-mapping gate against a fixture repo root.
	 *
	 * @return array{code: int, stdout: string, stderr: string}
	 */
	private function run_gate( string $fixture_root ): array {
		return $this->run_command(
			[
				'bash',
				'wp/dailyos/tests/token-mapping-manifest-gate.sh',
			],
			$this->repo_root(),
			[
				'DAILYOS_TOKEN_MAPPING_ROOT' => $fixture_root,
			]
		);
	}

	/**
	 * Returns the repository root.
	 */
	private function repo_root(): string {
		return dirname( __DIR__, 4 );
	}

	/**
	 * Skips the test when a required command is unavailable.
	 */
	private function require_command( string $command ): void {
		$result = $this->run_command(
			[ 'bash', '-lc', 'command -v ' . escapeshellarg( $command ) ],
			$this->repo_root()
		);
		if ( 0 !== $result['code'] ) {
			$this->markTestSkipped( "Required command '{$command}' is unavailable." );
		}
	}

	/**
	 * Runs a command and captures stdout/stderr.
	 *
	 * @param array<int, string>    $command Command argv.
	 * @param array<string, string> $env     Extra environment.
	 * @return array{code: int, stdout: string, stderr: string}
	 */
	private function run_command( array $command, string $cwd, array $env = [] ): array {
		$descriptor_spec = [
			1 => [ 'pipe', 'w' ],
			2 => [ 'pipe', 'w' ],
		];
		$base_env        = $_ENV;
		foreach ( [ 'PATH', 'HOME', 'TMPDIR' ] as $key ) {
			if ( isset( $_SERVER[ $key ] ) && ! isset( $base_env[ $key ] ) ) {
				$base_env[ $key ] = (string) $_SERVER[ $key ];
			}
		}

		$process = proc_open(
			implode( ' ', array_map( 'escapeshellarg', $command ) ),
			$descriptor_spec,
			$pipes,
			$cwd,
			array_merge( $base_env, $env )
		);
		if ( ! is_resource( $process ) ) {
			$this->fail( 'Failed to start process: ' . implode( ' ', $command ) );
		}

		$stdout = stream_get_contents( $pipes[1] );
		$stderr = stream_get_contents( $pipes[2] );
		fclose( $pipes[1] );
		fclose( $pipes[2] );

		return [
			'code'   => proc_close( $process ),
			'stdout' => false === $stdout ? '' : $stdout,
			'stderr' => false === $stderr ? '' : $stderr,
		];
	}

	/**
	 * Recursively removes a test-created path.
	 */
	private function remove_path( string $path ): void {
		if ( ! file_exists( $path ) ) {
			return;
		}
		if ( is_file( $path ) || is_link( $path ) ) {
			unlink( $path );
			return;
		}
		$entries = scandir( $path );
		if ( false === $entries ) {
			return;
		}
		foreach ( $entries as $entry ) {
			if ( '.' === $entry || '..' === $entry ) {
				continue;
			}
			$this->remove_path( $path . DIRECTORY_SEPARATOR . $entry );
		}
		rmdir( $path );
	}
}
