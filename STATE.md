# STATE

Living "where am I right now" doc. Update on context switch. Read first
on resume.

## Now

**Consolidate lighting — multiple lights, then point lights — before the map stack.**

The renderer draws a textured, directionally-lit cube: albedo sampled
through a descriptor, a directional light delivered through a second
descriptor binding (UBO), per-vertex world-space normals, depth-correct
under reverse-Z, orbiting under a controllable camera. That's the core of
a forward renderer — the sockets for the rest of PBR now exist.

Next step is extending the single directional light into a small array of
lights, then point lights with attenuation. This stays on the cube (no new
geometry primitives), and it's mostly extending the light UBO + a loop in
the fragment shader — no new plumbing. The reason to consolidate lighting
before normal/roughness maps: the maps are only satisfying to look at once
there's real, varied light for them to interact with.

Immediate sub-step: the light UBO goes from one light to `N` lights (a
count + a fixed-size array, or a small storage buffer), and the fragment
shader loops and accumulates. Then point lights add position + attenuation
falloff. Keep it static-light for now — a dynamic light is what will
finally pull the per-frame / multi-buffered UBO into existence, deferred
until then.

## Next

In rough order:

1. **Specular.** One reflection term — camera position (into the same light
   UBO) + `reflect` + a power. Surfaces start reading as materials, not
   flat paint. Last piece before normal maps make sense.

2. **Normal maps.** The "brick-wall ridges look real" one. Bigger jump:
   needs a tangent basis on the vertices (tangent + bitangent) and a second
   sampler in the existing descriptor set. Per-*pixel* normals on top of the
   per-vertex ones we just added.

3. **Roughness / metallic maps → PBR.** More samplers in the same set,
   feeding a physically-based lighting equation. The destination of the
   whole lighting track.

4. **Renderer restructuring (its own mini-project).** `renderer.rs` is
   navigable now but `render_frame` is long. Deliberately deferred until the
   draw path matures — let the command types and per-draw work pull the
   split along real seams (per-command record functions, a `renderer/`
   tree) rather than guessing them now. File the issue, let it ride.

5. **Mesh / asset arc (large, dedicated project).** This is its own long
   list, heavily involving the job system: abstracted filesystem,
   streaming, io_uring. Pulls into existence: a **mesh registry** (GPU
   buffers stay renderer-owned, indexed by `MeshHandle`); `Transform` gains
   `mesh: MeshHandle` and `RenderCommand::DrawMesh` gains a handle field
   (additive — the command seam already exists); OBJ loader first (fastest
   path to a real mesh), then a hand-written glTF loader for the subset we
   export (own it all the way through skinning; keep a reference impl only
   as a differential-test oracle). Primitive generators
   (`primitives::cube() -> MeshData`) feed the *same* registry path as
   loaded meshes — engine produces mesh *data*, never bakes in scene
   content. Blender -> glTF is the target (open Khronos standard).

6. **GPU sub-allocator.** Trigger is allocation *count* approaching the
   driver cap (`maxMemoryAllocationCount`, ~4096), not a date — which lands
   with the mesh/asset arc when resource count climbs. The seam is already
   in place: everything goes through `create_buffer` / `create_image`, so a
   slab allocator drops in behind them invisibly. Evaluate
   `gpu-allocator`/VMA as the measurement baseline before writing our own.
   Measure first — a dozen allocations today doesn't justify it.

## Parked

Consciously deferred. None of these are forgotten or dropped — they're
waiting for the right moment.

- **Second pipeline + dynamic pipeline cache.** Textured-vs-untextured is
  now a *potential* variant, but nothing in the scene needs a distinct
  pipeline yet, so the cache would be built in a vacuum. Waits until a real
  second pipeline (a shadow pass, transparency, post-processing) demands
  it — then its shape is known instead of guessed. The `PipelineDesc` seam
  already exists; the cache lives behind it.

- **Profiler.** Standalone external program that connects to the engine
  (when profiling is enabled) over sockets or pipes. Engine ships raw
  profiling data over the wire; the profiler is a pure viewer/decoder.
  Picks up the `JobRecord` infrastructure that already exists. Parked until
  there's enough engine to be worth profiling.

- **Animation system.** A deliberate learning project (own skinning: inverse-
  bind matrices, joint hierarchies). After the renderer and asset arc are
  solid. Reference glTF impl kept as a correctness oracle, not a dependency.

- **Physics system.** From-scratch, not third-party. The many-objects scene
  path (field of cubes) is the pressure-test rig it'll grow into; entity/
  component evolution of the scene gets pulled by "things that move and
  collide," which a flat `Vec<Transform>` can't serve.

- **Audio system.** Personal background means this isn't a learning problem
  and can slot in "when there's time." Low priority.

## Loose ends (cheap cleanups, do opportunistically)

- **Normal matrix.** The vertex shader uses `mat3(model)` for normals — a
  shortcut that's correct only for rotation + uniform scale. The moment a
  non-uniformly-scaled object appears, this needs the inverse-transpose
  (normal matrix). Load-bearing shortcut, not the final form.
- **Dead `color` vertex attribute.** Superseded by `uv` + `normal`; the
  shader no longer reads it. Already removed in the lighting work — confirm
  no stragglers reference it.
- `rok-abi/src/lib.rs` header still describes the engine as a `cdylib` the
  host `dlopen`s and mentions an `EngineVTable`. Engine is an rlib linked
  into the host; no `EngineVTable` exists. Update the comment.
- Dead empty `rok-math` files: `aabb.rs`, `frustrum.rs`, `ray.rs` (not in
  the module tree). Delete, and fix the `frustrum` -> `frustum` spelling if
  re-added.
- `rok-engine/src/target.rs` opens with `// target.s`.

## Done (this session — squashed from the running log)

- **First Light** — controllable orbit camera around a cube. Wired the
  host->engine->target chain (engine boxed for stable address, `EngineApi`
  built and passed, target `init`/`update`/`render`/`shutdown` called);
  input plumbed (host forwards raw events, engine aggregates into
  `DeviceState`); data-driven pipeline from `PipelineDesc`; vertex/index
  buffers via staging; depth image under reverse-Z (`GREATER`, clear 0.0);
  MVP push constant; orbit camera with Q/E zoom.
- **A Field of Cubes** — `RenderCommand` enum (tagged union) + engine-built
  command list drained by the renderer; `Transform` (T·R·S, matrix derived
  not stored); `Scene { instances: Vec<Transform> }`. Proved the
  scene->commands->renderer seam (1->N objects touched only the producer).
- **Renderer out of lib.rs** — `Renderer` moved to `renderer.rs`; `lib.rs`
  is a thin crate root. (The deeper `render_frame` split is Next #4.)
- **Grouchy Cat on a Cube** — texturing: 24-vertex cube with UVs, `Texture`
  (device-local image + view + sampler, staging upload with layout
  transitions), descriptor set / pool / layout. The descriptor keystone.
- **Let There Be Light** — per-vertex world-space normals, a directional
  light UBO bound as a second descriptor binding, Lambertian `dot(n,l)` +
  ambient floor, modulating the albedo. Static light (write-once UBO).

## Conventions for updating this file

- "Now" should be one thing, the thing you'd resume tomorrow. If it grows
  into a list, split the list off into "Next" and pick the lead item.
- "Next" is the queue, in rough order. Don't bother prioritizing past
  ~5 items — the bottom of the list will get reshuffled by reality.
- "Parked" is for conscious deferrals. If something's dropped entirely,
  delete it; don't leave it here as a guilt trip.
- "Done" is a short squashed record, not a changelog — collapse it as it
  grows; git has the detail.
- Update on context switch, not on every commit.