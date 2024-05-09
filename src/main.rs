use std::os::raw::c_float;

use cgmath::Vector3;
use glfw::ffi::glfwGetTime;
use noise::{NoiseFn, Perlin};

use crate::engine::Data;
use crate::engine::renderable::Renderable;

mod engine;

fn main() {
    let mut engine = engine::Engine::new();
    let mut renderable = unsafe { Renderable::from_obj("objects/cube.obj", "shaders/pos_shader") };
    let grid_verts = create_grid(100, 100, 0.5);
    let mut grid = Renderable::new(grid_verts.0, grid_verts.1, grid_verts.2, unsafe { engine::renderable::Shader::load_from_path("shaders/pos_shader") });
    // Renderable::new(vertices, indices, vec![], unsafe {Shader::load_from_path("shaders/orientation_shader")}),
    renderable.uniform_scale(0.1);
    engine.add_renderable(renderable);
    engine.add_renderable(grid);
    engine.callback = callback;
    engine.run();
}

fn callback(data: &mut Data) {
    let offset = unsafe {glfwGetTime()} as f32;
    let perlin = Perlin::new(1);
    let renderable = data.get_renderable_mut(1);
    for i in 0..renderable.vertices.len() {
        let noise = perlin.get([((renderable.vertices[i].x + offset) / 5.) as f64, ((renderable.vertices[i].z + offset) / 5.) as f64, 0.]) as c_float;
        renderable.vertices[i].y = noise / 2.;
    }
    println!("{:?}", renderable.vertices[90]);
    unsafe { renderable.update_vertex_buffer() };
}

fn create_grid(width: u32, length: u32, scale: f32) -> (Vec<Vector3<f32>>, Vec<u32>, Vec<Vector3<f32>>) {
    let mut vertices = Vec::new();
    let mut indices = Vec::new();
    let mut normals = Vec::new();
    let mut offset = 0;

    for i in 0..width {
        for j in 0..length {
            vertices.push(Vector3::new(i as f32 * scale, 0.0, j as f32 * scale));
            normals.push(Vector3::new(0.0, 1.0, 0.0));
            if i != 0 && j != 0 {
                indices.push(offset - length - 1);
                indices.push(offset - length);
                indices.push(offset);
                indices.push(offset - 1);
                indices.push(offset - length as u32 - 1);
                indices.push(offset);
            }
            offset += 1;
        }
    }
    (vertices, indices, normals)
}
