# STATE

Living "where am I right now" doc. Update on context switch. Read first
on resume.

## Now

**Resuming the math crate (`rok-math`).**

The bare minimum surface for the renderer to lean on:

- `Vec2`, `Vec3`, `Vec4`, `Mat4x4`, `Quaternion`

Decided against (or undecided) for this pass:

- `Mat3x3` — probably not. Will decide if the renderer or transform code
  ever actually wants it.
- `Transform` class — undecided. Holding off until the shape is forced by
  use, not designed in a vacuum.
- `AABB`, `Plane`, `Frustum`, `Ray` — the current stub files will probably
  be deleted for now and re-added when physics / culling / picking
  actually need them. No point shaping these APIs before there's a
  consumer.

Current state of math:
- `F32x4` SIMD layer: done
- `Vec4`: thin wrapper over F32x4, done
- `Vec3`: mostly done (scalar, not SIMD-backed)
- `Mat4x4`: done with tests for determinant, multiply, inverse, transpose
- `Vec2`: empty
- `Quaternion`: empty

## Next

In order:

1. **Math test suite** — after the basic math types exist, build a real
   test suite for `Mat4x4` and `Quaternion` specifically. From prior
   engine experience, matrix and quaternion bugs are either obscure or
   miserable to track down. If any code in the engine deserves
   defensive test coverage, it's this. The existing `Mat4x4` tests are a
   start; quaternion needs the same treatment with worked numerical
   examples verified against an external source (numpy / wolfram).

2. **Renderer resume — first milestone: a moving 3D cube with a camera.**
   The engine should be able to drive the renderer to display a single
   3D cube and let a camera move and rotate around it. This is the
   real first-light test of the engine→renderer integration, not the
   clear-color we have now.

3. **Input system wired into the engine.** Raw input is already
   collected in rok-window and the ABI is defined. What's missing is
   the engine-side device-state aggregation and the route into the
   target via EngineApi. The cube-camera milestone above depends on
   this — can't move a camera without input.

4. **Basic asset system — shaders first.** Triggered by the renderer
   needing real pipelines. Generational arenas, `Asset<T>` handles,
   `AssetSystem` as a view onto host-owned memory. Scope is "load a
   shader from disk, hand back a handle the renderer can use." Bigger
   asset machinery deferred until more types need it.

5. **Dynamic pipelines / pipeline cache.** Static pipelines are not
   acceptable long-term. Need to investigate how Vulkan pipeline caches
   actually work in practice — never built one. Probably: hash the
   pipeline state, look up in cache, miss → compile + insert. The
   driver-side `VkPipelineCache` is a separate concern (serializing
   compiled binaries to disk to skip recompilation across runs).

## Parked

Consciously deferred. None of these are forgotten or dropped — they're
waiting for the right moment.

- **Profiler.** Standalone external program that connects to the engine
  (when profiling is enabled) over sockets or pipes. Engine ships raw
  profiling data over the wire; the profiler is a pure viewer/decoder.
  Picks up the `JobRecord` infrastructure that already exists. Parked
  until there's enough engine to be worth profiling.

- **Animation system.** Way later. After renderer, asset system,
  scene/transform plumbing are solid.

- **Physics system.** Needs a functioning renderer to actually see
  anything. Out of scope until then. (Reminder: this is a from-scratch
  physics engine, not a third-party integration.)

- **Audio system.** Personal background means this isn't a learning
  problem and can slot in "when there's time." Low priority — the engine
  works fine silent for now.

## Conventions for updating this file

- "Now" should be one thing, the thing you'd resume tomorrow. If it grows
  into a list, split the list off into "Next" and pick the lead item.
- "Next" is the queue, in rough order. Don't bother prioritizing past
  ~5 items — the bottom of the list will get reshuffled by reality.
- "Parked" is for conscious deferrals. If something's dropped entirely,
  delete it; don't leave it here as a guilt trip.
- Update on context switch, not on every commit.
