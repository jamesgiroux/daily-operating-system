<?php
/**
 * Generic block-kit integration harness.
 *
 * @package DailyOS
 */

declare(strict_types=1);

// phpcs:disable

if ( ! isset( $GLOBALS['dailyos_test_block_types'] ) ) {
	$GLOBALS['dailyos_test_block_types'] = [];
}

if ( ! class_exists( 'WP_Block', false ) ) {
	class WP_Block {
		/** @var array<string, mixed> */
		public array $parsed_block;
		/** @var array<string, mixed> */
		public array $attributes;
		public string $name;

		/**
		 * @param array<string, mixed> $parsed_block Parsed block.
		 */
		public function __construct( array $parsed_block ) {
			$this->parsed_block = $parsed_block;
			$this->attributes   = isset( $parsed_block['attrs'] ) && is_array( $parsed_block['attrs'] )
				? $parsed_block['attrs']
				: [];
			$this->name         = isset( $parsed_block['blockName'] ) ? (string) $parsed_block['blockName'] : '';
		}

		/**
		 * @param array<string, mixed> $options Render options.
		 */
		public function render( array $options = [] ): string {
			unset( $options );
			return render_block( $this->parsed_block );
		}
	}
}

if ( ! function_exists( 'register_block_type' ) ) {
	/**
	 * @param string               $block_type Block name.
	 * @param array<string, mixed> $args       Registration settings.
	 */
	function register_block_type( string $block_type, array $args = [] ): object|false {
		if ( '' === $block_type ) {
			return false;
		}

		$registered = (object) [
			'name'            => $block_type,
			'attributes'      => isset( $args['attributes'] ) && is_array( $args['attributes'] ) ? $args['attributes'] : [],
			'render_callback' => $args['render_callback'] ?? null,
			'settings'        => $args,
		];

		$GLOBALS['dailyos_test_block_types'][ $block_type ] = $registered;
		return $registered;
	}
}

if ( ! function_exists( 'register_block_type_from_metadata' ) ) {
	/**
	 * @param string               $file_or_folder Metadata file or block directory.
	 * @param array<string, mixed> $args           Registration overrides.
	 */
	function register_block_type_from_metadata( string $file_or_folder, array $args = [] ): object|false {
		$block_dir     = is_dir( $file_or_folder ) ? $file_or_folder : dirname( $file_or_folder );
		$metadata_path = $block_dir . '/block.json';
		$metadata      = dailyos_starter_kit_read_json_file( $metadata_path );
		$block_name    = isset( $metadata['name'] ) ? (string) $metadata['name'] : '';
		$settings      = $args;

		if ( isset( $metadata['attributes'] ) && is_array( $metadata['attributes'] ) ) {
			$settings['attributes'] = $metadata['attributes'];
		}

		if ( isset( $metadata['render'] ) && is_string( $metadata['render'] ) ) {
			$render_file                 = dailyos_starter_kit_resolve_metadata_file( $block_dir, $metadata['render'] );
			$settings['render_callback'] = static function ( array $attributes = [], string $content = '', ?WP_Block $block = null ) use ( $render_file ): string {
				return dailyos_starter_kit_include_render_file( $render_file, $attributes, $content, $block );
			};
		}

		return register_block_type( $block_name, $settings );
	}
}

if ( ! function_exists( 'render_block' ) ) {
	/**
	 * @param array<string, mixed> $parsed_block Parsed block.
	 */
	function render_block( array $parsed_block ): string {
		$block_name = isset( $parsed_block['blockName'] ) ? (string) $parsed_block['blockName'] : '';
		$registry   = $GLOBALS['dailyos_test_block_types'] ?? [];

		if ( ! isset( $registry[ $block_name ] ) ) {
			dailyos_starter_kit_fail( 'render_block.blockName', 'registered block', $block_name, dailyos_starter_kit_nearest_block_name( $block_name ) );
		}

		$block_type = $registry[ $block_name ];
		$attributes = isset( $parsed_block['attrs'] ) && is_array( $parsed_block['attrs'] ) ? $parsed_block['attrs'] : [];
		$content    = isset( $parsed_block['innerHTML'] ) ? (string) $parsed_block['innerHTML'] : '';
		$callback   = $block_type->render_callback ?? null;

		if ( is_callable( $callback ) ) {
			return (string) call_user_func( $callback, $attributes, $content, new WP_Block( $parsed_block ) );
		}

		return $content;
	}
}

