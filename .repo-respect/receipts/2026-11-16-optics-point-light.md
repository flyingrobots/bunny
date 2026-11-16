# Receipt: Point Light Sample

Task:
flyingrobots/bunny#365: `bunny-point-light/1`, a point light (position, intensity) whose
`LightSample` has an irradiance of intensity times cosine over distance squared and a shadow
ray that ends at the light, beside `DirectionalLight`. A hit at or nearer than the minimum
distance is refused with a typed error.

Files read:
CONTRIBUTING.md, CODE_STANDARDS.md, docs/NUMERIC_CONSTITUTION.md
docs/topics/optics/README.md and test-plan.md
`crates/bunny-optics/src/shading/**` (hit, light, refusal, specular, wide)
crates/bunny-optics/tests/exact/mod.rs, specular_reflection_tests.rs
crates/bunny-query/src/trace.rs

Files edited:
In progress; completed in the final commit of the pull request.

Topic docs:
docs/topics/optics/README.md and test-plan.md

Generated artifacts:
None.

Checks run:
In progress.

Known risks:
In progress.

Human reviewer:
James Ross (maintainer); review pending.
