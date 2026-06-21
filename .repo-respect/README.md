# Repo Respect Receipts

Every pull request must add or update a receipt here. Receipts are required for
all contributors and all contribution methods; they are not specific to
AI-assisted work.

Create a receipt template from the repository root with:

```bash
cargo run --locked -p xtask -- repo-respect receipt <short-topic>
```

Minimum receipt fields:

```text
Task:
Files read:
Files edited:
Topic docs:
Generated artifacts:
Checks run:
Known risks:
Human reviewer:
```

Commit trailer:

```text
Repo-Respect-Receipt: .repo-respect/receipts/<id>.md
```

Local hooks run the same checks as CI. If a branch or staged commit changes
non-receipt files without a receipt, the hook fails and prints the template
command above.