if ( ! function_exists( 'get_block_wrapper_attributes' ) ) {
	/**
	 * @param array<string, mixed> $extra_attributes Extra wrapper attributes.
	 */
	function get_block_wrapper_attributes( array $extra_attributes = [] ): string {
		$attributes = [];
		foreach ( $extra_attributes as $name => $value ) {
			if ( null === $value || false === $value ) {
				continue;
			}
			$attributes[ (string) $name ] = true === $value ? (string) $name : (string) $value;
		}

		$out = [];
		foreach ( $attributes as $name => $value ) {
			$out[] = dailyos_starter_kit_escape_attr( $name ) . '="' . dailyos_starter_kit_escape_attr( $value ) . '"';
		}
		return implode( ' ', $out );
	}
}

/**
 * @param array<int, string> $argv CLI args.
 */
function dailyos_starter_kit_main( array $argv ): int {
	try {
		if ( 4 !== count( $argv ) ) {
			dailyos_starter_kit_fail( 'argv', 'projection.json ability_name expected.json', 'wrong argument count' );
		}

		require_once __DIR__ . '/../bootstrap.php';
		if ( function_exists( 'dailyos_test_reset_globals' ) ) {
			dailyos_test_reset_globals();
		}
		$GLOBALS['dailyos_test_block_types'] = [];

		$projection_path = dailyos_starter_kit_assert_fixture_path( $argv[1], 'projection.json' );
		$expected_path   = dailyos_starter_kit_assert_fixture_path( $argv[3], 'expected.json' );
		if ( dirname( $projection_path ) !== dirname( $expected_path ) || 'expected.json' !== basename( $expected_path ) ) {
			dailyos_starter_kit_fail( 'expected_json_path', 'sibling expected.json', $argv[3] );
		}

		$projection = dailyos_starter_kit_read_json_file( $projection_path );
		$expected   = dailyos_starter_kit_read_json_file( $expected_path );
		$block      = dailyos_starter_kit_resolve_block( $argv[2] );

		register_block_type_from_metadata( $block['dir'] );
		dailyos_starter_kit_register_runtime_client( $projection );
		$html = dailyos_starter_kit_render_registered_block( $block['name'], $projection );

		dailyos_starter_kit_assert_diagnostics( $projection, $expected );
		dailyos_starter_kit_assert_wrapper( $html, $expected );
		dailyos_starter_kit_assert_renderer_branches( $html, $expected );
		dailyos_starter_kit_assert_bindings( $projection, $expected );

		echo $html;
		return 0;
	} catch ( Throwable $error ) {
		fwrite( STDERR, "block-kit integration failed\n" . $error->getMessage() . "\n" );
		return 1;
	}
}

/**
 * @return array<string, mixed>
 */
function dailyos_starter_kit_read_json_file( string $path ): array {
	$contents = file_get_contents( $path );
	if ( false === $contents ) {
		dailyos_starter_kit_fail( 'json_file', 'readable JSON file', $path );
	}
	$decoded = json_decode( $contents, true );
	if ( ! is_array( $decoded ) ) {
		dailyos_starter_kit_fail( 'json_file', 'JSON object', $path );
	}
	return $decoded;
}

function dailyos_starter_kit_assert_fixture_path( string $path, string $label ): string {
	$root = realpath( __DIR__ . '/../fixtures/blocks' );
	$real = realpath( $path );
	if ( false === $root || false === $real ) {
		dailyos_starter_kit_fail( $label, 'existing path under tests/fixtures/blocks', $path );
	}
	$prefix = rtrim( $root, DIRECTORY_SEPARATOR ) . DIRECTORY_SEPARATOR;
	if ( ! str_starts_with( $real, $prefix ) ) {
		dailyos_starter_kit_fail( $label, 'path under tests/fixtures/blocks', $path );
	}
	return $real;
}

/**
 * @return array{name: string, dir: string}
 */
