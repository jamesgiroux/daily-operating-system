#!/usr/bin/env python3
"""Read GitHub PR labels JSON from stdin, print the wave-WN value if present.

Used by the L3 workflow gate. Stdin format: JSON array of label name strings.
Output: e.g. "W0" or "W1.5". Empty if no wave label found.
"""
import sys
import json
import re

labels = json.load(sys.stdin)
for label in labels:
    m = re.match(r"wave-(W[0-9]+(?:\.[0-9]+)?)$", label)
    if m:
        print(m.group(1))
        sys.exit(0)
print("")
