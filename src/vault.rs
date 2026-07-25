use std::{hash::{Hash, Hasher}, path::PathBuf, sync::Arc};

use ahash::AHasher;
use anarchy::{Res, Scheduler, anyhow, macros::{Resource, system}};
use cell::{App, Graphics, Plugin};
use derive_more::{Deref, DerefMut};
use gltf::Gltf;
use mutual::{CowData, DashMap, RefCowData};
use gearbox::{AssetContent, AssetVault, BasicMaterial, BindlessArrayTextureVault, Handle, HotSwapMaterial, MeshAssetVault, glam::Vec4};

use crate::{SkeletalMesh, SkeletalMeshHandle, loader};

/// Registers the `SkeletalMeshVault` resource and the render-schedule system
/// that finishes loading meshes queued by it. Required for `SkeletalMeshVault::load`
/// to ever resolve a queued glTF/FBX file into a usable `SkeletalMesh`.
pub struct SkeletalMeshVaultPlugin;
impl Plugin for SkeletalMeshVaultPlugin {
    fn build(self, app: App) -> App {
        app.add_resource(SkeletalMeshVault::default())
            .on_render_update(load_inprogress)
    }
}

/// The source file format to parse a `SkeletalMeshVault::load` call's content as.
#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub enum SkeletalMeshLoadType {
    GLTF,
    FBX
}

/// Resource that loads and caches `SkeletalMesh`es, deduplicated by content hash.
/// A cheap `Arc`-backed handle to `SkeletalMeshVaultInner`; cloning it shares the
/// same underlying cache.
#[derive(Resource, Default, Deref, DerefMut)]
pub struct SkeletalMeshVault(Arc<SkeletalMeshVaultInner>);

/// Tracks a mesh through its load pipeline, one map per stage: `preload` holds
/// a handle while its file bytes are being read asynchronously; once read, the
/// bytes are parsed (off the render thread) into `inprogress_gltf` or
/// `inprogress_fbx`; `load_inprogress` then converts that parsed scene into GPU
/// resources on the render thread and moves the result into `mesh`, the final
/// cache queried by `get`/`has`. A hash only ever lives in one map at a time.
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

    /// Whether a fully-loaded mesh exists in the cache for this handle.
    pub fn has(&self, handle: &SkeletalMeshHandle) -> bool { self.mesh.contains_key(&handle.handle().inner().0) }

    /// Look up an already-cached handle by content hash, if a mesh has finished loading for it.
    pub fn get_handle(&self, hash: u64) -> Option<SkeletalMeshHandle> {
        self.mesh.get(&hash)
            .map(|a| a.0.clone())
    }

    /// Insert an already-constructed `SkeletalMesh` directly into the cache under
    /// `hash`, skipping the async file-load pipeline, and return its handle.
    pub fn load_raw(&self, hash: u64, asset: SkeletalMesh) -> SkeletalMeshHandle {
        let handle = Handle::new((hash, Arc::clone(&self.0)));
        // let material: HotSwapMaterial = CowData::new(Box::new(asset.material.unwrap_or(|| BasicMaterial::new(Vec4::ONE))));
        let material: HotSwapMaterial = 
            if let Some(mat) = &asset.material { CowData::new(Box::new(mat.clone())) }
            else { CowData::new(Box::new(BasicMaterial::new(Vec4::ONE))) };
        let handle = SkeletalMeshHandle::new(handle, material);
        self.mesh.insert(hash, (handle.clone(), CowData::new(asset)));
        return handle;
    }
}

impl SkeletalMeshVaultInner {
    /// Drop a fully-loaded mesh from the cache. Called by `SkeletalMesh::unload`
    /// once its handle's reference count drops to the unload threshold.
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

    /// Queue `content` for loading as `ty` and return a handle immediately.
    /// Hashes `content` first and, if a handle already exists for that hash
    /// (loaded, or anywhere in the load pipeline), returns the existing handle
    /// instead of loading a duplicate. Otherwise spawns an async task that
    /// reads the content's bytes and parses them into a `Gltf`/`ufbx::SceneRoot`,
    /// stashing the result in `inprogress_gltf`/`inprogress_fbx` for the
    /// `load_inprogress` render-schedule system to finish (uploading GPU
    /// resources requires the render thread, so that part can't happen here).
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
        let material: HotSwapMaterial = CowData::new(Box::new(BasicMaterial::new(Vec4::ONE)));
        let handle = SkeletalMeshHandle::new(handle, material);
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

/// Finishes loading every mesh currently sitting in `inprogress_gltf`/`inprogress_fbx`:
/// turns each parsed scene into GPU resources (requires the render thread, hence
/// this being a render-schedule system) and moves the result into the vault's
/// `mesh` cache. Runs at `i32::MIN`, i.e. after every other render-schedule
/// system this frame, so a mesh finished here becomes visible to the rest of
/// the render schedule starting next frame.
#[system(std::i32::MIN)]
pub fn load_inprogress(
    graphics: Res<Graphics>,
    vault: Res<SkeletalMeshVault>,
    meshes: Res<MeshAssetVault>,
    textures: Res<BindlessArrayTextureVault>
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
            let (mesh, _animations) = loader::gltf::load(gltf, &graphics, &meshes, &textures, &PathBuf::new(), &PathBuf::new(), None, hash);
            if let Some(material) = mesh.material.as_ref() {
                handle.material().set(Box::new(material.clone()));
            }
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
            let (mesh, _animations) = loader::fbx::load(&graphics, &fbx, &meshes, &textures, None, None, hash);
            if let Some(material) = mesh.material.as_ref() {
                handle.material().set(Box::new(material.clone()));
            }
            vault.mesh.insert(hash, (handle, CowData::new(mesh)));
        }
        
        vault.inprogress_fbx.remove(&hash);
    }
}
