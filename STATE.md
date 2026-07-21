# STATE

Living "where am I right now" doc. Update on context switch. Read first
on resume.

## Now

**Load a second, different asset — prove the forward pass survives variety.**

The DamagedHelmet renders complete: full glTF/GLB loading (own loader, raw
bytes -> `MeshData`), PBR material stack (albedo, metallic-roughness,
normal, emissive, per-material factors UBO) through a Cook-Torrance forward
shader, node-transform placement, material registry with `MaterialHandle`.
It looks right — metal reads as metal, the emissive HUD glows independent
of lighting.

But every code path has only ever been exercised by *one* well-behaved
asset. The next step flushes out what a single asset hides: put a second,
structurally-different model on screen next to the helmet. Pick something
deliberately different — an OBJ (exercises the no-material fallback path) or
`BoxTextured` (PNG decode instead of the helmet's JPEG, trivial geometry so
bugs are obvious). This immediately pulls the two loose ends below
(fallback material, multi-primitive), and sets up the gallery.

## Next

In rough order:

1. **Fallback material.** A mesh with no material (`material_index == None`,
   every OBJ mesh) must get a sane default `Material` instead of crashing —
   all slots on their 1x1 fallbacks, factors at identity. Pulled the instant
   a second, untextured asset loads.

2. **Multi-primitive / multi-material meshes.** Register *all* of a mesh's
   primitives, not just `meshes[0]`. The loader already produces the data;
   this is the engine registering each primitive as its own draw with its
   own material. Pulled by any multi-material asset.

3. **Sampler settings.** Wrap/filter modes from the glTF `sampler` (the
   helmet uses default REPEAT/LINEAR, so this is invisible until an asset
   needs CLAMP/NEAREST). Rides along with the arbitrary-asset work.

4. **Gallery + free-fly camera.** The capstone that proves "load any mesh":
   several varied Khronos models on screen at once, WASD + mouse-look camera
   to inspect them. The orbit camera can't navigate a gallery; the free-fly
   camera pairs naturally. This is the honest test of the whole asset arc.

5. **Instancing.** Many of the same mesh via SSBO-indexed per-instance data,
   `instanceCount > 1`. Deliberately *after* the gallery — scaling one asset
   to 500 is a better (and more honestly tested) payoff once "several
   different assets" already works. This is also where the per-object model
   matrix migrates from the push constant into an instance SSBO, unifying
   the 1-and-N paths (keeps a single pipeline: SSBO-driven, no instance-rate
   vertex binding).

6. **Shared vertex/index buffer + offsets.** Distinct meshes coexisting in
   one buffer via `firstIndex`/`vertexOffset` — the prerequisite for
   indirect drawing. Pulled by *many distinct meshes*, not by instancing
   (a helmet swarm is one mesh). This is the GPU sub-allocator arriving from
   draw/bind-efficiency pressure rather than the allocation-count trigger.

7. **TextureRegistry + `TextureHandle`.** Where texture *dedup* actually
   lives — keyed by glTF image index, decode-and-upload-once, materials
   reference shared textures instead of owning their own. Pulled by a
   multi-material asset that shares textures across slots/materials. Design
   it origin-agnostic (holds both uploaded and render-target images) so
   render-to-texture drops in later without reshaping it.

8. **Generic `Registry<T>`.** `MeshRegistry` + `MaterialRegistry` +
   `TextureRegistry` are the same `Vec<T>` + `add -> Handle` + `get` +
   `destroy` shape. Extract the generic when TextureRegistry makes it three
   (rule of three) — the shape is only correct once all three exist to
   compare. Add generational handles here (see Parked).

9. **Renderer restructuring (mini-project).** `render_frame` is long and
   `record_draws` does a lot. Deferred until the draw path matures — let
   instancing + the data-driven parts pull the split along real seams
   (per-command record functions, a `renderer/` tree) rather than guessing.

10. **Mesh / asset arc (large, dedicated project).** Abstracted filesystem,
    streaming, io_uring, job-system-driven parallel loads. The loaders stay
    pure (bytes -> data); the asset system (VFS, streaming) is a separate
    layer that *feeds* bytes to loaders. `include_bytes!` is the current
    bastardized stand-in, ripped out here.

11. **GPU sub-allocator.** Trigger is allocation *count* nearing
    `maxMemoryAllocationCount` (~4096), which lands with the asset arc as
    resource count climbs. Seam already in place (everything goes through
    `create_buffer`/`create_image`). Evaluate `gpu-allocator`/VMA as the
    baseline before writing our own. Measure first.

## Parked

Consciously deferred. None forgotten — waiting for the right moment.

