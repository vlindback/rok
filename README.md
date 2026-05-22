# rok
 
A Rust game engine and simulation platform, built from scratch.
 
## Status
 
Early development. Currently runs a Vulkan render loop showing a clear
color in a window, with a working hot-reload boundary between the engine
and the game target. Most subsystems are scaffolded but not yet wired
together into anything visually interesting.
 
This is a personal project under active development. Expect breaking
changes, undocumented internals, and entire subsystems that exist only
as ARCHITECTURE.md notes.
 
See [`STATE.md`](STATE.md) for the current focus and immediate work
queue.
 
## What rok is
 
rok is a from-scratch game engine and real-time simulation platform
written in Rust. The engine is built on a host/engine/target
architecture: a thin host executable owns OS resources, a statically
linked engine library owns the subsystems, and a hot-reloadable target
DLL owns the actual simulation logic.
 
The intent is to keep the engine itself in Rust for systems-level
performance, while exposing a stable C-ABI surface (`rok-abi`) that
will eventually be the binding target for scripting and visual scripting
layers.
 
## What rok will become
 
The long-term direction is a full simulation and game development
platform with:
 
- A Vulkan-based PBR renderer with data-driven pipelines
- A custom physics engine
- An asset system with hot-reload support
- A scene/entity model
- An editor for content authoring
- An embedded scripting language as the primary user-facing surface
- A visual scripting layer on top of that scripting language
- A standalone profiler that hooks into the engine over IPC
Rust-native user code remains supported — the architecture is open to
it — but scripting will be the encouraged path. The engine is the
hard-real-time core; user simulation logic lives a layer up.
 
This is a multi-year project. There is no roadmap with dates, only a
direction and an ordered queue of work.
 
## Building
 
Requires:
 
- A recent stable Rust toolchain (`cargo`)
- A Vulkan 1.3-capable GPU and up-to-date drivers
- On Windows: standard MSVC build tools (installed automatically with
  `rustup-init` on Windows)
- On Linux: a Wayland-capable display server (Wayland support is
  partially scaffolded; X11 is not currently planned)
Build and run:
 
```
cargo build
cargo run --bin rok-host -- game
```
 
The `game` argument selects the target config in
`config/targets/game/game.cfg`, which currently loads `rok_game.dll`
(or the platform equivalent).
 
## Project structure
 
```
rok/
├── bin/
│   └── rok-host/             Thin executable: owns OS window + event loop
├── libs/
│   ├── rok-abi/              Stable C-ABI surface for the target boundary
│   ├── rok-core/             Glue crate (currently empty)
│   ├── rok-engine/           Engine, statically linked into the host
│   ├── rok-io/               Async I/O (Windows IoRing, Linux io_uring)
│   ├── rok-jobs/             Work-stealing job system
│   ├── rok-log/              Logging with cross-DLL routing
│   ├── rok-math/             SIMD-backed math primitives
│   ├── rok-renderer/         Vulkan renderer
│   └── rok-window/           Platform window and input
├── targets/
│   └── rok-game/             The hot-reloadable game DLL
├── config/                   Per-target config files
├── STATE.md                  Current focus and immediate work queue
└── CONTRIBUTING.md           Project's stance on contributions
```
 
Each crate has (or will have) its own `ARCHITECTURE.md` describing its
shape and rationale.
 
## Contributing
 
rok is a personal project and is not currently accepting external
contributions. See [`CONTRIBUTING.md`](CONTRIBUTING.md) for details.
 
Bug reports and observations are welcome via issues.
 
## License
 
Licensed under the MIT License ([`LICENSE`](LICENSE) or
http://opensource.org/licenses/MIT).
