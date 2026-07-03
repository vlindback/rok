# STATE

Living "where am I right now" doc. Update on context switch. Read first
on resume.

## Now

**First Light — a single 3D cube rendering, then a camera around it.**

`rok-math` is at a usable 1.0 (`Vec2`/`Vec3`/`Vec4`/`Mat4x4`/`Quaternion`,
plus `Lerp` and the `F32x4` SIMD layer). The clear-color renderer works
end to end (dynamic rendering, sync2, timeline-semaphore frame sync,
resize/swapchain recreation). The next real milestone is getting actual
geometry on screen.

The immediate step: stand up a minimal graphics pipeline **built from a
descriptor, not hardcoded**, and record a cube draw at the existing
`// (Future: record draw commands here)` hook in `render_frame`. Pipeline
state is data from day one — that's the thing worth getting right up front,
because hardcoded pipeline logic is exactly what forces the rework we're
avoiding. The cache behind it starts dumb (single slot / trivial map) and
grows into a hashed cache only when usage or profiling asks for it.

Two independent tracks feed this milestone; neither blocks the other:

- **Renderer gains geometry:** pipeline-from-descriptor, shader modules,
  vertex/index buffers (staging upload), depth image + attachment, MVP via
  push constants, and a public renderer API that consumes a render-command
  list instead of only `render()`.
- **Integration gets wired:** the engine must actually call the target
  vtable and build an `EngineApi` (see Next #5) — today the target loads
  but never runs.

Open decision to make first: does the **engine** draw the cube internally
(fastest path to pixels, no plugin boundary involved), or does the
**target** submit it through `EngineApi`? Leaning engine-internal for
first-light to decouple "get a cube on screen" from "wire the whole call
chain," then move draw submission to the target once both halves work.

## Next

In rough order:

1. **Render command list.** A flat `Vec<RenderCommand>` where
   `RenderCommand` is a Rust enum (native tagged union — the direct
   equivalent of the C++ enum+union, no manual tag, exhaustive match).
   Keep variants `Copy`/POD, small, cache-friendly. Renderer drains the
   list at the draw hook. This is the irreversible surface; get its shape
   right, keep the implementation behind it minimal.

2. **Pipeline from descriptor + trivial cache.** A `PipelineDesc` (vertex
   layout, shader handles, render state, color/depth formats for dynamic
   rendering) → pipeline. Start with one pipeline; the "cache" can be a
   single slot. Lean on Vulkan dynamic state (viewport/scissor) to keep
   distinct pipeline objects to a minimum. Hashed lookup + `VkPipelineCache`
   disk serialization deferred until there's more than one pipeline to
   justify them.

3. **Buffer upload path + depth.** Staging-buffer upload for vertex/index
   data; a depth image and depth attachment wired into the dynamic-
   rendering info. Reverse-Z, `[0,1]` depth (already the locked
   convention).

4. **Camera + MVP.** Add `rok-math` as a renderer/engine dependency (not
   wired yet). Push-constant MVP first — simplest thing that moves the
   cube. Descriptor-set UBO path can come later when there's more than one
   object.

5. **Wire the engine ↔ target call chain.** The engine currently stores
   the target vtable as `_vtable` and never calls it, and never constructs
   an `EngineApi`. Build the `EngineApi` instance (log submit, fences,
   schedule, input queries), call `init`/`update`/`render`/`shutdown` at
   the right points, and thread the borrowed `EngineApi` pointer through.
   Until this exists the target is dormant.

6. **Input into the engine.** `rok-host` already pumps raw events into a
   `Vec`, but `FrameInput` has no events field so they're dropped. Add the
   events channel to `FrameInput`, aggregate device state engine-side, and
   expose it (the `EngineApi` input queries are already defined). The
   camera can't move without this.

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

- **Asset system (shaders first).** Triggered when the renderer needs
  real pipelines loaded from disk. Generational arenas, `Asset<T>`
  handles, `AssetSystem` as a view onto host-owned memory. For first-light
  the cube's shaders can be embedded / loaded ad hoc; the real asset
  machinery waits until more than one thing needs loading.

## Loose ends (cheap cleanups, do opportunistically)

- `rok-abi/src/lib.rs` header still describes the engine as a `cdylib` the
  host `dlopen`s and mentions an `EngineVTable`. Engine is now an rlib
  linked into the host; no `EngineVTable` exists. Update the comment.
- Dead empty `rok-math` files: `aabb.rs`, `frustrum.rs`, `ray.rs` (not in
  the module tree). Delete, and fix the `frustrum` → `frustum` spelling if
  re-added.
- `rok-engine/src/target.rs` opens with `// target.s`.

## Conventions for updating this file

- "Now" should be one thing, the thing you'd resume tomorrow. If it grows
  into a list, split the list off into "Next" and pick the lead item.
- "Next" is the queue, in rough order. Don't bother prioritizing past
  ~5 items — the bottom of the list will get reshuffled by reality.
- "Parked" is for conscious deferrals. If something's dropped entirely,
  delete it; don't leave it here as a guilt trip.
- Update on context switch, not on every commit.