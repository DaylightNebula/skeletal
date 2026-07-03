use std::fs::File;
use std::io::BufReader;
use std::path::PathBuf;

use anarchy::{EntityBuilder, Query, Res, WorldDatabase};
use anarchy::macros::system;
use cell::{App, Graphics};
use gearbox::{BasicMaterial, Camera, MaterialRef, MeshRef, RenderPlugin, Transform};
use gltf::Gltf;
use magician_vgpu::glam::*;
use skeletal::loader;

fn main() -> anyhow::Result<()> {
    App::new()
        .add_plugin(RenderPlugin)
        .on_render_startup(startup_triangle)
        .on_render_update(update_triangle)
        .run()
}

#[system]
fn startup_triangle(
    graphics: Res<Graphics>
) {
    world.insert(
        EntityBuilder::default()
            .add(Transform::new(Vec3::new(0.0, 10.0, 30.0), Quat::from_rotation_x(-0.5), Vec3::ONE))
            .add(Camera::default())
            .build()
    );

    let path: PathBuf = "./examples/gltf/Barbarian.glb".into();
    println!("Loading path {:?} {:?}", std::env::current_dir(), std::fs::canonicalize(&path));
    let file = File::open(&path)?;
    let gltf = Gltf::from_reader(BufReader::new(file))?;
    let model = loader::load(gltf, &*graphics, &path, &path, None);

    world.insert(
        EntityBuilder::default()
            .add(Transform::new(Vec3::ZERO, Quat::IDENTITY, Vec3::ONE * 3.0))
            .add(MaterialRef::new(BasicMaterial::new(Vec4::new(0.8, 0.2, 0.4, 1.0))))
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
