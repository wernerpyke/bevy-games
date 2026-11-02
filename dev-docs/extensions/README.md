# Extensions

Our own reusable design decisions, built on top of Bevy. They are not part of Bevy. They are choices that other games in this workspace may want to reuse.

All code still lives inside `games/long-way/src/`. Extracting it into shared code is roadmap item 2 and needs the developer's approval first (`adr-w6`). These documents describe each extension as if it were already separate: the general rules are here, and the values and rules that belong to one game are in `games/<game>/docs/`.

## Where things are documented

| Question | Document |
|---|---|
| How does Bevy itself work? | `dev-docs/bevy/concepts.md`, `dev-docs/bevy/notes.md` |
| What is the extension and how do I use it? | This folder |
| Which numbers and rules does Long Way use? | `games/long-way/docs/` |
| Why was a decision made? | `dev-docs/adr/` |

## Index

| Extension | What it gives | Code today | ADRs |
|---|---|---|---|
| [collision.md](collision.md) | Feet-point collision, merged outlines, forgiving sliding | `game-kit` `collision.rs`, `outline.rs` | lw4, lw10 (lw1, lw13 for Long Way) |
| [collision-research.md](collision-research.md) | Approaches compared, why outlines | none | lw1, lw10 |
| [draw-order.md](draw-order.md) | Depth sort by feet, named layers | `game-kit` `depth.rs`, `layers.rs` | lw2, lw9 (lw12 for Long Way) |
| [map-files.md](map-files.md) | Markdown map format and parser contract | `game-kit` `map.rs` | lw8, lw11 |
| [debug-tools.md](debug-tools.md) | Grid, info box, collision overlay | `game-kit` `debug.rs`, `collision_debug.rs` | lw3, lw6 |
| [dev-feedback.md](dev-feedback.md) | Plain function tests, logging, scripted input, screenshots | `script.rs`, `dev-scripts/rust.sh` | lw5 |
| [plugin-configuration.md](plugin-configuration.md) | Plugin fields stored as a resource | `game-kit` `collision.rs`, `debug.rs` | none |
| [pixel-art.md](pixel-art.md) | View height scale, animation tag switching | `camera.rs`, `player.rs` | e7 |

## Entry template

Every extension document uses these headings, in this order. Leave out a heading only when it has nothing to say.

1. **Summary.** What it is, in a few lines.
2. **Status.** Where the code is today and whether it is ready to share.
3. **Terms.** Words used in the rules.
4. **Parameters the game supplies.** What a game must provide, with the Long Way value.
5. **Specification.** Numbered rules, grouped by topic.
6. **Usage.** How to add and use it.
7. **Limits.** Known gaps.
8. **Tests.** Where they are.

## Rules for writing an extension document

- State the general rule here. Put game values (sizes, speeds, art pixels) in the game's docs and name the parameter here.
- Put history and alternatives in an ADR and link it. Do not repeat the reasoning.
- Plain explanations of Bevy features belong in `dev-docs/bevy/concepts.md`.
- Do not refer to plans or plan steps. Plans are deleted when done.
- Add the extension to the index above.
