#![cfg(not(target_arch = "wasm32"))]

use anarchy::{Query, Res, World};
use gearbox::{App, AssetContent, spawn, TextureType, BindlessArrayTextureVault, TextureHandle, Camera, GearboxRenderPlugin, LoadableAssetVault, MaterialRef, MeshRef, SimpleTexturedMaterial, Transform};
use magician_vgpu::glam::*;
use skeletal::anim::Animator;
use skeletal::{SkeletalMeshLoadType, SkeletalMeshPlugin, SkeletalMeshVault};

#[cfg(not(target_arch = "wasm32"))]
fn main() -> gearbox::anyhow::Result<()> {
    App::new()
        .add_plugin(GearboxRenderPlugin)
        .add_plugin(SkeletalMeshPlugin)
        .on_render_startup(startup_triangle)
        .on_render_update(update_triangle)
        .run()
}

fn startup_triangle(
    world: World,
    meshes: Res<SkeletalMeshVault>,
    textures: Res<BindlessArrayTextureVault>
) {
    spawn(&world, vec![
        Box::new(Transform::new(Vec3::new(0.0, 10.0, 30.0), Quat::from_rotation_x(-0.5), Vec3::ONE)),
        Box::new(Camera::default()),
    ]);

    for z in -1 .. 2 {
        let loaded = meshes.load(&world, AssetContent::LocalPath("./examples/fbx/SK_Character_Alien_Male_01.fbx".into()), SkeletalMeshLoadType::FBX)
            .and_then(|mesh| Ok((mesh, textures.load(&world, AssetContent::LocalPath("./examples/fbx/PolygonSciFiCity_Texture_01_A.png".into()), TextureType::PNG)?)));
        let ((model, _, anims), texture) = match loaded {
            Ok(loaded) => loaded,
            Err(err) => return log::error!("Failed to load the alien model: {err:#}"),
        };
        let texture = TextureHandle::Bindless(texture);
        let material = MaterialRef::new(SimpleTexturedMaterial::new(texture));
        let mut animator = Animator::empty();
        animator.load_animations(anims);

        spawn(&world, vec![
            Box::new(Transform::new(Vec3::new(z as f32 * 5.0, 0.0, 0.0), Quat::IDENTITY, Vec3::ONE * 0.025)),
            Box::new(material),
            Box::new(animator),
            Box::new(MeshRef::new(model)),
        ]);
    }
}

fn update_triangle(
    mut transforms: Query<(&'static mut Transform, &'static MeshRef)>
) {
    while let Some((_id, (mut transform, _))) = transforms.next().unwrap() {
        let rotation = Quat::from_euler(EulerRot::XYZ, 0.0, 0.005 / 3.0, 0.0) * transform.rotation();
        transform.set_rotation(rotation);
    }
}
