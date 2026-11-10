# Developer documentation

| Group | Where | Contents |
|---|---|---|
| New game workflow | `WORKFLOW.md` | Repeatable steps to start a game: interview, spec, decisions, wiring to the kit. |
| Shared code | `../crates/game-kit/` | The kit crate. Its extensions are in `extensions/`, the decisions in `adr/EXTENSIONS.md`. |
| Decisions | `adr/` | Why a decision was made. One file per topic. Format in `adr/README.md`. |
| Bevy | `bevy/` | What we learned about Bevy itself. `concepts.md` explains how it works, `notes.md` holds verified snippets and pitfalls for the pinned version. |
| Extensions | `extensions/` | Our reusable additions on top of Bevy: what each does, its rules, parameters and limits. `extensions/README.md` is the index and holds the entry template. |
| Plans, roadmap and diagram rules | this folder | `ROADMAP.md` (open items, stable IDs), `ROADMAP-completed.md`, `diagrams.md` (Mermaid rules), and `PLAN-*.md` while a plan is active, `RESEARCH-*.md` for open assessments. |
| Game documents | `games/<game>/docs/` | Values, object kinds, source layout and checks for one game. |

Where to put new content:
- A rule that any game could reuse: an extension document. Name the parameters the game supplies.
- A number, size or object kind of one game: that game's docs.
- A reason for a decision: an ADR.
- An explanation of a Bevy feature: `bevy/concepts.md`. A verified snippet: `bevy/notes.md`.
- Human readers who only want to run a game: `docs/README.md`.