function dailyos_starter_kit_resolve_block( string $ability_name ): array {
	$allowlist = dailyos_starter_kit_block_allowlist();
	$key       = strtolower( trim( $ability_name ) );
	if ( isset( $allowlist[ $key ] ) ) {
		return $allowlist[ $key ];
	}
	dailyos_starter_kit_fail( 'ability_name', 'allowlisted BlockType/block.json name', $ability_name, dailyos_starter_kit_nearest( $key, array_keys( $allowlist ) ) );
}

/**
 * @return array<string, array{name: string, dir: string}>
 */
function dailyos_starter_kit_block_allowlist(): array {
	$blocks_dir = dirname( __DIR__, 2 ) . '/blocks';
	$allowlist  = [];
	foreach ( glob( $blocks_dir . '/*/block.json' ) ?: [] as $block_json ) {
		$dir      = dirname( $block_json );
		$metadata = dailyos_starter_kit_read_json_file( $block_json );
		$name     = isset( $metadata['name'] ) ? (string) $metadata['name'] : '';
		$slug     = basename( $dir );
		foreach ( [ $name, $slug, str_replace( '-', '_', $slug ) ] as $alias ) {
			if ( '' !== $alias ) {
				$allowlist[ strtolower( $alias ) ] = [
					'name' => $name,
					'dir'  => $dir,
				];
			}
		}
	}
	return $allowlist;
}

function dailyos_starter_kit_resolve_metadata_file( string $block_dir, string $metadata_value ): string {
	if ( ! str_starts_with( $metadata_value, 'file:' ) ) {
		dailyos_starter_kit_fail( 'block.json.render', 'file: render callback', $metadata_value );
	}
	$relative = substr( $metadata_value, 5 );
	$path     = realpath( $block_dir . '/' . $relative );
	$root     = realpath( $block_dir );
	if ( false === $path || false === $root || ! str_starts_with( $path, rtrim( $root, DIRECTORY_SEPARATOR ) . DIRECTORY_SEPARATOR ) ) {
		dailyos_starter_kit_fail( 'block.json.render', 'render file under allowlisted block', $metadata_value );
	}
	return $path;
}

/**
 * @param array<string, mixed> $attributes Block attributes.
 */
function dailyos_starter_kit_include_render_file( string $render_file, array $attributes, string $content = '', ?WP_Block $block = null ): string {
	unset( $content, $block );
	$result = require $render_file;
	return is_string( $result ) ? $result : '';
}

/**
 * @param array<string, mixed> $projection Projection payload.
 */
function dailyos_starter_kit_register_runtime_client( array $projection ): void {
	$client = new class( $projection ) {
		/** @var array<string, mixed> */
		private array $projection;

		/** @param array<string, mixed> $projection Projection payload. */
		public function __construct( array $projection ) {
			$this->projection = $projection;
		}

		/** @return array<string, mixed> */
		public function project_composition_for_surface( string $composition_id, int $composition_version, ?string $cache_hint_token = null ): array {
			unset( $composition_id, $composition_version, $cache_hint_token );
			return [
				'projection'       => $this->projection,
				'cache_hint_token' => 'fixture-cache-token',
			];
		}
	};

	add_filter(
		'dailyos_runtime_client_for_block',
		static function () use ( $client ): object {
			return $client;
		},
		10,
		0
	);
}

/**
 * @param array<string, mixed> $projection Projection payload.
 */
function dailyos_starter_kit_render_registered_block( string $block_name, array $projection ): string {
	$attrs = [
		'composition_id'      => isset( $projection['composition_id'] ) ? (string) $projection['composition_id'] : '',
		'composition_version' => isset( $projection['composition_version'] ) ? (int) $projection['composition_version'] : 0,
	];
	if ( isset( $projection['blocks'][0]['block_id'] ) ) {
		$attrs['block_id'] = (string) $projection['blocks'][0]['block_id'];
	}

	$block = new WP_Block(
		[
			'blockName'    => $block_name,
			'attrs'        => $attrs,
			'innerHTML'    => '',
			'innerContent' => [],
			'innerBlocks'  => [],
		]
	);
	return $block->render();
}

/**
 * @param array<string, mixed> $projection Projection payload.
 * @param array<string, mixed> $expected   Expected contract.
 */
