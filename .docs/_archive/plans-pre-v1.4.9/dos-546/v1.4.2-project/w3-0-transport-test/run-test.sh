#!/usr/bin/env bash
# W3-0 byte-exactness harness.
#
# Starts a Python receiver on a free local port, invokes the PHP probe
# via `studio wp eval-file` against a Studio dev site, then compares
# sha256(input) vs sha256(wire body) for each case.

set -euo pipefail

HERE="$(cd "$(dirname "$0")" && pwd)"
SITE_DIR="${DOS_STUDIO_SITE:-$HOME/Studio/dailyos-dev}"
PORT="${DOS_BYTE_EXACTNESS_PORT:-38791}"
EXPECTED=4

if [[ ! -d "$SITE_DIR" ]]; then
	echo "FAIL Studio site directory not found: $SITE_DIR" >&2
	exit 2
fi

# Stage inside the Studio site (mounted at /wordpress in the sandbox) so
# WP-CLI can both read the PHP file and write/read the input dumps.
STAGE_HOST="$SITE_DIR/wp-content/uploads/dos-w3-0"
STAGE_SANDBOX="/wordpress/wp-content/uploads/dos-w3-0"
rm -rf "$STAGE_HOST"
mkdir -p "$STAGE_HOST"
cp "$HERE/byte-exactness.php" "$STAGE_HOST/byte-exactness.php"
cat > "$STAGE_HOST/config.json" <<EOF
{
  "host": "127.0.0.1",
  "port": $PORT,
  "outdir": "$STAGE_SANDBOX"
}
EOF
OUTDIR="$STAGE_HOST"

echo "harness stage-host=$STAGE_HOST stage-sandbox=$STAGE_SANDBOX port=$PORT site=$SITE_DIR"

# Start receiver in background.
python3 "$HERE/receiver.py" "$PORT" "$EXPECTED" "$OUTDIR" \
	> "$OUTDIR/receiver.log" 2>&1 &
RECEIVER_PID=$!

cleanup() {
	if kill -0 "$RECEIVER_PID" 2>/dev/null; then
		kill "$RECEIVER_PID" 2>/dev/null || true
	fi
}
trap cleanup EXIT

# Give the receiver a moment to bind.
sleep 0.5

# Run the PHP probe via Studio WP-CLI.
echo "--- wp eval-file output ---"
cd "$SITE_DIR"
DOS_BYTE_EXACTNESS_HOST=127.0.0.1 \
DOS_BYTE_EXACTNESS_PORT="$PORT" \
DOS_BYTE_EXACTNESS_OUTDIR="$STAGE_SANDBOX" \
	studio wp eval-file "$STAGE_SANDBOX/byte-exactness.php" || true
echo "--- end wp output ---"

# Wait briefly for the receiver to finish (it self-exits after EXPECTED).
wait "$RECEIVER_PID" 2>/dev/null || true

echo "--- receiver log ---"
cat "$OUTDIR/receiver.log"
echo "--- end receiver log ---"

# Compare per-case.
FAILS=0
PASSES=0
for i in $(seq 1 "$EXPECTED"); do
	INPUT="$OUTDIR/case-$i.input"
	RAW="$OUTDIR/case-$i.raw"

	if [[ ! -f "$INPUT" ]]; then
		echo "case-$i: MISSING input dump (PHP probe did not write it)"
		FAILS=$((FAILS + 1))
		continue
	fi
	if [[ ! -f "$RAW" ]]; then
		echo "case-$i: MISSING wire capture (receiver did not get the connection)"
		FAILS=$((FAILS + 1))
		continue
	fi

	# Extract body from the raw HTTP request (split on the first blank line).
	BODY="$OUTDIR/case-$i.body"
	python3 -c "
import sys, pathlib
raw = pathlib.Path(sys.argv[1]).read_bytes()
sep = b'\r\n\r\n' if b'\r\n\r\n' in raw else (b'\n\n' if b'\n\n' in raw else None)
body = raw.split(sep, 1)[1] if sep else b''
pathlib.Path(sys.argv[2]).write_bytes(body)
" "$RAW" "$BODY"

	INPUT_SHA=$(shasum -a 256 "$INPUT" | awk '{print $1}')
	BODY_SHA=$(shasum -a 256 "$BODY" | awk '{print $1}')
	INPUT_LEN=$(wc -c < "$INPUT" | tr -d ' ')
	BODY_LEN=$(wc -c < "$BODY" | tr -d ' ')

	if [[ "$INPUT_SHA" == "$BODY_SHA" ]]; then
		echo "case-$i: PASS  len=$INPUT_LEN sha=$INPUT_SHA"
		PASSES=$((PASSES + 1))
	else
		echo "case-$i: FAIL  input(len=$INPUT_LEN sha=$INPUT_SHA) vs wire(len=$BODY_LEN sha=$BODY_SHA)"
		echo "  input hexdump (head):"
		xxd "$INPUT" | head -8 | sed 's/^/    /'
		echo "  wire-body hexdump (head):"
		xxd "$BODY" | head -8 | sed 's/^/    /'
		FAILS=$((FAILS + 1))
	fi
done

echo
echo "outdir=$OUTDIR"
echo "summary: passes=$PASSES fails=$FAILS expected=$EXPECTED"

if [[ "$FAILS" -eq 0 && "$PASSES" -eq "$EXPECTED" ]]; then
	echo "VERDICT: PASS"
	exit 0
else
	echo "VERDICT: FAIL"
	exit 1
fi
