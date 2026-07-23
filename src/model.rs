use std::sync::Arc;

use ahash::AHashMap;
use anarchy::macros::{AsAny, Getters, GettersMut};
use gearbox::{Asset, Handle, MeshAsset, glam::Mat4};
use magician_vgpu::{BindableObject, MutableBuffer};
use skeletal_shaders::AnimationInfo;

use crate::{SkeletalMeshVault, SkeletalMeshVaultInner, data::*};

pub type SkeletalMeshVertex = skeletal_shaders::VertexInput;

#[derive(Getters, GettersMut, AsAny)]
pub struct SkeletalMesh {
    pub(crate) bones: Vec<ModelBone>,
    pub(crate) skin: Option<Vec<(u16, Mat4)>>,
    pub(crate) meshes: AHashMap<usize, SkeletalSubMesh>,
    pub(crate) material: Option<SkeletalMaterial>,
    pub(crate) node_id_map: AHashMap<String, usize>
}

pub struct SkeletalSubMesh {
    pub mesh: Handle<MeshAsset>,
    pub label: String,
    pub visible: bool
}

pub struct SkeletalAnimationBuffers {
    pub buffer: MutableBuffer<AnimationInfo>,
    pub bindable: BindableObject<skeletal_shaders::AnimationInfoInput>
}

impl Asset for SkeletalMesh {
    type Vault = SkeletalMeshVault;
    type HandleTracker = (u64, Arc<SkeletalMeshVaultInner>);

    fn unload_threshold() -> usize { 2 }
    fn unload(tracker: &Self::HandleTracker) {
        tracker.1.remove(tracker.0);
    }
}
