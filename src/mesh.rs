use anarchy::{Entity, World, anyhow::{self, Context}, macros::AsAny};
use derive_more::{Deref, DerefMut};
use gearbox::{AssetVault, Handle, Mesh};
use magician_vgpu::{BindGroupProvider, DrawSettings, ImmutableBuffer, Pipeline, PipelineBuilder, ShaderSource, ShaderType, SinglePass, VirtualGpu};
use wgpu::ShaderStages;

use crate::{SkeletalMesh, SkeletalMeshVault, SkeletalMeshVertex, instance_buffer_layout, vertex_buffer_layout};

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

#[derive(AsAny, Deref, DerefMut, Debug, Clone)]
pub struct SkeletalMeshHandle(pub Handle<SkeletalMesh>);

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
        let vault = world
            .get_resource_ref::<SkeletalMeshVault>()
            .context("Could not find skeletal mesh vault")?;

        let mesh = vault.get(self)
            .context("Failed to get mesh")?;

        mesh.draw(vgpu, pass, world, entity)
    }
}
