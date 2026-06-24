#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
DEST="${ROOT_DIR}/upstream"
mkdir -p "$DEST"

clone_if_missing() {
  local url="$1"
  local dir="$2"
  if [ -d "${DEST}/${dir}/.git" ]; then
    echo "[skip] ${dir} already cloned"
    return 0
  fi
  echo "[clone] ${dir}"
  git clone --depth 1 --filter=blob:none "$url" "${DEST}/${dir}"
}

# Priority set
clone_if_missing https://github.com/rethinkdb/rethinkdb.git rethinkdb
clone_if_missing https://github.com/kuzudb/kuzu.git kuzu
clone_if_missing https://github.com/arangodb/arangodb.git arangodb
clone_if_missing https://github.com/apache/age.git age
clone_if_missing https://github.com/JanusGraph/janusgraph.git janusgraph
clone_if_missing https://github.com/vesoft-inc/nebula.git nebula

echo "Done. Repos are in: ${DEST}"
