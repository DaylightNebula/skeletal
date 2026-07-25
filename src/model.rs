use std::sync::Arc;

use ahash::AHashMap;
use anarchy::macros::{AsAny, Getters, GettersMut};
use gearbox::{Asset, Handle, MeshAsset, SimpleTexturedMaterial, glam::Mat4};
use magician_vgpu::{BindableObject, MutableBuffer};
use skeletal_shaders::AnimationInfo;

use crate::{SkeletalMeshVault, SkeletalMeshVaultInner, data::*};

pub type SkeletalMeshVertex = skeletal_shaders::VertexInput;

/// A loaded skeleton and its render data: the bone/node hierarchy (`bones`),
/// the skin's per-joint inverse bind matrices (`skin`, joint index paired with
/// its inverse bind pose, absent if the model isn't skinned), the sub-meshes
/// attached to bones (`meshes`, keyed by the source format's mesh index), the
/// model's material, and a lookup from authored node name to node id
/// (`node_id_map`, used to resolve `Animator` channels onto this skeleton's ids).
#[derive(Getters, GettersMut, AsAny)]
pub struct SkeletalMesh {
    pub(crate) bones: Vec<ModelBone>,
    pub(crate) skin: Option<Vec<(u16, Mat4)>>,
    pub(crate) meshes: AHashMap<usize, SkeletalSubMesh>,
    pub(crate) material: Option<SimpleTexturedMaterial>,
    pub(crate) node_id_map: AHashMap<String, usize>
}

/// A single renderable mesh attached to a `ModelBone`, along with the bone's
/// authored name (`label`), used to toggle per-bone mesh visibility.
pub struct SkeletalSubMesh {
    pub mesh: Handle<MeshAsset>,
    pub label: String
}

/// The GPU-side uniform buffer and bind group holding the current frame's
/// `AnimationInfo` (skin + node matrices) for a `SkeletalMeshHandle`, lazily
/// created on first draw and rewritten every subsequent frame.
pub struct SkeletalAnimationBuffers {
    pub buffer: MutableBuffer<AnimationInfo>,
    pub bindable: BindableObject<skeletal_shaders::AnimationInfoInput>
}

impl Asset for SkeletalMesh {
    type Vault = SkeletalMeshVault;
    type HandleTracker = (u64, Arc<SkeletalMeshVaultInner>);

    // Threshold of 2 accounts for the `SkeletalMeshHandle` clone held inside the
    // vault's own `mesh` map alongside every handle given out to callers.
    fn unload_threshold() -> usize { 2 }
    fn unload(tracker: &Self::HandleTracker) {
        tracker.1.remove(tracker.0);
    }
}
