#![cfg_attr(target_arch = "spirv", no_std)]

use spirv_std::glam::{vec4, Vec4, Mat4, Vec3};
use spirv_std::spirv;

#[spirv(fragment)]
pub fn fragment(output: &mut Vec4) {
    *output = vec4(1.0,0.0,0.0,1.0);
}


pub struct Matrices {
    camera_pos: Vec3,
    view: Mat4,
    projection: Mat4
}
pub struct Inputs {
    model: Mat4,
    time: f32
}
pub struct VertexOut {
    normal: Vec3,
    frag_pos: Vec3
}


#[spirv(vertex)]
pub fn vertex(
    #[spirv(in)] a_pos: Vec3,
    #[spirv(layout)] a_normal: Vec3,
    #[spirv(uniform, binding=0)] global_matrix: &Matrices,
    #[spirv(uniform, binding=1)] inputs: &Inputs,
    #[spirv(position, invariant)] out_pos: &mut Vec4,
    mut out: VertexOut,
){
    out.frag_pos = (inputs.model * a_pos.extend(1.0)).truncate();
    *out_pos = vec4(
        (a_pos.x) as f32,
        global_matrix.camera_pos.y,
        inputs.time,
        1.0
    );
}
