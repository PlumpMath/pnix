#!/usr/bin/env bash
# Run both clj host-import demos from any cwd.
set -euo pipefail
root="$(cd "$(dirname "$0")" && pwd)"

echo "== clj (eval-file) =="
(cd "$root/clj" && clojure -M -m smoke)

echo "== clj-imports (import ./lib.px) =="
(cd "$root/clj-imports" && clojure -M -m smoke)

echo "OK"
