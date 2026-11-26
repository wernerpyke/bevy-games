# Jet documentation

Documentation for this game only. General rules that other games may reuse are extensions, in `dev-docs/extensions/`.

| Document | Contents |
|---|---|
| [SPEC.md](SPEC.md) | The specification: rules, values, what is decided, open questions |
| [adr.md](adr.md) | Decisions that apply to Jet only (`adr-jt1` and on) |
| [art-tags.md](art-tags.md) | The palette, the `.aseprite` files, every frame and tag, and where they came from |

Status: being ported from the user's Pyxel game. The port plan is `dev-docs/PLAN-jet-port.md`.

- Run it: `./dev-scripts/rust.sh run jet`. It opens an empty black 960 × 540 window for now.
- Source layout (`games/jet/src/`): `main.rs` builds the app, `camera.rs` has the view height, window size and colours.
- `input/` (git-ignored) holds the raw Pyxel `.aseprite` files. The game never reads it.
