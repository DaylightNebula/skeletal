//! Format-specific loaders that each turn a parsed scene (glTF or FBX) into the
//! same engine-native `SkeletalMesh` + animation data, so callers don't need to
//! care which format a model came from.

pub mod fbx;
pub mod gltf;
