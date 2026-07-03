use bytemuck::{Pod, Zeroable};
use magician_macros::{ShaderGroup, ShaderLayout, shader};
use magician_rust::{Mat4, UVec4, Vec2, Vec3, Vec4, length_vec4};
use magician_vgpu::macros::{BindableObject, UniformBufferObject};

#[repr(C)]
#[derive(Pod, Zeroable, Clone, Copy, ShaderLayout)]
pub struct VertexInput {
    #[location = 0] pub position: Vec3,
    #[location = 1] pub uvs: Vec2,
    #[location = 2] pub normal: Vec3,
    #[location = 3] pub weights: Vec4,
    #[location = 4] pub joints: UVec4
}

#[repr(C)]
#[derive(Pod, Zeroable, Clone, Copy, ShaderLayout)]
pub struct InstanceInput {
    #[location = 5] pub mm0: Vec4,
    #[location = 6] pub mm1: Vec4,
    #[location = 7] pub mm2: Vec4,
    #[location = 8] pub mm3: Vec4
}

#[derive(ShaderLayout)]
pub struct VertexOutput {
    #[builtin = "position"] pub clip_position: Vec4,
    #[location = 0] pub uvs: Vec2,
    #[location = 1] pub color: Vec4,
    #[location = 2] pub world_normal: Vec3,
    #[location = 3] pub world_position: Vec3
}

#[repr(C)]
#[derive(Pod, Zeroable, Clone, Copy, UniformBufferObject)]
pub struct Camera {
    pub view_pos: Vec4,
    pub view_proj: Mat4
}

#[derive(ShaderGroup, BindableObject)]
pub struct CameraInput {
    #[uniform] pub camera: Camera
}


#[derive(ShaderGroup, BindableObject)]
pub struct AnimationInfoInput {
    #[uniform] pub info: AnimationInfo
}

#[repr(C)]
#[derive(Pod, Zeroable, Clone, Copy, UniformBufferObject)]
pub struct AnimationInfo {
    pub bones: [Mat4; 32]
}

#[shader("./shader_out", vertex)]
pub fn skeletal_vertex_main(
    #[group = 2] anim_info: AnimationInfoInput,
    #[group = 1] cam_in: CameraInput,
    model: VertexInput,
    instance: InstanceInput
) -> VertexOutput {
    let mut mm = Mat4::new(instance.mm0, instance.mm1, instance.mm2, instance.mm3);

    if length_vec4(model.weights) > 0.0 {
        // mm = (model.weights.x() * anim_info.info.bones[model.joints.x])
        //     + (model.weights.y() * anim_info.info.bones[model.joints.y])
        //     + (model.weights.z() * anim_info.info.bones[model.joints.z])
        //     + (model.weights.w() * anim_info.info.bones[model.joints.w]);
        // let bone_mat = 
        //     (anim_info.info.bones[model.joints.x() as usize] * model.weights.x()) + 
        //     (anim_info.info.bones[model.joints.y() as usize] * model.weights.y()) +
        //     (anim_info.info.bones[model.joints.z() as usize] * model.weights.z()) +
        //     (anim_info.info.bones[model.joints.w() as usize] * model.weights.w());
        let bone_mat = 
            (model.weights.x() * anim_info.info.bones[model.joints.x() as usize]) + 
            (model.weights.y() * anim_info.info.bones[model.joints.y() as usize]) + 
            (model.weights.z() * anim_info.info.bones[model.joints.z() as usize]) + 
            (model.weights.w() * anim_info.info.bones[model.joints.w() as usize]);
        mm = mm * bone_mat;
    } else {
        mm = mm * anim_info.info.bones[model.joints.x() as usize];
    }

    let world_position = mm * Vec4::from_vec3_w(model.position, 1.0);

    return VertexOutput {
        clip_position: cam_in.camera.view_proj * world_position, 
        uvs: model.uvs, 
        color: Vec4::new(1.0, 1.0, 1.0, 1.0), 
        world_normal: model.normal, 
        world_position: world_position.xyz()
    };
}
