use ahash::AHashMap;
use anarchy::{ComponentMeta, extract_comps_distributed};
use chrono::{DateTime, Utc};
use gearbox::{Mesh, Transform};
use magician_vgpu::{BindGroupProvider, BindableObject, Buffer, DrawSettings, ImmutableBuffer, MutableBuffer, Pipeline, PipelineBuilder, ShaderSource, ShaderType, SinglePass, VirtualGpu, WritableBuffer, glam::*};
use mutual::{CastableSharedData, CowData, RefCastGuard};
use skeletal_shaders::{AnimationInfo, AnimationInfoInput};
use wgpu::ShaderStages;

use crate::{anim::animate_matrices, data::*};

pub type SkeletalMeshVertex = skeletal_shaders::VertexInput;

pub struct SkeletalMesh {
    pub bones: Vec<ModelBone>,
    pub animations: AHashMap<String, Animation>,
    pub skin: Option<Vec<(u16, Mat4)>>,
    pub meshes: AHashMap<usize, SkeletalSubMesh>,
    pub material: Option<SkeletalMaterial>,
    pub node_id_map: AHashMap<String, usize>,
    pub instance_buffer: CowData<MutableBuffer<[Mat4]>>,
    pub animation_buffers: CowData<SkeletalAnimationBuffers>,
    pub anim_start_time: DateTime<Utc>
}

pub struct SkeletalSubMesh {
    pub vertices: ImmutableBuffer<[SkeletalMeshVertex]>,
    pub indices: ImmutableBuffer<[u32]>
}

pub struct SkeletalAnimationBuffers {
    pub buffer: MutableBuffer<AnimationInfo>,
    pub bindable: BindableObject<skeletal_shaders::AnimationInfoInput>
}

#[allow(unused)]
impl Mesh for SkeletalMesh {
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
            .layout_raw::<skeletal_shaders::AnimationInfoInput>(skeletal_shaders::AnimationInfoInput::layout(vgpu, ShaderStages::VERTEX_FRAGMENT))
    }

    fn draw<'a>(
        &'a self,
        vgpu: &VirtualGpu,
        pass: &mut SinglePass<'a>, 
        entity: &'a anarchy::Entity
    ) {
        // extract transform and mesh components
        let (mut comps, _ctx) = extract_comps_distributed(
            entity, 
            &[Transform::bit_mask()], 
            None
        );
        let transform: RefCastGuard<_, Transform> = comps.next().flatten()
            .expect("BasicMaterial requires Transform companion component").lock_cast_ref();

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
        for bone in &self.bones {
            let animation = self.animations.get("Walking_A").expect("failed to find test animation");
            let seconds_since_start = Utc::now().signed_duration_since(self.anim_start_time).as_seconds_f32();
            let time = seconds_since_start % animation.length;

            // animate nodes
            let mut nodes = Vec::new();
            animate_matrices(&mut nodes, bone, animation, &Mat4::IDENTITY, time);

            // generate bones
            // let mut bones = nodes.iter().map(|a| (*a).into()).collect::<Vec<magician_vgpu::rust::Mat4>>();
            let bones = if let Some(skin) = self.skin.as_ref() {
                let mut bones = skin.iter()
                    .map(|(idx, ibp)| nodes[*idx as usize] * ibp)
                    .map(|a| a.into())
                    .collect::<Vec<magician_vgpu::rust::Mat4>>();
                bones.resize(32, Mat4::IDENTITY.into());
                bones
            } else {
                vec![Mat4::IDENTITY.into(); 32]
            };
            let info = AnimationInfo { bones: bones.as_slice().try_into().unwrap() };

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

            pass.bind_raw::<skeletal_shaders::AnimationInfoInput>(2, &self.animation_buffers.get_ref().bindable.bind_group());


            recr_bone(
                vgpu,
                pass,
                self,
                bone
            );
        }
    }
}

fn recr_bone(
    vgpu: &VirtualGpu,
    pass: &mut SinglePass,
    mesh: &SkeletalMesh,
    bone: &ModelBone
) {
    // attempt to find bone mesh to draw
    let bone_mesh = bone.mesh.map(|a| mesh.meshes.get(&a)).flatten();
    if let Some(bone_mesh) = bone_mesh {
        // draw bone specific mesh
        pass.draw(
            &bone_mesh.vertices,
            &bone_mesh.indices,
            DrawSettings::default()
        )
    }
    
    // draw children bones
    for child in &bone.children {
        recr_bone(vgpu, pass, mesh, child);
    }
}
