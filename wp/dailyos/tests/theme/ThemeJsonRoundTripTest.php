<?php
/**
 * Theme JSON generator round-trip test.
 *
 * Covers v1.4.3 L0 Packet E §8.1: the committed `wp/dailyos/theme/theme.json`
 * must equal `generate-theme-json.mjs` output byte-for-byte. Drift between the
 * canonical CSS tokens and the magazine theme would silently break Site Editor
 * presets, so this test gates regeneration discipline at PR time.
 *
 * Also asserts the V1.4 shape extensions (§5.1, §5.1.b):
 *   - `customTemplates`: present, empty array (W3 has no assignable templates).
 *   - `templateParts`: 3 entries (header / footer / sidebar-account-summary).
 *   - `styles.blocks`: present (empty in W3; W4 adds per-block overrides here).
 *
 * @package DailyOS
 */

declare(strict_types=1);

use PHPUnit\Framework\TestCase;

/**
 * Theme.json regeneration discipline + V1.4 shape gate.
 */
final class DailyOS_ThemeJsonRoundTripTest extends TestCase {
	/**
	 * Round-trip: regeneration produces the exact committed theme.json.
	 *
	 * Uses the generator's own `--check` mode, which serializes a fresh
	 * theme.json from canonical tokens and diffs it against the committed
	 * file. Non-zero exit means drift.
	 */
	public function test_generator_check_mode_matches_committed_theme_json(): void {
		$this->require_command( 'node' );

		$repo_root = $this->repo_root();
		$result    = $this->run_command(
			[ 'node', 'wp/dailyos/scripts/generate-theme-json.mjs', '--check' ],
			$repo_root
		);

		$this->assertSame(
			0,
			$result['code'],
			"generate-theme-json --check failed; committed wp/dailyos/theme/theme.json is out of sync with src/styles/design-tokens.css.\n"
			. "Run: pnpm dailyos:generate-theme-json\n"
			. "stdout:\n{$result['stdout']}\nstderr:\n{$result['stderr']}"
		);
	}

	/**
	 * The committed theme.json carries the V1.4 magazine extensions.
	 *
	 * Belt-and-braces against the generator drifting to a closed shape and
	 * the `--check` test still passing (because closed shape would round-trip).
	 */
	public function test_committed_theme_json_has_v14_extensions(): void {
		$theme_json_path = $this->repo_root() . '/wp/dailyos/theme/theme.json';
		$this->assertFileExists( $theme_json_path );

		$decoded = json_decode( (string) file_get_contents( $theme_json_path ), true );
		$this->assertIsArray( $decoded );

		// customTemplates: present and empty (§5.1.b).
		$this->assertArrayHasKey( 'customTemplates', $decoded );
		$this->assertIsArray( $decoded['customTemplates'] );
		$this->assertSame( [], $decoded['customTemplates'] );

		// templateParts: 3 entries matching W3 parts/.
		$this->assertArrayHasKey( 'templateParts', $decoded );
		$this->assertIsArray( $decoded['templateParts'] );
		$this->assertCount( 3, $decoded['templateParts'] );

		$names = array_map(
			static fn ( array $part ): string => (string) ( $part['name'] ?? '' ),
			$decoded['templateParts']
		);
		$this->assertSame(
			[ 'header', 'footer', 'sidebar-account-summary' ],
			$names,
			'templateParts must declare header, footer, sidebar-account-summary in that order.'
		);

		// styles.blocks: present (empty in W3; W4 will populate).
		$this->assertArrayHasKey( 'styles', $decoded );
		$this->assertIsArray( $decoded['styles'] );
		$this->assertArrayHasKey( 'blocks', $decoded['styles'] );
		$this->assertIsArray( $decoded['styles']['blocks'] );
	}

	/**
	 * Returns the repository root (four levels up from this file).
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
}