function dailyos_starter_kit_assert_diagnostics( array $projection, array $expected ): void {
	$declared = array_map( 'strval', array_column( $expected['diagnostics'] ?? [], 'reason' ) );
	$actual   = array_map( 'strval', array_column( $projection['diagnostics'] ?? [], 'reason' ) );
	if ( $declared !== $actual ) {
		dailyos_starter_kit_fail( 'projection.diagnostics[*].reason', json_encode( $declared ), json_encode( $actual ) );
	}
}

/**
 * @param array<string, mixed> $expected Expected contract.
 */
function dailyos_starter_kit_assert_wrapper( string $html, array $expected ): void {
	$wrapper = isset( $expected['wrapper'] ) && is_array( $expected['wrapper'] ) ? $expected['wrapper'] : [];
	$tag     = isset( $wrapper['tag'] ) ? strtolower( (string) $wrapper['tag'] ) : '';
	$class   = isset( $wrapper['class'] ) ? (string) $wrapper['class'] : '';
	$node    = dailyos_starter_kit_find_wrapper_node( $html, $tag, $class );
	if ( null === $node ) {
		dailyos_starter_kit_fail( 'wrapper', $tag . '.' . $class, 'not found' );
	}

	foreach ( $wrapper['data_attrs'] ?? [] as $pair ) {
		if ( ! is_array( $pair ) || 2 > count( $pair ) ) {
			dailyos_starter_kit_fail( 'wrapper.data_attrs', '[name, value]', json_encode( $pair ) );
		}
		$name  = (string) $pair[0];
		$value = (string) $pair[1];
		if ( $node->getAttribute( $name ) !== $value ) {
			dailyos_starter_kit_fail( 'wrapper.' . $name, $value, $node->getAttribute( $name ) );
		}
	}
}

function dailyos_starter_kit_find_wrapper_node( string $html, string $tag, string $class ): ?DOMElement {
	if ( '' === $tag || '' === $class ) {
		return null;
	}
	$document = new DOMDocument();
	$previous = libxml_use_internal_errors( true );
	$document->loadHTML( '<!doctype html><html><body>' . $html . '</body></html>' );
	libxml_clear_errors();
	libxml_use_internal_errors( $previous );
	foreach ( $document->getElementsByTagName( $tag ) as $node ) {
		if ( dailyos_starter_kit_class_list_contains( $node->getAttribute( 'class' ), $class ) ) {
			return $node;
		}
	}
	return null;
}

function dailyos_starter_kit_class_list_contains( string $actual, string $expected ): bool {
	$actual_classes   = preg_split( '/\s+/', trim( $actual ) ) ?: [];
	$expected_classes = preg_split( '/\s+/', trim( $expected ) ) ?: [];
	foreach ( $expected_classes as $class ) {
		if ( '' !== $class && ! in_array( $class, $actual_classes, true ) ) {
			return false;
		}
	}
	return true;
}

/**
 * @param array<string, mixed> $expected Expected contract.
 */
function dailyos_starter_kit_assert_renderer_branches( string $html, array $expected ): void {
	foreach ( $expected['renderer_branches'] ?? [] as $branch ) {
		$label   = isset( $branch['branch_label'] ) ? (string) $branch['branch_label'] : 'unknown';
		$pattern = isset( $branch['expected_html_pattern'] ) ? (string) $branch['expected_html_pattern'] : '';
		if ( '' === $pattern || ! str_contains( $html, $pattern ) ) {
			dailyos_starter_kit_fail( 'renderer_branch.' . $label, $pattern, 'missing' );
		}
	}
}

/**
 * @param array<string, mixed> $projection Projection payload.
 * @param array<string, mixed> $expected   Expected contract.
 */
