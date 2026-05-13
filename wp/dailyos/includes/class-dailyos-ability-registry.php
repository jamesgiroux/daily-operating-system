<?php
/**
 * DailyOS ability inventory reader and registrar.
 *
 * @package DailyOS
 */

declare(strict_types=1);

namespace DailyOS;

/**
 * Registers inventory-backed abilities without hardcoded ability names.
 */
final class DailyOS_Ability_Registry {
	public const INVENTORY_SCHEMA_VERSION = '1.0';

	/**
	 * Absolute path to the ability inventory.
	 *
	 * @var string
	 */
	private string $inventory_path;

	/**
	 * Constructor.
	 *
	 * @param string|null $inventory_path Absolute path to the ability inventory.
	 */
	public function __construct( ?string $inventory_path = null ) {
		$this->inventory_path = $inventory_path ?? DAILYOS_PLUGIN_DIR . 'tools/dailyos-abilities.json';
	}

	/**
	 * Load the JSON inventory. Missing or empty inventory is not an error.
	 *
	 * @return array<string, mixed>
	 */
	public function load_inventory(): array {
		if ( ! is_readable( $this->inventory_path ) ) {
			return $this->empty_inventory();
		}

		// phpcs:ignore WordPress.WP.AlternativeFunctions.file_get_contents_file_get_contents -- Local plugin inventory file.
		$contents = file_get_contents( $this->inventory_path );

		if ( false === $contents || '' === trim( $contents ) ) {
			return $this->empty_inventory();
		}

		try {
			$inventory = json_decode( $contents, true, 512, JSON_THROW_ON_ERROR );
		} catch ( \JsonException $exception ) {
			return $this->empty_inventory();
		}

		if ( ! is_array( $inventory ) || ! isset( $inventory['abilities'] ) || ! is_array( $inventory['abilities'] ) ) {
			return $this->empty_inventory();
		}

		return $inventory;
	}

	/**
	 * Register every inventory entry with the WP Abilities API when available.
	 */
	public function register_all(): int {
		$inventory = $this->load_inventory();
		$abilities = $inventory['abilities'] ?? [];

		if ( empty( $abilities ) || ! function_exists( 'wp_register_ability' ) ) {
			return 0;
		}

		$registered = 0;

		foreach ( $abilities as $ability ) {
			if ( ! is_array( $ability ) || empty( $ability['name'] ) ) {
				continue;
			}

			$result = wp_register_ability(
				'dailyos/' . $this->normalize_ability_name( (string) $ability['name'] ),
				$this->build_registration_args( $ability )
			);

			if ( ! function_exists( 'is_wp_error' ) || ! is_wp_error( $result ) ) {
				++$registered;
			}
		}

		return $registered;
	}

	/**
	 * Return the inventory path.
	 */
	public function get_inventory_path(): string {
		return $this->inventory_path;
	}

	/**
	 * Normalize an inventory name into a DailyOS ability slug suffix.
	 *
	 * @param string $name Inventory ability name.
	 */
	public function normalize_name( string $name ): string {
		return $this->normalize_ability_name( $name );
	}

	/**
	 * Build WP Abilities API registration arguments.
	 *
	 * @param array<string, mixed> $ability Ability descriptor.
	 * @return array<string, mixed>
	 */
	private function build_registration_args( array $ability ): array {
		$mcp_exposure           = $this->normalize_mcp_exposure( $ability['mcp_exposure'] ?? 'None' );
		$client_side_executable = isset( $ability['client_side_executable'] )
			? (bool) $ability['client_side_executable']
			: false;

		return [
			'description'            => isset( $ability['description'] ) ? (string) $ability['description'] : '',
			'category'               => isset( $ability['category'] ) ? (string) $ability['category'] : 'dailyos',
			'input_schema'           => $ability['input_schema'] ?? [],
			'output_schema'          => $ability['output_schema'] ?? [],
			'required_scopes'        => $ability['required_scopes'] ?? [],
			'allowed_actors'         => $ability['allowed_actors'] ?? [],
			'mcp_exposure'           => $mcp_exposure,
			'client_side_executable' => $client_side_executable,
			'execute_callback'       => static function () {
				return new \WP_Error(
					'dailyos_runtime_client_unavailable',
					__( 'DailyOS runtime invocation is not available in this scaffold.', 'dailyos' )
				);
			},
		];
	}

	/**
	 * Preserve the MCP enum separately from browser-side execution eligibility.
	 *
	 * @param mixed $value MCP exposure value.
	 */
	private function normalize_mcp_exposure( mixed $value ): string {
		$allowed = [ 'None', 'MetadataOnly', 'Invocable' ];

		if ( is_string( $value ) && in_array( $value, $allowed, true ) ) {
			return $value;
		}

		return 'None';
	}

	/**
	 * Normalize an inventory name into a DailyOS ability slug suffix.
	 *
	 * @param string $name Inventory ability name.
	 */
	private function normalize_ability_name( string $name ): string {
		$normalized = strtolower( preg_replace( '/[^a-zA-Z0-9_-]+/', '-', $name ) ?? '' );

		return trim( $normalized, '-' );
	}

	/**
	 * Return the empty inventory shape.
	 *
	 * @return array<string, mixed>
	 */
	private function empty_inventory(): array {
		return [
			'schema_version' => self::INVENTORY_SCHEMA_VERSION,
			'abilities'      => [],
		];
	}
}
