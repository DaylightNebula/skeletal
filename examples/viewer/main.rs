use std::fs::File;
use std::io::BufReader;
use std::path::PathBuf;

use anarchy::macros::{Resource, system};
use anarchy::anyhow::{self, bail};
use anarchy::{EntityBuilder, Query, Res, ResMut, WorldDatabase};
use cell::{EguiPlugin, Graphics};
use cell::{App, EguiCtx, egui::egui};
use gearbox::{BasicMaterial, MaterialRef, MeshRef, SimpleTexturedMaterial, glam::*};
use gearbox::{Camera, GearboxRenderPlugin, Transform};
use gltf::Gltf;
use skeletal::anim::Animator;
use skeletal::loader;
use skeletal::mesh::SkeletalMesh;

#[derive(Debug, Resource)]
pub struct ViewerData {
    pub loop_animations: bool
}

fn main() -> anyhow::Result<()> {
    App::new()
        .add_plugin(GearboxRenderPlugin)
        .add_plugin(EguiPlugin)
        .on_render_startup(setup)
        .on_render_update(update)
        .add_resource(ViewerData { loop_animations: true })
        .run()
}

#[system]
fn setup(
    graphics: Res<Graphics>
) {
    world.insert(
        EntityBuilder::default()
            .add(Transform::new(Vec3::new(0.0, 10.0, 30.0), Quat::from_rotation_x(-0.5), Vec3::ONE))
            .add(Camera::default())
            .build()
    );
    
    let Some(path) = get_path() else { bail!("No path provided") };
    let file = File::open(&path)?;
    let gltf = Gltf::from_reader(BufReader::new(file))?;
    let (model, animations) = loader::load(gltf, &*graphics, &path, &path, None);

    let material = model.material().as_ref()
        .map(|std_mat| std_mat.albedo_texture.as_ref())
        .flatten()
        .map(|albedo_bytes| SimpleTexturedMaterial::from_png(&*graphics, &albedo_bytes).ok())
        .flatten()
        .map(|textured_mat| MaterialRef::new(textured_mat))
        .unwrap_or_else(|| MaterialRef::new(BasicMaterial::new(Vec4::new(0.8, 0.4, 0.2, 1.0))));

    let mut animator = Animator::new(&model, &animations);
    animator.play("2H_Melee_Attack_Spin", true);

    world.insert(
        EntityBuilder::default()
            .add(Transform::new(Vec3::new(0.0, -1.0, 0.0), Quat::IDENTITY, Vec3::ONE * 3.0))
            .add(material)
            .add(animator)
            .add(MeshRef::new(model))
            .build()
    );
}

#[system]
fn update(
    graphics: Res<Graphics>,
    egui: Res<EguiCtx>,
    query: Query<(&mut Animator, &mut MeshRef)>,
    data: ResMut<ViewerData>
) {
    egui::Window::new("Animations").show(&egui.context, |ui| {
        if ui.button("Add animations...").clicked() {
            let Some(path) = rfd::FileDialog::new()
                .set_title("Select a GLTF/GLB file")
                .pick_file() else { return };
            let file = File::open(&path).unwrap();
            let gltf = Gltf::from_reader(BufReader::new(file)).unwrap();
            let (_model, animations) = loader::load(gltf, &*graphics, &path, &path, None);
        
            for (mut animator, mut mesh) in query.as_iter() {
                let Some(mesh) = mesh.0.as_any_mut().downcast_mut::<SkeletalMesh>() else { continue };
                animator.add_preprocessed_animations(mesh, animations.clone().into_iter());
            }
        }

        for (mut animator, _) in query.as_iter() {
            let animations = animator.animations()
                .iter()
                .map(|a| a.0.clone())
                .collect::<Vec<_>>();

            ui.horizontal(|ui| {
                ui.label("Looping");
                ui.checkbox(&mut data.loop_animations, "");
            });

            egui::ScrollArea::vertical()
                .auto_shrink([false; 2])
                .show(ui, |ui| {
                    for anim in animations.into_iter() {
                        if ui.button(&anim).clicked() {
                            animator.play(anim, data.loop_animations);
                        }
                    }
                });
        }
    });

    egui::Window::new("Mesh").show(&egui.context, |ui| {
        for (_, mut mesh) in query.as_iter() {
            let Some(mesh) = mesh.0.as_any_mut().downcast_mut::<SkeletalMesh>() else { continue };
            for (_submesh_id, submesh) in mesh.meshes_mut().iter_mut() {
                ui.checkbox(&mut submesh.visible, submesh.label.clone());
            }
        }
    });
}

fn get_path() -> Option<PathBuf> {
    // Skip args[0] (the binary name) and look for the first real argument
    if let Some(arg) = std::env::args().nth(1) {
        Some(PathBuf::from(arg))
    } else {
        rfd::FileDialog::new()
            .set_title("Select a GLTF/GLB file")
            .pick_file()
    }
}
