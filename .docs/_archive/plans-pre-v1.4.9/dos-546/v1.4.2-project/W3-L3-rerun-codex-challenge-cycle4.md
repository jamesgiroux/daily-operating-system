VERDICT: APPROVE

## New findings

None.

## Bottom-line

Cycle-4 closes the legitimate-marker rejection. `sc_<32 hex>` runtime ids and absent `projection_version` now pass the activation marker gate, with `projection_version` still shape-gated when present. Targeted regression passed: `./vendor/bin/phpunit --filter test_activation_accepts_real_runtime_marker_shape`.
