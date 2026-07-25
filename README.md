# Skeletal

> **Work in progress.** The API is still taking shape and may change without notice. Use at your own risk.

Skinned mesh loading and rendering for `cell` + `anarchy`, built on top of `gearbox` and
`magician-vgpu`. Loads glTF and FBX models (mesh, skeleton, skin, and animations) into a shared
engine-native format and renders them with GPU vertex skinning.

## What it does

`skeletal` provides:
- `SkeletalMeshPlugin` — a `cell::App` plugin that registers the mesh vault and its
  render-schedule loading system.
- `SkeletalMeshVault` — an `AssetVault` that loads `.gltf`/`.glb`/`.fbx` files (deduplicated by
  content hash) into `SkeletalMesh`, asynchronously off the render thread except for the final
  GPU upload.
- `SkeletalMeshHandle` — the `Mesh` component to attach to a renderable entity (alongside
  `Transform` and `Animator`). Supports hiding individual sub-meshes by bone label via
  `invisible_bones` (e.g. swapping equipped gear on a character).
- `Animator` — a component holding a set of named animations and which one (if any) is currently
  playing; drives per-bone matrices each frame.
- `loader::gltf` / `loader::fbx` — the two format-specific loaders, each producing the same
  `SkeletalMesh` + animation data so callers don't need to care which format a model came from.
- A skeletal vertex shader (in `shaders/`) that blends up to 4 weighted bone matrices per vertex,
  falling back to a rigid per-node transform for unskinned meshes attached to a bone (props,
  weapons, etc).

This crate is part of a small workspace of sibling crates (`anarchy`, `cell`, `gearbox`, `mutual`,
`shader_magician/magician-vgpu`) referenced by relative path in `Cargo.toml`, so it is not
currently usable as a standalone dependency outside that workspace layout.

## Usage

```rust
use anarchy::{EntityBuilder, Query, Res, WorldDatabase, anyhow};
use anarchy::macros::system;
use cell::{App, Graphics};
use gearbox::{AssetContent, AssetVault, Camera, GearboxRenderPlugin, MaterialRef, MeshRef, Transform};
use magician_vgpu::glam::*;
use skeletal::anim::Animator;
use skeletal::{SkeletalMeshLoadType, SkeletalMeshPlugin, SkeletalMeshVault};

fn main() -> anyhow::Result<()> {
    App::new()
        .add_plugin(GearboxRenderPlugin)
        .add_plugin(SkeletalMeshPlugin)
        .on_render_startup(setup)
        .run()
}

#[system]
fn setup(graphics: Res<Graphics>, meshes: Res<SkeletalMeshVault>) {
    world.insert(
        EntityBuilder::default()
            .add(Transform::new(Vec3::new(0.0, 10.0, 30.0), Quat::from_rotation_x(-0.5), Vec3::ONE))
            .add(Camera::default())
            .build()
    );

    let mesh = meshes.load(AssetContent::LocalPath("model.glb".into()), SkeletalMeshLoadType::GLTF)?;
    let material = MaterialRef::new(mesh.material().clone());

    let mut animator = Animator::empty();
    animator.play("Idle", true);

    world.insert(
        EntityBuilder::default()
            .add(Transform::new(Vec3::ZERO, Quat::IDENTITY, Vec3::ONE))
            .add(material)
            .add(animator)
            .add(MeshRef::new(mesh))
            .build()
    );
}
```

See `examples/gltf/main.rs`, `examples/fbx/main.rs`, and `examples/viewer/main.rs` (an interactive
egui-based viewer for picking a model file, playing its animations, and toggling bone visibility)
for full runnable versions.

## Running the examples

```sh
cargo run --example gltf
cargo run --example fbx
cargo run --example viewer -- path/to/model.glb
```
