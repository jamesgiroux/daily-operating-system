<?php
/**
 * DailyOS projection envelope value object.
 *
 * @package DailyOS
 */

declare(strict_types=1);

namespace DailyOS;

/**
 * W3-A-pinned projection row shape for every W4-C-signed projection row.
 *
 * Every storage row destined for W4-C signature carries:
 * dailyos_canonical_id, dailyos_signature, dailyos_source_runtime, and
 * dailyos_projection_version. This object only preserves that envelope.
 */
final class DailyOS_Projection_Envelope {
	private string $dailyos_canonical_id;
	private string $dailyos_signature;
	private string $dailyos_source_runtime;
	private string $dailyos_projection_version;

	/**
	 * Constructor.
	 */
	public function __construct(
		string $dailyos_canonical_id,
		string $dailyos_signature,
		string $dailyos_source_runtime,
		string $dailyos_projection_version
	) {
		$this->dailyos_canonical_id       = $dailyos_canonical_id;
		$this->dailyos_signature          = $dailyos_signature;
		$this->dailyos_source_runtime     = $dailyos_source_runtime;
		$this->dailyos_projection_version = $dailyos_projection_version;
	}

	/**
	 * Return the canonical projection ID.
	 */
	public function get_dailyos_canonical_id(): string {
		return $this->dailyos_canonical_id;
	}

	/**
	 * Return the row signature.
	 */
	public function get_dailyos_signature(): string {
		return $this->dailyos_signature;
	}

	/**
	 * Return the source runtime identifier.
	 */
	public function get_dailyos_source_runtime(): string {
		return $this->dailyos_source_runtime;
	}

	/**
	 * Return the projection version.
	 */
	public function get_dailyos_projection_version(): string {
		return $this->dailyos_projection_version;
	}

	/**
	 * Convert the envelope to its storage array shape.
	 *
	 * @return array<string, string>
	 */
	public function to_array(): array {
		return [
			'dailyos_canonical_id'       => $this->dailyos_canonical_id,
			'dailyos_signature'          => $this->dailyos_signature,
			'dailyos_source_runtime'     => $this->dailyos_source_runtime,
			'dailyos_projection_version' => $this->dailyos_projection_version,
		];
	}
}