function dailyos_starter_kit_assert_bindings( array $projection, array $expected ): void {
	foreach ( $expected['bindings'] ?? [] as $binding ) {
		$pointer  = isset( $binding['pointer'] ) ? (string) $binding['pointer'] : '';
		$required = (bool) ( $binding['required'] ?? false );
		$declared = isset( $binding['value_kind'] ) ? (string) $binding['value_kind'] : '';
		[ $exists, $value ] = dailyos_starter_kit_json_pointer( $projection, $pointer );
		if ( ! $exists ) {
			if ( $required ) {
				dailyos_starter_kit_fail( 'binding.' . $pointer, $declared, 'missing', dailyos_starter_kit_nearest( $pointer, dailyos_starter_kit_collect_json_pointers( $projection ) ) );
			}
			continue;
		}
		$actual = dailyos_starter_kit_value_kind( $value );
		if ( $declared !== $actual ) {
			dailyos_starter_kit_fail( 'binding.' . $pointer, $declared, $actual, dailyos_starter_kit_nearest( $pointer, dailyos_starter_kit_collect_json_pointers( $projection ) ) );
		}
	}
}

/**
 * @param array<string, mixed> $root Root object.
 * @return array{0: bool, 1: mixed}
 */
function dailyos_starter_kit_json_pointer( array $root, string $pointer ): array {
	if ( '' === $pointer || '/' === $pointer ) {
		return [ true, $root ];
	}
	$value = $root;
	foreach ( explode( '/', ltrim( $pointer, '/' ) ) as $segment ) {
		$key = str_replace( [ '~1', '~0' ], [ '/', '~' ], $segment );
		if ( ! is_array( $value ) || ! array_key_exists( $key, $value ) ) {
			return [ false, null ];
		}
		$value = $value[ $key ];
	}
	return [ true, $value ];
}

function dailyos_starter_kit_value_kind( mixed $value ): string {
	if ( is_string( $value ) ) {
		return 'string';
	}
	if ( is_int( $value ) || is_float( $value ) ) {
		return 'number';
	}
	if ( is_bool( $value ) ) {
		return 'bool';
	}
	if ( null === $value ) {
		return 'null';
	}
	if ( is_array( $value ) ) {
		return array_is_list( $value ) ? 'array' : 'object';
	}
	return gettype( $value );
}

/**
 * @param array<string, mixed>|array<int, mixed> $value JSON value.
 * @return array<int, string>
 */
function dailyos_starter_kit_collect_json_pointers( array $value, string $base = '' ): array {
	$pointers = [ '' === $base ? '/' : $base ];
	foreach ( $value as $key => $child ) {
		$path = $base . '/' . str_replace( [ '~', '/' ], [ '~0', '~1' ], (string) $key );
		if ( is_array( $child ) ) {
			$pointers = array_merge( $pointers, dailyos_starter_kit_collect_json_pointers( $child, $path ) );
		} else {
			$pointers[] = $path;
		}
	}
	return $pointers;
}

function dailyos_starter_kit_nearest_block_name( string $value ): string {
	return dailyos_starter_kit_nearest( $value, array_keys( $GLOBALS['dailyos_test_block_types'] ?? [] ) );
}

/**
 * @param array<int, string> $candidates Candidates.
 */
function dailyos_starter_kit_nearest( string $value, array $candidates ): string {
	$nearest  = '';
	$distance = PHP_INT_MAX;
	foreach ( $candidates as $candidate ) {
		$current = levenshtein( $value, $candidate );
		if ( $current < $distance ) {
			$distance = $current;
			$nearest  = $candidate;
		}
	}
	return '' === $nearest ? 'n/a' : $nearest;
}

function dailyos_starter_kit_escape_attr( string $value ): string {
	return function_exists( 'esc_attr' ) ? esc_attr( $value ) : htmlspecialchars( $value, ENT_QUOTES | ENT_SUBSTITUTE, 'UTF-8' );
}

function dailyos_starter_kit_fail( string $location, ?string $declared, ?string $actual, string $did_you_mean = 'n/a' ): never {
	throw new RuntimeException(
		'DOS-670 contract mismatch' . "\n"
		. 'location: ' . $location . "\n"
		. 'declared: ' . ( $declared ?? 'n/a' ) . "\n"
		. 'actual: ' . ( $actual ?? 'n/a' ) . "\n"
		. 'did_you_mean: ' . $did_you_mean
	);
}

if ( PHP_SAPI === 'cli' && realpath( (string) ( $_SERVER['SCRIPT_FILENAME'] ?? '' ) ) === __FILE__ ) {
	exit( dailyos_starter_kit_main( $argv ?? [] ) );
}

// phpcs:enable
