#!/usr/bin/env bash
# Dictionary ingest is disabled unless a source passes the SQLite-backed
# legal provenance gate. Deny source details live only in license-deny.sqlite.
set -euo pipefail

echo "unsupported-proprietary-clean-room-required: dictionary ingest is disabled without a clean legal provenance receipt and SQLite deny DB miss. Use project-owned dictionary seeds only." >&2
exit 2
