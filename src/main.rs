use cgmath::num_traits::ToPrimitive;
use cgmath::Vector3;
use imgui::{Condition, Ui};
use noise::{NoiseFn, Vector2};

use crate::engine::{Data, Engine};
use crate::engine::renderable::{Renderable, Shader};

mod engine;
const FRAME_SECONDS: usize = 60;
static mut FRAMES: [f32; FRAME_SECONDS * 60] = [0.0; 60 * FRAME_SECONDS];

fn main() {
    let mut engine = Engine::new(true);
    // let grid_size = 3;
    // for i in -grid_size..grid_size + 1 {
    //     for j in -grid_size..grid_size + 1 {
    let size = 20.;

    let vertices_per_unit = 0.1;
    let converted_size: f32 = size / vertices_per_unit;
    println!("{:?}", converted_size.round() as u32);
    // let grid_verts = create_grid(converted_size.round() as u32, converted_size.round() as u32, vertices_per_unit, Vector2::new(-size / 2., -size / 2.));
    //
    // let mut grid = Renderable::new(grid_verts.0, grid_verts.1, grid_verts.2, unsafe { Shader::load_from_path("shaders/pos_shader") });
    // engine.add_renderable(grid);

    let px_grid = create_grid(2, 2, 2.0, Vector2::new(-1.0, -1.0));
    let mut px = Renderable::new(px_grid.0, px_grid.1, px_grid.2, unsafe { Shader::load_from_path("shaders/pixel_shader") });
    px.translate(0.0, 0.0, 0.0);
    px.rotate(0.5 * std::f32::consts::PI, 0.0, 0.0);

    // Renderable::new(vertices, indices, vec![], unsafe {Shader::load_from_path("shaders/orientation_shader")}),

    // let mut renderable = unsafe { Renderable::from_obj("objects/chapel.obj", "shaders/base_shader") };
    // renderable.uniform_scale(0.1);
    // renderable.translate(0.0, 50.0, 0.0);
    // engine.add_renderable(renderable);
    engine.add_renderable(px);
    let mut velocity = Vector3::new(0., 0., 0.);
    let acceleration = Vector3::new(0., -9.81, 0.);
    while engine.should_keep_running() {
        engine.update(callback);

        // let translation = (velocity * engine.frametime);
        //
        // velocity += acceleration;
        //
        // if engine.data.renderables[1].translation.y < 1.0 {
        //     velocity.y = 0.0;
        // }
        // engine.data.renderables[1].translate(translation.x as f32, translation.y as f32, translation.z as f32);
    }
}


fn callback(imgui: &mut Ui, frametime: f64) {
    // imgui.show_demo_window(&mut true);
    imgui.window("info").size([300.0, 100.0], Condition::Always).build(|| {
        imgui.label_text("Framerate", ( 1.0/frametime ).to_string())
    });
}

fn create_grid(width: u32, length: u32, scale: f32, pos: Vector2<f32>) -> (Vec<Vector3<f32>>, Vec<u32>, Vec<Vector3<f32>>) {
    let mut vertices = Vec::new();
    let mut indices = Vec::new();
    let mut normals = Vec::new();
    let mut offset = 0;

    for i in 0..width {
        for j in 0..length {
            vertices.push(Vector3::new((i as f32 * scale) + pos.x, 0.0, j as f32 * scale + pos.y));
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
