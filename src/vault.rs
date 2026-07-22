use std::{hash::{Hash, Hasher}, sync::Arc};

use ahash::AHasher;
use anarchy::{Scheduler, macros::Resource};
use cell::{App, Plugin};
use derive_more::{Deref, DerefMut};
use gltf::Gltf;
use mutual::{CowData, DashMap, RefCowData};
use gearbox::{Asset, AssetContent, AssetVault, Handle, Mesh};

use crate::{SkeletalMesh, loader};

#[derive(Resource, Default, Deref, DerefMut)]
pub struct SkeletalMeshVault(Arc<SkeletalMeshVaultInner>);

#[derive(Default)]
pub struct SkeletalMeshVaultInner {
    pub mesh: DashMap<u64, (Handle<SkeletalMesh>, CowData<SkeletalMesh>)>,
    pub inprogress: DashMap<u64, Handle<SkeletalMesh>>
}

impl SkeletalMeshVault {
    pub fn new() -> Self { Self::default() }

    pub fn has(&self, handle: &Handle<SkeletalMesh>) -> bool { self.mesh.contains_key(&handle.inner().0) }

    pub fn get_handle(&self, hash: u64) -> Option<Handle<SkeletalMesh>> {
        self.mesh.get(&hash)
            .map(|a| a.0.clone())
    }

    pub fn load_raw(&self, hash: u64, asset: SkeletalMesh) -> Handle<SkeletalMesh> {
        let handle = Handle::new((hash, Arc::clone(&self.0)));
        self.mesh.insert(hash, (handle.clone(), CowData::new(asset)));
        return handle;
    }
}

impl SkeletalMeshVaultInner {
    pub fn remove(&self, hash: u64) -> Option<(u64, (Handle<SkeletalMesh>, CowData<SkeletalMesh>))> {
        self.mesh.remove(&hash)
    }
}

impl AssetVault for SkeletalMeshVault {
    type Asset = SkeletalMesh;
    type LoadResult = Handle<SkeletalMesh>;
    type Lookup = Handle<SkeletalMesh>;
    type LookupResult = RefCowData<SkeletalMesh>;

    fn get(&self, handle: &Self::Lookup) -> Option<Self::LookupResult> {
        self.mesh.get(&handle.inner().0).map(|a| a.1.get_ref())
    }

    fn load(&self, content: AssetContent) -> anarchy::anyhow::Result<Self::LoadResult> {
        // get content hash
        let mut hasher = AHasher::default();
        content.hash(&mut hasher);
        let hash = hasher.finish();

        // attempt to find previous handle with the same hash and return that
        if let Some(handle) = self.mesh.get(&hash) { return Ok(handle.0.clone()); }
        if let Some(handle) = self.inprogress.get(&hash) { return Ok(handle.clone()); }

        // create new handle and store inprogress
        let handle = Handle::new((hash, Arc::clone(&self.0)));
        self.inprogress.insert(hash, handle.clone());

        // start load
        Scheduler::run_async(async move {
            let bytes = content.into_bytes()
                .await
                .expect("Failed to read skeletal mesh content");
            
            let gltf = Gltf::from_slice(&bytes)
                .expect("Failed to read gltf from bytes");

            todo!("Finishing loading GLTF")

            // let mesh_vault = world.get_resource_ref();

            // loader::gltf::load(gltf, mesh_vault, asset_file, source_file, extra_buffer);
        });

        Ok(handle)
    }
}
