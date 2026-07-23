use anarchy::{ComponentMeta, Entity, World, anyhow::{self, Context, bail}, extract_comps_distributed, macros::{AsAny, Getters}};
use gearbox::{AssetVault, Handle, Mesh, MeshAssetVault, Transform, glam::Mat4};
use magician_vgpu::{BindGroupProvider, BindableObject, Buffer, DrawSettings, ImmutableBuffer, MutableBuffer, Pipeline, PipelineBuilder, ShaderSource, ShaderType, SinglePass, VirtualGpu, WritableBuffer};
use mutual::{CastableSharedData, CowData, MutCastGuard, RefCastGuard};
use skeletal_shaders::{AnimationInfo, AnimationInfoInput};
use wgpu::ShaderStages;

use crate::{Animator, ModelBone, SkeletalAnimationBuffers, SkeletalMesh, SkeletalMeshVault, SkeletalMeshVertex, instance_buffer_layout, vertex_buffer_layout};

#[derive(AsAny)]
pub struct SkeletalRenderableMesh {
    pub vertices: ImmutableBuffer<[SkeletalMeshVertex]>,
    pub indices: ImmutableBuffer<[u32]>,
}

impl Mesh for SkeletalRenderableMesh {
    fn create_pipeline<'a>(
        &'a self,
        _vgpu: &VirtualGpu
    ) -> PipelineBuilder<'a>
    {
        panic!("Building pipeline from skeletal sub mesh")
    }

    fn draw(
        &self,
        _vgpu: &VirtualGpu,
        pass: &mut SinglePass,
        _world: &World,
        _entity: &Entity
    ) -> anyhow::Result<()> {
        pass.draw(
            &self.vertices, 
            &self.indices, 
            DrawSettings::default()
        );
        Ok(())
    }
}

#[derive(AsAny, Getters)]
pub struct SkeletalMeshHandle {
    pub(crate) handle: Handle<SkeletalMesh>,
    pub(crate) instance_buffer: CowData<MutableBuffer<[Mat4]>>,
    pub(crate) animation_buffers: CowData<SkeletalAnimationBuffers>
}

impl SkeletalMeshHandle {
    pub fn new(handle: Handle<SkeletalMesh>) -> Self {
        Self {
            handle,
            instance_buffer: CowData::null(),
            animation_buffers: CowData::null()
        }
    }
}

impl Clone for SkeletalMeshHandle {
    fn clone(&self) -> Self {
        Self {
            handle: self.handle.clone(),
            instance_buffer: CowData::null(),
            animation_buffers: CowData::null()
        }
    }
}

