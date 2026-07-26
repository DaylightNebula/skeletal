use anarchy::{EntityBuilder, Query, Res, WorldDatabase, anyhow};
use anarchy::macros::system;
use cell::App;
use gearbox::{AssetContent, BindlessArrayTextureVault, Camera, GearboxRenderPlugin, LoadableAssetVault, MaterialRef, MeshRef, Transform};
use magician_vgpu::glam::*;
use skeletal::anim::Animator;
use skeletal::{SkeletalMeshLoadType, SkeletalMeshPlugin, SkeletalMeshVault};

fn main() -> anyhow::Result<()> {
    App::new()
        .add_plugin(GearboxRenderPlugin)
        .add_plugin(SkeletalMeshPlugin)
        .on_render_startup(startup_triangle)
        .on_render_update(update_triangle)
        .run()
}

#[system]
fn startup_triangle(
    meshes: Res<SkeletalMeshVault>,
    textures: Res<BindlessArrayTextureVault>
) {
    world.insert(
        EntityBuilder::default()
            .add(Transform::new(Vec3::new(0.0, 10.0, 30.0), Quat::from_rotation_x(-0.5), Vec3::ONE))
            .add(Camera::default())
            .build()
    );

    // load mesh and material
    let (model, material, animations) = meshes.load(world, AssetContent::LocalPath("./examples/gltf/Barbarian.glb".into()), SkeletalMeshLoadType::GLTF)?;
    let mut animator = Animator::empty();
    animator.load_animations(animations);
    animator.play("2H_Melee_Attack_Spin", true);

    world.insert(
        EntityBuilder::default()
            .add(Transform::new(Vec3::ZERO, Quat::IDENTITY, Vec3::ONE * 3.0))
            .add(MaterialRef::new(material))
            .add(animator)
            .add(MeshRef::new(model))
            .build()
    );
}

#[system]
fn update_triangle(
    transforms: Query<(&mut Transform, &MeshRef)>
) {
    transforms.as_iter().for_each(|(mut transform, _)| {
        let rotation = Quat::from_euler(EulerRot::XYZ, 0.0, 0.005 / 3.0, 0.0) * transform.rotation();
        transform.set_rotation(rotation);
    });
}
