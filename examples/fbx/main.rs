use anarchy::{EntityBuilder, Query, Res, WorldDatabase, anyhow};
use anarchy::macros::system;
use cell::{App, Graphics};
use gearbox::{AssetContent, AssetVault, BasicMaterial, BindlessArrayTextureVault, Camera, GearboxRenderPlugin, MaterialRef, MeshRef, Transform};
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
    graphics: Res<Graphics>,
    meshes: Res<SkeletalMeshVault>,
    textures: Res<BindlessArrayTextureVault>
) {
    world.insert(
        EntityBuilder::default()
            .add(Transform::new(Vec3::new(0.0, 10.0, 30.0), Quat::from_rotation_x(-0.5), Vec3::ONE))
            .add(Camera::default())
            .build()
    );

    for z in -1 .. 2 {
        // let path: PathBuf = "./examples/fbx/SK_Character_Alien_Male_01.fbx".into();
        // println!("Loading path {:?} {:?}", std::env::current_dir(), std::fs::canonicalize(&path));
        // let scene = ufbx::load_file(path.to_str().expect("Non UTF-8 fbx path"), loader::fbx::load_opts())
        //     .map_err(|e| anyhow::anyhow!("Failed to load fbx: {}", e.description))?;

        // // this fbx's diffuse texture reference is stale (baked from the artist's machine and
        // // under a different filename), so point it at the texture we actually have on disk.
        // let (model, animations) = loader::fbx::load(&*graphics, &scene, &meshes, Some(&path), None, 0);
        let model = meshes.load(AssetContent::LocalPath("./examples/fbx/SK_Character_Alien_Male_01.fbx".to_string()), SkeletalMeshLoadType::FBX)?;

        // let material = model.material().as_ref()
        //     .and_then(|std_mat| std_mat.albedo_texture.as_ref())
        //     .and_then(|albedo_bytes| textures.load(AssetContent::Binary(albedo_bytes.clone().into_boxed_slice())).ok())
        //     .map(|handle| SimpleTexturedMaterial::new(handle))
        //     .map(|textured_mat| MaterialRef::new(textured_mat))
        //     .unwrap_or_else(|| MaterialRef::new(BasicMaterial::new(Vec4::new(0.8, 0.4, 0.2, 1.0))));
        let material = MaterialRef::new(BasicMaterial::new(Vec4::new(0.8, 0.4, 0.2, 1.0)));

        let animator = Animator::empty();
        // animator.play("2H_Melee_Attack_Spin", true);

        world.insert(
            EntityBuilder::default()
                .add(Transform::new(Vec3::new(z as f32 * 5.0, 0.0, 0.0), Quat::IDENTITY, Vec3::ONE * 0.025))
                .add(material)
                .add(animator)
                .add(MeshRef::new(model))
                .build()
        );
    }
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
