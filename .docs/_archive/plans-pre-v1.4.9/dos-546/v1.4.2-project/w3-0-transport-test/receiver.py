#!/usr/bin/env python3
"""HTTP byte-capture receiver for WP HTTP API byte-exactness probe.

Listens on 127.0.0.1:<port>, accepts N connections, captures the raw
request bytes per connection, and writes each to <outdir>/case-<i>.raw.

Sends a minimal HTTP/1.1 200 OK response with Content-Length:2 body "OK"
so wp_remote_post sees a clean exchange.

Usage: receiver.py <port> <expected_connections> <outdir>
"""
import os
import socket
import sys
from pathlib import Path


def read_request(conn: socket.socket) -> bytes:
    """Read until headers complete, then read Content-Length body if present."""
    buf = b""
    conn.settimeout(5.0)
    # Read headers
    while b"\r\n\r\n" not in buf and b"\n\n" not in buf:
        chunk = conn.recv(4096)
        if not chunk:
            return buf
        buf += chunk
        if len(buf) > 1_000_000:
            return buf

    if b"\r\n\r\n" in buf:
        head, sep, rest = buf.partition(b"\r\n\r\n")
    else:
        head, sep, rest = buf.partition(b"\n\n")

    # Parse Content-Length
    content_length = 0
    for line in head.split(b"\r\n"):
        if line.lower().startswith(b"content-length:"):
            try:
                content_length = int(line.split(b":", 1)[1].strip())
            except ValueError:
                content_length = 0
            break

    body = rest
    while len(body) < content_length:
        chunk = conn.recv(min(4096, content_length - len(body)))
        if not chunk:
            break
        body += chunk

    return head + sep + body


def main() -> int:
    if len(sys.argv) != 4:
        print("usage: receiver.py <port> <expected_connections> <outdir>", file=sys.stderr)
        return 2
    port = int(sys.argv[1])
    expected = int(sys.argv[2])
    outdir = Path(sys.argv[3])
    outdir.mkdir(parents=True, exist_ok=True)

    s = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
    s.setsockopt(socket.SOL_SOCKET, socket.SO_REUSEADDR, 1)
    s.bind(("127.0.0.1", port))
    s.listen(8)
    s.settimeout(30.0)

    reply = b"HTTP/1.1 200 OK\r\nContent-Length: 2\r\nConnection: close\r\n\r\nOK"

    print(f"listening on 127.0.0.1:{port}, expecting {expected} connections", flush=True)

    for i in range(expected):
        try:
            conn, addr = s.accept()
        except socket.timeout:
            print(f"timeout waiting for connection #{i+1}", file=sys.stderr)
            return 1
        with conn:
            raw = read_request(conn)
            path = outdir / f"case-{i+1}.raw"
            path.write_bytes(raw)
            print(f"captured #{i+1} from {addr}: {len(raw)} bytes -> {path}", flush=True)
            try:
                conn.sendall(reply)
            except OSError:
                pass

    s.close()
    return 0


if __name__ == "__main__":
    sys.exit(main())
