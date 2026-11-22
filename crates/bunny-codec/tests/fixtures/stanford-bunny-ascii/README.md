# Stanford Bunny ASCII PLY (reconstruction, resolution 3)

`bun_zipper_res3.ply` is the Stanford Bunny at the lowest reconstruction resolution
(1889 vertices, 3851 triangles; `format ascii 1.0`, vertex properties `x y z confidence
intensity`, faces `property list uchar int vertex_indices`).

* Source: Stanford Computer Graphics Laboratory, Stanford 3D Scanning Repository,
  archive `https://graphics.stanford.edu/pub/3Dscanrep/bunny.tar.gz`, member
  `bunny/reconstruction/bun_zipper_res3.ply`, as recorded in Geordi's asset README
  (`fixtures/render-everywhere/assets/stanford-bunny/README.md`, retrieved 2026-05-23).
* This copy is byte-identical to the file checked in to Geordi, SHA-256
  `975e7f9b160b4ea15b0e225e21b10828ebcf678df020d2f6a46aa408fdcf5cd6`, 142784 bytes.
  Geordi normalized line-end whitespace in it relative to the upstream archive.
* `STANFORD_RECONSTRUCTION_README.txt` is Stanford's reconstruction README, byte-identical
  to Geordi's copy (SHA-256
  `b05569908cfe8f50cbd2babeec2c5453a2ae6e7ed739f238fdbd00a1854a06de`).
* This is Stanford's data, used with attribution. It is not covered by Bunny's licence.
  Read Stanford's README for its terms of use.
* The bytes must never be rewritten: `.gitattributes` marks the directory `-text`, and the
  test `aply_tp_051_geordi_stanford_bunny_asset_reads_with_exact_bounds` checks the exact
  length. The fixture is excluded from the published crate package.
