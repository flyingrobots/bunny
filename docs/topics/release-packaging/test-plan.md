# Release Packaging Test Plan

This plan defines how Bunny verifies the release archives. The current chapter is
[`README.md`](README.md).

## Scope

It covers the `packaged-tests` step of `scripts/publish-crates.sh` and the manifest
`exclude` lists it protects. It does not cover publication itself.

## Test Goals

- Prove that a test whose input is missing from the archive fails the release check.
- Prove that the check needs no sibling version on crates.io.
- Prove that the repository passes it.

## Requirements

| ID | Requirement | Current Source |
| --- | --- | --- |
| REL-REQ-001 | Every test file in a published archive compiles from the unpacked archive, with the sibling crates taken from their unpacked archives. | `README.md#packaged-tests` |
| REL-REQ-002 | `scripts/publish-crates.sh verify`, which the release workflow runs, includes that check. | `README.md#modes` |

```toml
[[requirement]]
id = "REL-REQ-001"
summary = "Every test file in a published archive compiles from the unpacked archive, with the sibling crates taken from their unpacked archives."
status = "active"

[[requirement]]
id = "REL-REQ-002"
summary = "`scripts/publish-crates.sh verify`, which the release workflow runs, includes that check."
status = "active"
```

## Test Cases

| ID | Category | Requirements | Oracle | Evidence |
| --- | --- | --- | --- | --- |
| REL-TP-001 | red state | REL-REQ-001 | On the unfixed manifests the check fails and names exactly `ply_ascii_stanford_tests` in `bunny-codec` and `generated_version_tests` in `bunny-contract`. | `scripts/publish-crates.sh` |
| REL-TP-002 | green state | REL-REQ-001 | With the test files excluded beside their inputs, `packaged-tests` exits 0 in a container with the repository's siblings unpublished at that version. | `scripts/publish-crates.sh` |
| REL-TP-003 | wiring | REL-REQ-002 | `verify` calls `verify_packaged_tests` after `verify_packages`. | `scripts/publish-crates.sh` |

```toml
[[case]]
id = "REL-TP-001"
requirements = ["REL-REQ-001"]
evidence = "static-check"
artifact = "scripts/publish-crates.sh"
oracle = "On the unfixed manifests the check fails and names exactly `ply_ascii_stanford_tests` in `bunny-codec` and `generated_version_tests` in `bunny-contract`."
tier = "slow"
status = "implemented"

[[case]]
id = "REL-TP-002"
requirements = ["REL-REQ-001"]
evidence = "static-check"
artifact = "scripts/publish-crates.sh"
oracle = "With the test files excluded beside their inputs, `packaged-tests` exits 0 in a container with the repository's siblings unpublished at that version."
tier = "slow"
status = "implemented"

[[case]]
id = "REL-TP-003"
requirements = ["REL-REQ-002"]
evidence = "static-check"
artifact = "scripts/publish-crates.sh"
oracle = "`verify` calls `verify_packaged_tests` after `verify_packages`."
tier = "slow"
status = "implemented"
```
