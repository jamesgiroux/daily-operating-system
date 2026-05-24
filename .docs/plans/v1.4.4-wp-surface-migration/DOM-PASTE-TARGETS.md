# v1.4.4 WP Surface Migration · DOM Paste Targets

Paste full Tauri-rendered DOM captures into these files as each surface becomes
the active parity target:

- `ACCOUNT-DETAIL-HEALTH-DOM-PASTE.html` — populated; currently used for Health, Context, and the Work sections present in that capture
- `ACCOUNT-DETAIL-CONTEXT-DOM-PASTE.html`
- `ACCOUNT-DETAIL-WORK-DOM-PASTE.html` — paste a full Work-view capture here when validating `commitments`, `suggestions`, `programs`, and `shared`
- `PERSON-DETAIL-DOM-PASTE.html`
- `PROJECT-DETAIL-DOM-PASTE.html`
- `ACTION-DETAIL-DOM-PASTE.html`
- `DAILY-BRIEFING-DOM-PASTE.html`
- `MEETING-DETAIL-DOM-PASTE.html`

Validator example:

```sh
python3 wp/dailyos/dev-tools/parity-check.py \
  --reference-file .docs/plans/v1.4.4-wp-surface-migration/ACCOUNT-DETAIL-CONTEXT-DOM-PASTE.html \
  the-room
```
