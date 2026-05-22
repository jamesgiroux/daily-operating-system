# wp/dailyos/dev-tools/

Dev-only tooling for the WordPress plugin. **Nothing in this directory ships
with the plugin** — these files are tracked here so Studio environments stay
in sync across rebuilds and contributors.

## mock-runtime-client.php

Intercepts the `dailyos_runtime_client_for_block` filter and returns canned
ability responses so the WordPress blocks render with realistic content
**without** a paired Tauri runtime + real SQLCipher data. Essential for
visual development cycles — without it, every UI iteration needs a full
pairing handshake + live data.

Mocked abilities:

| Ability | Returns |
|---------|---------|
| `get_entity_intelligence` | `EntityIntelligenceEnvelope` for meeting / account / project / person — canned personas match design reference (Acme Corp, Beta Migration, Priya Raman, mtg-acme-renewal-checkpoint). |
| `meeting_prep_status` | Ready status for the Acme renewal checkpoint. |
| `get_daily_briefing` | Available briefing for 2026-05-22 with Acme renewal as next meeting. |
| `claim_receipt` | Empty rows shell. |
| `list_accounts` / `list_open_loops` | Three accounts + two open loops. |

All other ability names fall through to the real plugin runtime client when
paired, or return `WP_Error('mock_unhandled', ...)` when unpaired.

### Enable in Studio

```sh
# From repo root:
ln -sf "$(pwd)/wp/dailyos/dev-tools/mock-runtime-client.php" \
  ~/Studio/dailyos-dev/wp-content/mu-plugins/dailyos-block-showcase.php
```

Or copy if you don't want a symlink:

```sh
cp wp/dailyos/dev-tools/mock-runtime-client.php \
  ~/Studio/dailyos-dev/wp-content/mu-plugins/dailyos-block-showcase.php
```

### Disable

```sh
rm ~/Studio/dailyos-dev/wp-content/mu-plugins/dailyos-block-showcase.php
```

When unpaired AND not mocked, blocks fall through to their `unavailable`
chip per the §10 invariant.

### Updating canned data

Personas + envelope shapes live in the `DailyOS_Mock_Data` class. Each
entity type has a dedicated `<kind>_envelope()` method that builds an
`EntityIntelligenceEnvelope` matching the producer contract at
`src-tauri/abilities-runtime/src/abilities/get_entity_intelligence/contracts.rs`.

When a producer contract changes, update the mock to match — the wire
shape mismatch will surface as silent empty-section chips since the WP
envelope-resolver filters out unrecognized states.
