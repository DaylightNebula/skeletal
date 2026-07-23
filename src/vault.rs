use std::{hash::{Hash, Hasher}, path::PathBuf, sync::Arc};

use ahash::AHasher;
use anarchy::{Res, Scheduler, anyhow, macros::{Resource, system}};
use cell::{App, Graphics, Plugin};
use derive_more::{Deref, DerefMut};
use gltf::Gltf;
use mutual::{CowData, DashMap, RefCowData};
use gearbox::{AssetContent, AssetVault, Handle, MeshAssetVault};

use crate::{SkeletalMesh, SkeletalMeshHandle, loader};

pub struct SkeletalMeshVaultPlugin;
impl Plugin for SkeletalMeshVaultPlugin {
    fn build(self, app: App) -> App {
        app.add_resource(SkeletalMeshVault::default())
            .on_render_update(load_inprogress)
    }
}

#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub enum SkeletalMeshLoadType {
    GLTF,
    FBX
}

#[derive(Resource, Default, Deref, DerefMut)]
pub struct SkeletalMeshVault(Arc<SkeletalMeshVaultInner>);

#[derive(Default)]
pub struct SkeletalMeshVaultInner {
    pub mesh: DashMap<u64, (SkeletalMeshHandle, CowData<SkeletalMesh>)>,
    pub preload: DashMap<u64, SkeletalMeshHandle>,
    pub inprogress_gltf: DashMap<u64, (SkeletalMeshHandle, Gltf)>,
    pub inprogress_fbx: DashMap<u64, (SkeletalMeshHandle, ufbx::SceneRoot)>
}

unsafe impl Send for SkeletalMeshVaultInner {}
unsafe impl Sync for SkeletalMeshVaultInner {}

impl SkeletalMeshVault {
    pub fn new() -> Self { Self::default() }

    pub fn has(&self, handle: &SkeletalMeshHandle) -> bool { self.mesh.contains_key(&handle.handle().inner().0) }

    pub fn get_handle(&self, hash: u64) -> Option<SkeletalMeshHandle> {
        self.mesh.get(&hash)
            .map(|a| a.0.clone())
    }

    pub fn load_raw(&self, hash: u64, asset: SkeletalMesh) -> SkeletalMeshHandle {
        let handle = Handle::new((hash, Arc::clone(&self.0)));
        let handle = SkeletalMeshHandle::new(handle);
        self.mesh.insert(hash, (handle.clone(), CowData::new(asset)));
        return handle;
    }
}

impl SkeletalMeshVaultInner {
    pub fn remove(&self, hash: u64) -> Option<(u64, (SkeletalMeshHandle, CowData<SkeletalMesh>))> {
        self.mesh.remove(&hash)
    }
}

impl AssetVault for SkeletalMeshVault {
    type Asset = SkeletalMesh;
    type LoadType = SkeletalMeshLoadType;
    type LoadResult = SkeletalMeshHandle;
    type Lookup = SkeletalMeshHandle;
    type LookupResult = RefCowData<SkeletalMesh>;

    fn get(&self, handle: &Self::Lookup) -> Option<Self::LookupResult> {
        self.mesh.get(&handle.handle().inner().0).map(|a| a.1.get_ref())
    }

    fn load(&self, content: AssetContent, ty: SkeletalMeshLoadType) -> anarchy::anyhow::Result<Self::LoadResult> {
        // get content hash
        let mut hasher = AHasher::default();
        content.hash(&mut hasher);
        let hash = hasher.finish();

        // attempt to find previous handle with the same hash and return that
        if let Some(handle) = self.mesh.get(&hash) { return Ok(handle.0.clone()); }
        if let Some(handle) = self.inprogress_gltf.get(&hash) { return Ok(handle.0.clone()); }
        if let Some(handle) = self.inprogress_fbx.get(&hash) { return Ok(handle.0.clone()); }
        if let Some(handle) = self.preload.get(&hash) { return Ok(handle.clone()); }

        // create new handle and store inprogress
        let handle = Handle::new((hash, Arc::clone(&self.0)));
        let handle = SkeletalMeshHandle::new(handle);
        self.preload.insert(hash, handle.clone());

        // start load
        let inner = Arc::clone(&self.0);
        let handle2 = handle.clone();
        Scheduler::run_async(async move {
            let bytes = content.into_bytes()
                .await
                .expect("Failed to read skeletal mesh content");
            
            match ty {
                SkeletalMeshLoadType::GLTF => {
                    let gltf = Gltf::from_slice(&bytes)
                        .expect("Failed to read gltf from bytes");
                    inner.inprogress_gltf.insert(hash, (handle2, gltf));
                },
                SkeletalMeshLoadType::FBX => {
                    let fbx = ufbx::load_memory(&bytes, loader::fbx::load_opts())
                        .map_err(|e| anyhow::anyhow!("Failed to load fbx: {}", e.description))
                        .expect("Failed to load FBX");
                    inner.inprogress_fbx.insert(hash, (handle2, fbx));
                },
            }

            inner.preload.remove(&hash);
        });

        Ok(handle)
    }
}

#[system(std::i32::MIN)]
pub fn load_inprogress(
    graphics: Res<Graphics>,
    vault: Res<SkeletalMeshVault>,
    meshes: Res<MeshAssetVault>
) {
    // take copy of all hashes in the inprogress maps
    let inprogress_gltf_hashes = vault.inprogress_gltf.iter()
        .map(|a| *a.key())
        .collect::<Vec<_>>();
    let inprogress_fbx_hashes = vault.inprogress_fbx.iter()
        .map(|a| *a.key())
        .collect::<Vec<_>>();


    for hash in inprogress_gltf_hashes.into_iter() {
        {
            let Some(content) = vault.inprogress_gltf.get(&hash)
                else { continue };
            let handle = content.0.clone();
            let gltf = &content.1;
            let (mesh, _animations) = loader::gltf::load(gltf, &graphics, &meshes, &PathBuf::new(), &PathBuf::new(), None, hash);
            vault.mesh.insert(hash, (handle, CowData::new(mesh)));
        }
        
        vault.inprogress_gltf.remove(&hash);
    }


    for hash in inprogress_fbx_hashes.into_iter() {
        {
            let Some(content) = vault.inprogress_fbx.get(&hash)
                else { continue };
            let handle = content.0.clone();
            let fbx = &content.1;
            let (mesh, _animations) = loader::fbx::load(&graphics, &fbx, &meshes, None, None, hash);
            vault.mesh.insert(hash, (handle, CowData::new(mesh)));
        }
        
        vault.inprogress_fbx.remove(&hash);
    }
}
