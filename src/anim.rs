use std::f32::consts::PI;

use gearbox::Transform;
use magician_vgpu::glam::{Mat4, Quat, Vec3};

use crate::data::{Animation, Interpolation, ModelBone};

/// For each node in the node tree, its global matrix is added to the node_matrices list
pub fn animate_matrices(
    node_matrices: &mut Vec<Mat4>,
    node: &ModelBone,
    animation: &Animation,
    parent: &Mat4,
    time: f32
) {
    // calculate animated matrix
    let animated_matrix = &animation.channels
        .get(&(node.id as usize)).map(|channel| {
            let translation = channel.positions
                .as_ref()
                .map(|(a, b, c)| compute_vector_interpolation(a, &b, &c, time))
                .unwrap_or(Vec3::ZERO);
            let rotation = channel.rotations
                .as_ref()
                .map(|(a, b, c)| compute_rotation_interpolation(a, &b, &c, time))
                .unwrap_or(Quat::IDENTITY);
            let scale = channel.scale
                .as_ref()
                .map(|(a, b, c)| compute_vector_interpolation(a, &b, &c, time))
                .unwrap_or(Vec3::ONE);

            Transform::new(translation, rotation, scale)
        }).unwrap_or(Transform::default());
    
    // calculate final matrix
    let matrix = parent * animated_matrix.as_matix();

    // animate children
    for node in &node.children {
        animate_matrices(node_matrices, node, animation, &matrix, time);
    }

    // save bone
    if node.id as usize >= node_matrices.len() {
        node_matrices.resize(node.id as usize + 1, Mat4::IDENTITY);
    }
    node_matrices[node.id as usize] = matrix;
}

/// Gets the before and after frame using the timing array
/// The frames are selected at the same indices that the given time value is between in the timing array
/// 
/// Returns (the before frame, the after frame, the time in the frame as a percentage)
fn get_timed_first_last<'a, T>(
    timing: &[f32],
    values: &'a [T],
    time: f32
) -> (&'a T, &'a T, f32) {
    // restrict timing to in range
    let time = time % timing.last().unwrap_or(&0.0);

    // get indices
    let mut last_idx = 0;
    while timing[last_idx] <= time { last_idx += 1; }
    let first_idx = last_idx.checked_sub(1).unwrap_or(0);

    // compile results
    (&values[first_idx], &values[last_idx], (time - timing[first_idx]) / (timing[last_idx] - timing[first_idx]))
}

/// Computes the animated interpolation of a list of vectors
fn compute_vector_interpolation(
    interpolation: &Interpolation,
    timing: &[f32],
    vectors: &[Vec3],
    time: f32
) -> Vec3 {
    let (first, last, time) = get_timed_first_last(timing, vectors, time);

    // return interpolated vectors
    return match interpolation {
        Interpolation::Linear => (*first * (1.0 - time)) + (*last * time),
        Interpolation::Wave => {
            let time = ((time - 0.5) * PI).sin() * 0.5 + 0.5;
            (*first * (1.0 - time)) + (*last * time)
        },
        Interpolation::Step => *first
    }
}

/// Computes the animated interpolation of a list of quaternion
fn compute_rotation_interpolation(
    interpolation: &Interpolation,
    timing: &[f32],
    rotations: &[Quat],
    time: f32
) -> Quat {
    let (first, last, time) = get_timed_first_last(timing, rotations, time);

    // return interpolated rotations
    // https://registry.khronos.org/glTF/specs/2.0/glTF-2.0.html#appendix-c-interpolation
    return match interpolation {
        Interpolation::Linear => first.slerp(*last, time),
        Interpolation::Wave => {
            // calculate time values then slerp the final rotation
            let time = ((time - 0.5) * PI).sin() * 0.5 + 0.5;
            first.slerp(*last, time)
        },
        Interpolation::Step => *first
    };
}
