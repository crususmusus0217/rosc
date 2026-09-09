# ROSC

**Return Of Spontaneous Circulation** — medical-realism body-building RTS (automation + tower defense).

Engine: **Bevy 0.19.1 (Rust)**. Core sim: custom nodal / sparse `Ax=b` perfusion solver (not engine collision physics).

## Phase 0 (this scaffold)

- Cargo project `rosc` with `nalgebra` + `nalgebra-sparse`
- Hierarchical tick scaffolding (neural 50ms / circulation 250ms / endocrine 2s / growth 30s)
- Vessel graph types + Hagen–Poiseuille resistances
- Steady flow solve + **3-node closed-loop unit tests**
- Empty `organs/` and `ui/` stubs

Not included yet: organ catalog, oral intake, lungs/liver/kidney, control circuits, immune TD, cerebrum / BBB.

## Requirements

- Rust **1.95+** (Bevy 0.19.1 MSRV)
- Linux example deps: `pkg-config`, `libasound2-dev`

```bash
sudo apt install pkg-config libasound2-dev
```

## Commands

```bash
cargo test
cargo run
```

## Bevy 0.19 window note

Resolution is physical `u32` pixels:

```rust
resolution: WindowResolution::new(960, 540),
// or: resolution: (960, 540).into(),
```

Do **not** use the old `(960., 540.).into()` f32 form from Bevy ≤0.15.

## Flow formulation (short)

Edge resistance \(R = 8\eta L / (\pi r^4)\).  
Nodal Kirchhoff at free-pressure nodes; heart / venous reference use fixed pressures.  
See `src/sim/flow.rs`.

## Layout

```
src/
  main.rs          Bevy app entry
  lib.rs
  sim/             graph, flow solver, ticks, transport stub
  organs/          stub (Phase 1+)
  ui/              stub
```

## Repo

Canonical GitHub: https://github.com/crususmusus0217/ROSC

## License

MIT — ROSC contributors
