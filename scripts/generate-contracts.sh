#!/usr/bin/env bash
set -euo pipefail

cargo run -p bunny-wesley -- \
  schemas/bunny/v0/graphics.graphql \
  --rust crates/bunny-contract/src/generated/graphics.rs \
  --typescript generated/typescript/bunny-graphics.ts \
  --manifest generated/bunny-graphics.manifest.json
