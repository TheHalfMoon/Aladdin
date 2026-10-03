#!/bin/sh
# Inspector qualification for the Qdral local client examples.
# Drives tools/list through three independent client paths and checks the
# 31-tool catalog on each. Requires Node.js, this repository checkout (for
# node apps/qdral-mcp/dist), and a qdrald binary for QDRAL_DAEMON.
# Fails closed on any mismatch. Uses the loopback bearer from the
# environment; export QDRAL_LOOPBACK_TOKEN first (at least 32 characters).
set -eu

cd "$(dirname "$0")/../.."

: "${QDRAL_LOOPBACK_TOKEN:?export QDRAL_LOOPBACK_TOKEN first}"
QDRALD="${QDRAL_DAEMON:-./target/debug/qdrald.exe}"
DIST="apps/qdral-mcp/dist"
EXPECTED="clipboard_read clipboard_write desktop_window_list desktop_window_tree fs_edit fs_find fs_list fs_mkdir fs_move fs_read fs_read_range fs_remove fs_search fs_stat fs_write fs_write_preview git_branch_create git_commit git_diff git_fetch git_fetch_preview git_log git_push git_push_preview git_stage git_status git_unstage process_spawn system_status web_fetch workspace_get"

check() {
  names="$1"
  label="$2"
  if [ "$names" != "$EXPECTED" ]; then
    echo "FAIL $label: got: $names"
    exit 1
  fi
  echo "PASS $label (31 tools)"
}

# Path 1: Inspector CLI over stdio.
out=$(npx -y @modelcontextprotocol/inspector --cli node "$DIST/index.js" \
  --method tools/list -e "QDRAL_DAEMON=$QDRALD" 2>/dev/null \
  | python3 -c "import json,sys; print(' '.join(sorted(t['name'] for t in (lambda t: json.loads(t[t.index(chr(123)):]))(sys.stdin.read())['tools'])))")
check "$out" "inspector-stdio"

# Path 2: Inspector CLI over loopback HTTP.
export QDRAL_LOOPBACK_PORT="${QDRAL_LOOPBACK_PORT:-8377}"
QDRAL_DAEMON="$QDRALD" node "$DIST/entrypoints/loopback_http.js" 2>/dev/null &
srv=$!
cfg=""
trap 'kill $srv 2>/dev/null; [ -n "$cfg" ] && rm -f "$cfg"' EXIT
sleep 4
url="http://127.0.0.1:$QDRAL_LOOPBACK_PORT/mcp"
out=$(npx -y @modelcontextprotocol/inspector --cli --transport http \
  --server-url "$url" --header "Authorization: Bearer $QDRAL_LOOPBACK_TOKEN" \
  --method tools/list 2>/dev/null \
  | python3 -c "import json,sys; print(' '.join(sorted(t['name'] for t in (lambda t: json.loads(t[t.index(chr(123)):]))(sys.stdin.read())['tools'])))")
check "$out" "inspector-http"
kill $srv 2>/dev/null
trap - EXIT

# Path 3: config-file mode with the mcpServers wrapper shape used by
# claude-desktop.json (absolute server entry here because this checkout is
# not an installed release; installed machines use "qdral" on PATH).
cfg="$PWD/$DIST/inspector-smoke-config.json"
winpwd="$(pwd -W)"
printf '{"mcpServers":{"qdral":{"command":"node","args":["%s/%s/index.js"]}}}' \
  "$winpwd" "$DIST" > "$cfg"
out=$(npx -y @modelcontextprotocol/inspector --cli --config "$cfg" \
  --server qdral --method tools/list -e "QDRAL_DAEMON=$QDRALD" 2>"$cfg.err" \
  | python3 -c "import json,sys; print(' '.join(sorted(t['name'] for t in (lambda t: json.loads(t[t.index(chr(123)):]))(sys.stdin.read())['tools'])))") || {
  echo "inspector config mode failed:"
  cat "$cfg.err"
  rm -f "$cfg" "$cfg.err"
  exit 1
}
rm -f "$cfg" "$cfg.err"
check "$out" "inspector-config"