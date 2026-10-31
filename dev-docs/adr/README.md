# Architecture Decision Records

One file per topic. Each file captures the decisions still in force for that topic: why a design was chosen over its alternatives, and how to apply it going forward.

## File naming

`<TOPIC>.md`, for example `ENGINE.md`, `ART.md`, `WORKSPACE.md`.

## Record structure

Each ADR is an H2 heading with a date tag. The ID has the form `adr-<prefix><n>`: a lowercase `adr-` namespace, a stable prefix derived from the file's topic, and a number.

| File | Prefix |
|---|---|
| `ENGINE.md` | `e` |
| `ART.md` | `a` |
| `WORKSPACE.md` | `w` |

IDs are assigned once and never reused or renumbered. A new record gets the next free number in its file. Add a new prefix to this table when a new topic file is created.

```markdown
## adr-e1. Decision title (YYYY-MM-DD)

**Decision.** …

**Rationale.** …

**How to apply.** …
```

- The ID is used for cross-references (for example "see `adr/ENGINE.md` adr-e2").
- The date is when the decision was made or last materially revised.
- Rationale is written in current tense: why the decision holds, not the historical deliberation.
- Superseded decisions are removed or annotated inline.

## Referencing other documents

Reference only long-lived documents: `ROADMAP.md` and its items, other ADRs, `docs/`, and the source. Do not reference `PLAN-*.md` files or their phases. Plans are deleted when done.
