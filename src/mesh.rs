use anarchy::{Entity, World, macros::AsAny};
use gearbox::Mesh;
use magician_vgpu::{DrawSettings, ImmutableBuffer, PipelineBuilder, SinglePass, VirtualGpu};

use crate::SkeletalMeshVertex;

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
    ) {
        pass.draw(
            &self.vertices, 
            &self.indices, 
            DrawSettings::default()
        );
    }
}