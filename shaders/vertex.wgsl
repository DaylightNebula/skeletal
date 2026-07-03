@public
struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) uvs: vec2<f32>,
    @location(2) normal: vec3<f32>,
    @location(3) weights: vec4<f32>,
    @location(4) joints: vec4<u32>
};

@public
struct InstanceInput {
    @location(5) model_matrix_0: vec4<f32>,
    @location(6) model_matrix_1: vec4<f32>,
    @location(7) model_matrix_2: vec4<f32>,
    @location(8) model_matrix_3: vec4<f32>,
    @location(9) mat_id: u32
};

@public
struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) uvs: vec2<f32>,
    @location(1) color: vec4<f32>,
    @location(2) world_normal: vec3<f32>,
    @location(3) world_position: vec3<f32>,
    @location(4) mat_id: u32
};

@public
@default(VertexOutput)
fn def_vertex_output() -> VertexOutput {
    var out: VertexOutput;
    out.world_normal = vec3<f32>(0.0, 1.0, 0.0);
    return out;
}

@group(3) @binding(0) var<uniform> bones: array<mat4x4<f32>, 32u>;
@group(3) @binding(1) var<uniform> nodes: array<mat4x4<f32>, 32u>;
@group(0) @binding(0) var<uniform> camera_vp: mat4x4<f32>;

@main
fn skeletal_vertex_main(
    model: VertexInput,
    instance: InstanceInput,
    @mut out: VertexOutput
) -> VertexOutput {
    // the models position, rotation, scale
    var model_matrix = mat4x4<f32>(
        instance.model_matrix_0,
        instance.model_matrix_1,
        instance.model_matrix_2,
        instance.model_matrix_3,
    );

    if length(model.weights) > 0.0 {
        model_matrix *= (model.weights.x * bones[model.joints.x])
            + (model.weights.y * bones[model.joints.y])
            + (model.weights.z * bones[model.joints.z])
            + (model.weights.w * bones[model.joints.w]);
    } else {
        model_matrix *= bones[model.joints.x];
    }

    // setup color and tex coord output
    out.uvs = model.uvs;
    out.color = vec4<f32>(1.0, 1.0, 1.0, 1.0);

    out.world_normal = model.normal;
    var world_position = model_matrix * vec4<f32>(model.position, 1.0);
    out.world_position = world_position.xyz;
    out.clip_position = camera_vp * world_position;
    out.mat_id = instance.mat_id;
    return out;
}
