#!/usr/bin/env bash
set -euo pipefail

# Helper function to create an issue and return its number
create_issue() {
  local title="$1"
  local body="$2"
  local url
  url=$(gh issue create --title "$title" --body "$body")
  echo "$url" | grep -oE "[0-9]+$"
}

# Helper function to update an issue's body
update_issue_body() {
  local num="$1"
  local body="$2"
  gh issue edit "$num" --body "$body"
}

echo "Creating issues..."

# ----------------------------------------------------
# v0.1.0 - Completed
# ----------------------------------------------------
# GP1
gp1_num=$(create_issue "Goalpost v0.1.0-GP1: Deterministic Scalar Profile (bunny-num)" "Track the baseline deterministic fixed-point scalar math profiles.")
s1_1=$(create_issue "Slice 1.1: Setup workspace and numeric conversion helpers (from_f32, to_f32)" "Parent Goalpost: #$gp1_num")
s1_2=$(create_issue "Slice 1.2: Implement type-safe FixedQ32_32 wrapper and standard operators" "Parent Goalpost: #$gp1_num")
s1_3=$(create_issue "Slice 1.3: Implement multiplication/division with Banker's rounding" "Parent Goalpost: #$gp1_num")
s1_4=$(create_issue "Slice 1.4: Implement deterministic square root (sqrt) and integration tests" "Parent Goalpost: #$gp1_num")
update_issue_body "$gp1_num" "Track the baseline deterministic fixed-point scalar math profiles.

### Slices:
- [x] #$s1_1
- [x] #$s1_2
- [x] #$s1_3
- [x] #$s1_4"

# Close completed issues
for num in "$gp1_num" "$s1_1" "$s1_2" "$s1_3" "$s1_4"; do
  gh issue close "$num"
done

# GP2
gp2_num=$(create_issue "Goalpost v0.1.0-GP2: Linear Algebra Primitives (bunny-linalg)" "Track the linear algebra vector representations using deterministic coordinates.")
s2_1=$(create_issue "Slice 2.1: Define Vec2/Vec3 and FixedVec2/FixedVec3 layouts and operators" "Parent Goalpost: #$gp2_num")
s2_2=$(create_issue "Slice 2.2: Implement dot products, cross products, normalization, and integration tests" "Parent Goalpost: #$gp2_num")
update_issue_body "$gp2_num" "Track the linear algebra vector representations using deterministic coordinates.

### Slices:
- [x] #$s2_1
- [x] #$s2_2"

# Close completed issues
for num in "$gp2_num" "$s2_1" "$s2_2"; do
  gh issue close "$num"
done

# GP3
gp3_num=$(create_issue "Goalpost v0.1.0-GP3: Workspace Infrastructure and Code Quality Gates" "Track code standards, formatting, Clippy, and cross-platform CI pipelines.")
s3_1=$(create_issue "Slice 3.1: Establish CODE_STANDARDS.md and enforce linter policies" "Parent Goalpost: #$gp3_num")
s3_2=$(create_issue "Slice 3.2: Implement GitHub Actions workflow for multi-platform determinism" "Parent Goalpost: #$gp3_num")
update_issue_body "$gp3_num" "Track code standards, formatting, Clippy, and cross-platform CI pipelines.

### Slices:
- [x] #$s3_1
- [x] #$s3_2"

# Close completed issues
for num in "$gp3_num" "$s3_1" "$s3_2"; do
  gh issue close "$num"
done

# Close Issue #2 and #3 which were the original open ones corresponding to GP1/GP2 math
gh issue close 2 || true
gh issue close 3 || true

# ----------------------------------------------------
# v0.1.1 - Planned
# ----------------------------------------------------
# GP1: Parent is Issue #1
gp1_v011_num=1
s1_1_v011=$(create_issue "Slice 1.1: Parse and extract @bunnyScalarProfile directive arguments from Wesley IR" "Parent Goalpost: #$gp1_v011_num")
s1_2_v011=$(create_issue "Slice 1.2: Implement dynamic mapping config based on extracted profiles" "Parent Goalpost: #$gp1_v011_num")
update_issue_body "$gp1_v011_num" "compiler: Parse @bunnyScalarProfile directive arguments dynamically instead of hardcoding names.

### Slices:
- [ ] #$s1_1_v011
- [ ] #$s1_2_v011"

# ----------------------------------------------------
# v0.2.0 - Planned
# ----------------------------------------------------
# GP1
gp1_v020_num=$(create_issue "Goalpost v0.2.0-GP1: Core Bounding Shapes (bunny-geom)" "Track core bounding envelopes using fixed-point vectors.")
s1_1_v020=$(create_issue "Slice 1.1: Implement FixedRay3, FixedAabb3, and FixedSphere3 using FixedVec3 coordinates" "Parent Goalpost: #$gp1_v020_num")
s1_2_v020=$(create_issue "Slice 1.2: Implement shape boundary conversion traits for float boundaries" "Parent Goalpost: #$gp1_v020_num")
update_issue_body "$gp1_v020_num" "Track core bounding envelopes using fixed-point vectors.

### Slices:
- [ ] #$s1_1_v020
- [ ] #$s1_2_v020"

