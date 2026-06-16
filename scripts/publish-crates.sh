#!/usr/bin/env bash
set -euo pipefail

readonly TOOLCHAIN="${RUST_TOOLCHAIN:-1.96.0}"
readonly RETRY_LIMIT="${CRATES_IO_RETRY_LIMIT:-40}"
readonly RETRY_SECONDS="${CRATES_IO_RETRY_SECONDS:-15}"

readonly -a CRATES=(
  bunny-num
  bunny-contract
  bunny-wesley
  bunny-linalg
  bunny-geom
  bunny-query
  bunny-broadphase
  bunny-mesh
  bunny-codec
)

readonly -a ROOT_CRATES=(
  bunny-num
  bunny-contract
  bunny-wesley
)

usage() {
  cat <<'USAGE'
Usage: scripts/publish-crates.sh <verify|dry-run|publish>

verify   Verify first-publish readiness without uploading.
dry-run  Dry-run crates whose internal dependencies are registry-visible.
publish  Publish every publishable Bunny crate to crates.io in dependency order.

Environment:
  RELEASE_TAG                 Optional tag guard; must equal v<crate-version>.
  CARGO_REGISTRY_TOKEN        Required by cargo for publish mode.
  RUST_TOOLCHAIN              Rust toolchain to use; defaults to 1.96.0.
  ALLOW_DIRTY=1               Pass --allow-dirty to Cargo packaging commands.
  CRATES_IO_RETRY_LIMIT       Registry search attempts after each publish.
  CRATES_IO_RETRY_SECONDS     Sleep duration between registry attempts.
USAGE
}

cargo_cmd() {
  cargo "+${TOOLCHAIN}" "$@"
}

package_version() {
  cargo_cmd pkgid --package "$1" | sed -E 's/.*#([^#]+)$/\1/'
}

validate_versions() {
  cargo_cmd metadata --locked --no-deps --format-version 1 >/dev/null

  local expected
  expected="$(package_version "${CRATES[0]}")"

  for crate in "${CRATES[@]}"; do
    local actual
    actual="$(package_version "$crate")"
    if [[ "$actual" != "$expected" ]]; then
      printf 'version mismatch: %s is %s, expected %s\n' \
        "$crate" "$actual" "$expected" >&2
      exit 1
    fi
  done

  if [[ -n "${RELEASE_TAG:-}" && "${RELEASE_TAG}" != "v${expected}" ]]; then
    printf 'release tag mismatch: %s does not match crate version v%s\n' \
      "${RELEASE_TAG}" "$expected" >&2
    exit 1
  fi

  printf 'validated publishable crate version: %s\n' "$expected"
}

cargo_dirty_args() {
  [[ "${ALLOW_DIRTY:-}" == "1" ]]
}

verify_packages() {
  local -a dirty_args=()
  if cargo_dirty_args; then
    dirty_args=(--allow-dirty)
  fi

  for crate in "${CRATES[@]}"; do
    if [[ "${VERIFY_REGISTRY_DEPS:-}" == "1" ]] \
      || array_contains "$crate" "${ROOT_CRATES[@]}"; then
      printf '::group::cargo package %s\n' "$crate"
      cargo_cmd package --locked --package "$crate" "${dirty_args[@]}"
      printf '::endgroup::\n'
    else
      printf '::group::cargo package --list %s\n' "$crate"
      cargo_cmd package --locked --list --package "$crate" "${dirty_args[@]}"
      printf '::endgroup::\n'
    fi
  done
}

dry_run_publish() {
  local -a dirty_args=()
  if cargo_dirty_args; then
    dirty_args=(--allow-dirty)
  fi

  for crate in "${CRATES[@]}"; do
    if [[ "${VERIFY_REGISTRY_DEPS:-}" == "1" ]] \
      || array_contains "$crate" "${ROOT_CRATES[@]}"; then
      printf '::group::cargo publish --dry-run %s\n' "$crate"
      cargo_cmd publish --locked --dry-run --package "$crate" "${dirty_args[@]}"
      printf '::endgroup::\n'
    else
      printf '::group::cargo package --list %s\n' "$crate"
      cargo_cmd package --locked --list --package "$crate" "${dirty_args[@]}"
      printf '::endgroup::\n'
    fi
  done
}

crate_version_exists() {
  local crate="$1"
  local version="$2"
  local status

  status="$(curl --silent --location --output /dev/null --write-out '%{http_code}' \
    --user-agent "flyingrobots-bunny-release" \
    "https://crates.io/api/v1/crates/${crate}/${version}" || true)"

  [[ "$status" == "200" ]]
}

publish_packages() {
  : "${CARGO_REGISTRY_TOKEN:?CARGO_REGISTRY_TOKEN must be set for publish mode}"

  local version
  version="$(package_version "${CRATES[0]}")"

  for crate in "${CRATES[@]}"; do
    publish_crate "$crate" "$version"
  done
}

publish_crate() {
  local crate="$1"
  local version="$2"
  local output_file

  if crate_version_exists "$crate" "$version"; then
    printf '%s %s is already published; skipping\n' "$crate" "$version"
    return 0
  fi

  output_file="$(mktemp)"

  for attempt in $(seq 1 "$RETRY_LIMIT"); do
    printf '::group::cargo publish %s attempt %s/%s\n' \
      "$crate" "$attempt" "$RETRY_LIMIT"
    if cargo_cmd publish --locked --package "$crate" 2>&1 | tee "$output_file"; then
      printf '::endgroup::\n'
      rm -f "$output_file"
      return 0
    fi
    printf '::endgroup::\n'

    if crate_version_exists "$crate" "$version"; then
      printf '%s %s became visible after publish returned failure; continuing\n' \
        "$crate" "$version"
      rm -f "$output_file"
      return 0
    fi

    if grep -Eiq 'no matching package named|failed to select a version|429 Too Many Requests|published too many new crates' "$output_file"; then
      printf 'registry dependency or publish limit for %s is not ready; retrying in %ss\n' \
        "$crate" "$RETRY_SECONDS"
      sleep "$RETRY_SECONDS"
      continue
    fi

    rm -f "$output_file"
    return 1
  done

  rm -f "$output_file"
  printf 'timed out publishing %s %s after %s attempts\n' \
    "$crate" "$version" "$RETRY_LIMIT" >&2
  return 1
}

array_contains() {
  local needle="$1"
  shift

  local item
  for item in "$@"; do
    if [[ "$item" == "$needle" ]]; then
      return 0
    fi
  done

  return 1
}

main() {
  local mode="${1:-verify}"

  case "$mode" in
    verify | dry-run | publish) ;;
    -h | --help)
      usage
      exit 0
      ;;
    *)
      usage >&2
      exit 2
      ;;
  esac

  validate_versions

  case "$mode" in
    verify) verify_packages ;;
    dry-run) dry_run_publish ;;
    publish) publish_packages ;;
  esac
}

main "$@"
