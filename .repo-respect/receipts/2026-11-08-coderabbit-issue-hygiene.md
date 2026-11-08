# Receipt: Coderabbit Issue Hygiene

Task:
Stop CodeRabbit from auto-labeling, enriching, and planning issues. Issues carry issue-dag/1 frontmatter, and maintainers own their labels.

Files read:
- `CONTRIBUTING.md`
- `.githooks/commit-msg`
- CodeRabbit configuration schema: `https://coderabbit.ai/integrations/schema.v2.json`

Files edited:
- `.coderabbit.yaml`

Topic docs:
None - configuration for an external review bot; no Bunny behavior changes.

Generated artifacts:
None

Checks run:
- `ruby -ryaml -e 'YAML.load_file(".coderabbit.yaml")'`
- `git diff --cached --check`
- `cargo run --locked -p xtask -- repo-respect check --staged`
- `cargo run --locked -p xtask -- code-dojo-commit-msg <message file>`

Known risks:
None. Pull request reviews are not affected.

Human reviewer:
James Ross (pull request review pending).