- **rok-mesh full decoupling + the ABI mesh-data problem.** rok-mesh must
  decouple from renderer and engine. The real issue: *targets* (game,
  editor) load mesh data, and targets only have `EngineApi` from rok-abi,
  not the engine — so mesh data has to cross the dlopen ABI boundary. That
  needs a flat, `repr(C)`, `Vec`-free representation on the ABI (f32 arrays,
  not `MeshData`'s `Vec`/`String`), with rich `MeshData` living host-side
  and converted at the seam. Deferred until targets actually load meshes —
  building the ABI representation now designs against a boundary nothing
  crosses yet.

- **Descriptor pool ownership.** Each `Material` owns its own descriptor
  pool. Should belong to a central descriptor *allocator* instead. Pulled
  when materials get many/dynamic (the asset arc), pairs with shared
  samplers as one "central resource ownership" pass.

- **Shared samplers.** Each `Texture` owns its sampler (redundant). Should
  be a shared sampler set on the renderer. Pairs with the descriptor
  allocator above.

- **Generational handles.** Registry handles are bare `u32` indices —
  positional, so they corrupt when assets *unload* (indices shift, or a
  reused slot resolves a stale handle to the wrong asset — the ABA problem).
  Fix is `{ index, generation }` with a per-slot generation check. Pulled by
  runtime *unloading* (streaming) — nothing unloads yet, so bare index is
  correct for now. Lands with the generic `Registry<T>` (add generation
  once, get it everywhere). Newtype handles (`MeshHandle`/`MaterialHandle`)
  are already distinct — that's the cheap half, done.

- **Material binding-scheme single source of truth.** The material binding
  layout (which binding is which sampler/UBO) is duplicated by hand across
  `MaterialLayout`, the pool/writes in `Material`, and the shader. Any slot
  change is a three-place edit with a validation error if one is missed (bit
  us twice). One declaration everything derives from. Renderer quality pass.

- **Draw-call sorting + redundant-bind skip.** Sort draws by
  mesh/material/pipeline and skip re-binding already-bound resources
  (`bound_mesh`/`bound_material`). Note: transparency breaks sort-by-state
  (needs back-to-front). Pulled by a profile showing bind overhead — needs
  far more geometry first.

- **`dynamic_offsets` for per-object UBO data.** One buffer + offset instead
  of N sets, for per-object UBO data when object count explodes. Localized
  change, only worth it at scale.

- **Occlusion (5th material slot).** Multiplies the *ambient* term only —
  near-invisible against the current flat `0.03` ambient, so no value yet.
  Note: DamagedHelmet ships AO as a *separate* texture, NOT packed in the
  metallic-roughness `.r` (that channel is empty on this asset). Pulled by
  real ambient / image-based lighting for it to modulate.

- **Vec3/Vec4/Quat <-> array conversion standardization.** The gltf work
  showed inconsistent conversions to/from `[f32; N]` (`from_array`,
  `from_vec3`, `.into()`, `from_cols_slice`). Standardize the naming +
  component-order conventions across the math crate. Cheap, do in a math
  quality pass.

- **HDR + tone mapping.** Fixes highlight clipping past intensity 1.0 (the
  point light blows to white now). Also what makes emissive *bloom* instead
  of just clamp-bright. Self-contained; pulled when the clipping annoys.

- **Second pipeline + dynamic pipeline cache.** Nothing needs a distinct
  pipeline yet. Waits for a real second pipeline (shadow pass, transparency,
  post) to demand it — then its shape is known. The debug axis-gizmo (below)
  is the gentlest thing that could pull it first.

- **Debug axis gizmo + free-fly camera.** Bright R/G/B world-axis lines to
  orient in the scene (would have made several orientation bugs obvious at a
  glance). It's the first real *second pipeline* (line-list topology, own
  tiny shader, depth-tested overlay) — good pipeline-arc groundwork. Camera
  pairs with the gallery (Next #4).

- **Profiler.** Standalone external viewer connecting over sockets/pipes;
  engine ships raw `JobRecord` data, profiler decodes. Parked until there's
  enough engine to profile.

- **Animation system.** Deliberate learning project (own skinning: inverse-
  bind matrices, joint hierarchies). glTF already models nodes/scenes; the
  animation/skin *schema* is unmodeled until this is pulled. Reference glTF
  impl kept as correctness oracle. Note: extracting a skeleton is an
  engine-side skeleton/channel extraction, NOT preserving glTF's node tree
  in the loader output.

- **Physics system.** From-scratch. The many-objects scene (demolition
  derby highlighted for dogfooding) is the pressure-test rig. Entity/scene
  model evolution gets pulled by "things that move and collide" — a flat
  instance list can't serve it. The entity/scene model is an explicitly
  unresolved, irreversible decision; the flat `Vec<LoadedGltfMesh>` +
  per-instance transform deliberately doesn't prejudge it.

- **Audio system.** Not a learning problem (personal background); slots in
  when there's time. Low priority.

- **Custom global allocator.** Trial `mimalloc`/`rpmalloc` against real
  profiles first — deferred until profiling infrastructure exists.

- **Fullscreen + minimize handling.** No proper fullscreen support yet.
  Minimizing the window emits Vulkan spec-violation errors (zero-extent
  swapchain / present on a minimized surface). Fix the minimize path
  (skip render on zero extent is partial; the errors need proper handling).

## Loose ends (cheap cleanups, do opportunistically)

- **`cmd_draw_indexed` extra params.** Currently
  `cmd_draw_indexed(cmd, index_count, 1, 0, 0, 0)` — the `instanceCount`,
  `firstIndex`, `vertexOffset`, `firstInstance` params are hardcoded. They
  become live with instancing (Next #5) and the shared buffer (Next #6);
  noting they exist.
- **Dead Blinn-Phong lines in `forward.frag`.** The old `shininess` /
  `spec_strength` / `pow`-based specular and any leftover debug `out_color`
  lines from the BRDF build — confirm they're gone.
- **Normal matrix.** Vertex shader uses `mat3(model)` for normals — correct
  only for rotation + uniform scale. A non-uniformly-scaled object needs the
  inverse-transpose normal matrix. Load-bearing shortcut.
- `rok-abi/src/lib.rs` header still describes the engine as a `cdylib` the
  host `dlopen`s with an `EngineVTable`. Engine is an rlib linked into the
  host; no `EngineVTable`. Update the comment.
- Dead empty `rok-math` files: `aabb.rs`, `frustrum.rs`, `ray.rs` (not in
  the module tree). Delete; fix `frustrum` -> `frustum` if re-added.
- `rok-engine/src/target.rs` opens with `// target.s`.
- `parse_glb` has a leftover `println!` dumping load stats — remove or route
  through the logger.
- **glb chunk-walk robustness.** `parse_glb` assumes JSON-then-optional-BIN
  at fixed offsets. Spec allows unknown chunk types and trailing chunks; a
  chunk-walking loop that classifies by type is the robust form. No current
  asset trips it.

## Done (squashed)

- **First Light** — orbit camera around a cube; host->engine->target chain;
  data-driven pipeline from `PipelineDesc`; staging buffers; reverse-Z depth
  (`GREATER`, clear 0.0); MVP push constant.
- **A Field of Cubes** — `RenderCommand` enum + engine-built command list
  drained by the renderer; `Transform` (T·R·S derived); `Scene`. Proved the
  scene->commands->renderer seam.
- **Grouchy Cat on a Cube** — texturing: UV cube, `Texture` (device-local
  image + view + sampler, staging + layout transitions), descriptor
  set/pool/layout.
- **Let There Be Light** — per-vertex world normals, directional light UBO,
  Lambertian + ambient.
- **Materials II** — multiple lights (fixed-cap UBO array), point lights with
  inverse-square attenuation, Blinn-Phong specular; albedo + normal (TBN) +
  roughness maps; per-slot texture formats; 1x1 fallback textures.
- **A Real Mesh** — `rok-mesh` crate; OBJ loader (triangulation, dedup,
  tangent generation, sub-mesh split); `MeshRegistry` + `MeshHandle`;
  `RenderCommand::DrawMesh`. Suzanne on screen.
- **Complex Geometry (glTF/GLB)** — own glb container parser + full static
  schema (serde) + accessor walker; `Vec<MeshData>` per primitive; node-graph
  traversal with T·R·S/matrix transforms (`from_trs` reuses canonical
  `to_mat4x4`); tangent widened Vec3 -> Vec4 (handedness in `.w`); JPEG +
  PNG decode moved into rok-mesh (`image.rs`); DamagedHelmet renders.
- **PBR** — Cook-Torrance forward shader (GGX / Smith / Fresnel-Schlick,
  metallic routing); `MaterialLayout` hoisted (per-type, renderer-owned);
  `MaterialRegistry` + `MaterialHandle`; `MaterialDesc`/`ImageData` loader
  seam + `MaterialCreateInfo` renderer seam (engine translates); per-material
  factors UBO (std140); emissive. Helmet renders with full PBR material stack.

## Conventions for updating this file

- "Now" is one thing — the thing you'd resume tomorrow.
- "Next" is the queue in rough order; don't prioritize past ~5, reality
  reshuffles the tail.
- "Parked" is conscious deferrals with their *trigger* noted. Drop (delete)
  anything truly abandoned — don't leave guilt trips.
- "Loose ends" are cheap opportunistic cleanups.
- "Done" is a squashed record, not a changelog — git has the detail.
- Update on context switch, not every commit.