impl Mesh for SkeletalMeshHandle {
    fn create_pipeline<'a>(
        &'a self,
        vgpu: &VirtualGpu
    ) -> PipelineBuilder<'a> {
        Pipeline::builder("Skeletal Vertex Shader")
            .source(
                ShaderType::Vertex, 
                ShaderSource {
                    source: skeletal_shaders::SHADER_skeletal_vertex_main.into(),
                    main_function: "skeletal_vertex_main".into()
                }
            )
            .vertex(vertex_buffer_layout())
            .vertex(instance_buffer_layout())
            .layout_raw::<gearbox::shaders::common::CameraInput>(2, gearbox::shaders::common::CameraInput::layout(vgpu, ShaderStages::VERTEX_FRAGMENT))
            .layout_raw::<skeletal_shaders::AnimationInfoInput>(3, skeletal_shaders::AnimationInfoInput::layout(vgpu, ShaderStages::VERTEX_FRAGMENT))
    }

    fn draw(
        &self,
        vgpu: &VirtualGpu,
        pass: &mut SinglePass,
        world: &World,
        entity: &Entity
    ) -> anyhow::Result<()> {
        let skvault = world
            .get_resource_ref::<SkeletalMeshVault>()
            .context("Could not find skeletal mesh vault")?;

        let mesh = skvault.get(self)
            .context("Failed to get mesh")?;

        let Some(vault) = world.get_resource_ref::<MeshAssetVault>()
            else { bail!("Missing mesh vault") };

        // extract transform and mesh components
        let (mut comps, _ctx) = extract_comps_distributed(
            entity, 
            &[Transform::bit_mask(), Animator::bit_mask()], 
            None
        );
        let transform: RefCastGuard<_, Transform> = comps.next().flatten()
            .expect("SkeletalMesh requires Transform companion component").lock_cast_ref();
        let mut animator: MutCastGuard<_, Animator> = comps.next().flatten()
            .expect("SkeletalMesh requires Animator companion component").lock_cast_mut();

        // create instance matrix to draw
        let instances = [
            Mat4::from_scale_rotation_translation(
                transform.scale, 
                transform.rotation, 
                transform.translation
            )
        ];

        // create or update instance buffer
        if self.instance_buffer.is_null() {
            self.instance_buffer.set(MutableBuffer::new(
                vgpu, 
                &instances, 
                wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST
            ));
        } else {
            self.instance_buffer.get_ref().write(vgpu, &instances)
                .expect("Failed to update instance buffer");
        }

        // setup instances buffer (same for all bones, animations are handled through bones_bindable)
        pass.bind_instances(&*self.instance_buffer.get_ref());

        // render each bone
        for bone in &mesh.bones {
            // get node transforms from the animator
            let node_transforms = animator.animate(bone);

            // generate skin matrices (world * inverse bind), used by actual skinned vertices
            let bones = if let (Some(skin), Some(nodes)) = (mesh.skin.as_ref(), node_transforms.as_ref()) {
                let mut bones = skin.iter()
                    .map(|(idx, ibp)| nodes[*idx as usize] * ibp)
                    .map(|a| a.into())
                    .collect::<Vec<magician_vgpu::rust::Mat4>>();
                bones.resize(32, Mat4::IDENTITY.into());
                bones
            } else {
                vec![Mat4::IDENTITY.into(); 32]
            };

            // generate plain per-node world matrices, used by rigid (non-skinned)
            // attachments (hats, weapons, shields, etc) parented to a bone
            let node_mats = if let Some(nodes) = node_transforms.as_ref() {
                let mut node_mats = nodes.iter()
                    .map(|m| (*m).into())
                    .collect::<Vec<magician_vgpu::rust::Mat4>>();
                node_mats.resize(32, Mat4::IDENTITY.into());
                node_mats
            } else {
                vec![Mat4::IDENTITY.into(); 32]
            };

            let info = AnimationInfo {
                bones: bones.as_slice().try_into().unwrap(),
                nodes: node_mats.as_slice().try_into().unwrap()
            };

            if self.animation_buffers.is_null() {
                let buffer = MutableBuffer::new(
                    vgpu, &info, 
                    wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST
                );
                let bindable = BindableObject
                    ::<AnimationInfoInput>
                    ::from_inputs(vgpu, &buffer.buffer());
                self.animation_buffers.set(SkeletalAnimationBuffers { buffer, bindable });
            } else {
                self.animation_buffers.get_ref().buffer.write(vgpu, &info)
                    .expect("Failed to update animation buffers");
            }

            pass.bind_raw(3, &self.animation_buffers.get_ref().bindable.bind_group());


            recr_bone(
                vgpu,
                &vault,
                pass,
                world,
                entity,
                &mesh,
                bone
            )?;
        }

        Ok(())
    }
}

fn recr_bone(
    vgpu: &VirtualGpu,
    vault: &MeshAssetVault,
    pass: &mut SinglePass,
    world: &World,
    entity: &Entity,
    mesh: &SkeletalMesh,
    bone: &ModelBone,
) -> anyhow::Result<()> {
    // attempt to find bone mesh to draw
    let bone_mesh = bone.mesh.map(|a| mesh.meshes.get(&a)).flatten();
    if let Some(bone_mesh) = bone_mesh {
        if bone_mesh.visible {
            // draw bone specific mesh
            if let Some(bone_mesh) = vault.get(&bone_mesh.mesh) {
                bone_mesh.draw(vgpu, pass, world, entity)?;
            }
        }
    }
    
    // draw children bones
    for child in &bone.children {
        recr_bone(vgpu, vault, pass, world, entity, mesh, child)?;
    }

    Ok(())
}