# GP2
gp2_v020_num=$(create_issue "Goalpost v0.2.0-GP2: Ray-Casting Queries (bunny-query)" "Track ray-intersection math solvers in bunny-query.")
s2_1_v020=$(create_issue "Slice 2.1: Implement ray-sphere intersection solver" "Parent Goalpost: #$gp2_v020_num")
s2_2_v020=$(create_issue "Slice 2.2: Implement ray-AABB intersection solver" "Parent Goalpost: #$gp2_v020_num")
s2_3_v020=$(create_issue "Slice 2.3: Implement ray-triangle intersection solver" "Parent Goalpost: #$gp2_v020_num")
update_issue_body "$gp2_v020_num" "Track ray-intersection math solvers in bunny-query.

### Slices:
- [ ] #$s2_1_v020
- [ ] #$s2_2_v020
- [ ] #$s2_3_v020"

# GP3
gp3_v020_num=$(create_issue "Goalpost v0.2.0-GP3: Closest Point Queries (bunny-query)" "Track minimum-distance calculations between shapes in bunny-query.")
s3_1_v020=$(create_issue "Slice 3.1: Implement Point-to-Triangle closest point solver" "Parent Goalpost: #$gp3_v020_num")
s3_2_v020=$(create_issue "Slice 3.2: Implement Segment-to-Segment closest point solver" "Parent Goalpost: #$gp3_v020_num")
s3_3_v020=$(create_issue "Slice 3.3: Implement AABB-to-Sphere closest point solver" "Parent Goalpost: #$gp3_v020_num")
update_issue_body "$gp3_v020_num" "Track minimum-distance calculations between shapes in bunny-query.

### Slices:
- [ ] #$s3_1_v020
- [ ] #$s3_2_v020
- [ ] #$s3_3_v020"

# ----------------------------------------------------
# v0.3.0 - Planned
# ----------------------------------------------------
# GP1
gp1_v030_num=$(create_issue "Goalpost v0.3.0-GP1: Stable BVH Tree (bunny-broadphase)" "Track the memory-stable, zero-allocation bounding volume hierarchy (BVH).")
s1_1_v030=$(create_issue "Slice 1.1: Define BVH node layout and array-backed tree layout" "Parent Goalpost: #$gp1_v030_num")
s1_2_v030=$(create_issue "Slice 1.2: Implement SAH tree building algorithm" "Parent Goalpost: #$gp1_v030_num")
s1_3_v030=$(create_issue "Slice 1.3: Implement deterministic BVH ray-traversal solver" "Parent Goalpost: #$gp1_v030_num")
s1_4_v030=$(create_issue "Slice 1.4: Implement BVH box overlap query" "Parent Goalpost: #$gp1_v030_num")
update_issue_body "$gp1_v030_num" "Track the memory-stable, zero-allocation bounding volume hierarchy (BVH).

### Slices:
- [ ] #$s1_1_v030
- [ ] #$s1_2_v030
- [ ] #$s1_3_v030
- [ ] #$s1_4_v030"

# GP2
gp2_v030_num=$(create_issue "Goalpost v0.3.0-GP2: Sweep-and-Prune Solver (bunny-broadphase)" "Track multi-axis collision sweeps.")
s2_1_v030=$(create_issue "Slice 2.1: Implement 1D/3D sorting and sweep overlap queries" "Parent Goalpost: #$gp2_v030_num")
s2_2_v030=$(create_issue "Slice 2.2: Implement active-pair generator with stable sorting" "Parent Goalpost: #$gp2_v030_num")
update_issue_body "$gp2_v030_num" "Track multi-axis collision sweeps.

### Slices:
- [ ] #$s2_1_v030
- [ ] #$s2_2_v030"

# ----------------------------------------------------
# v0.4.0 - Planned
# ----------------------------------------------------
# GP1
gp1_v040_num=$(create_issue "Goalpost v0.4.0-GP1: Compressed Mesh Layouts (bunny-mesh)" "Track quantized mesh layouts and content-addressable hashes.")
s1_1_v040=$(create_issue "Slice 1.1: Implement 16-bit integer quantization mapping for vertices" "Parent Goalpost: #$gp1_v040_num")
s1_2_v040=$(create_issue "Slice 1.2: Implement index buffer triangulation layouts" "Parent Goalpost: #$gp1_v040_num")
s1_3_v040=$(create_issue "Slice 1.3: Implement content-addressable hashing for mesh assets" "Parent Goalpost: #$gp1_v040_num")
update_issue_body "$gp1_v040_num" "Track quantized mesh layouts and content-addressable hashes.

### Slices:
- [ ] #$s1_1_v040
- [ ] #$s1_2_v040
- [ ] #$s1_3_v040"

# GP2
gp2_v040_num=$(create_issue "Goalpost v0.4.0-GP2: File Format Adapters (bunny-codec)" "Track zero-copy mesh format parsers.")
s2_1_v040=$(create_issue "Slice 2.1: Implement zero-copy PLY binary parser" "Parent Goalpost: #$gp2_v040_num")
s2_2_v040=$(create_issue "Slice 2.2: Implement zero-copy OBJ parser" "Parent Goalpost: #$gp2_v040_num")
s2_3_v040=$(create_issue "Slice 2.3: Create fixture regression suites using Stanford Bunny sample meshes" "Parent Goalpost: #$gp2_v040_num")
update_issue_body "$gp2_v040_num" "Track zero-copy mesh format parsers.

### Slices:
- [ ] #$s2_1_v040
- [ ] #$s2_2_v040
- [ ] #$s2_3_v040"

echo "All issues created successfully!"
