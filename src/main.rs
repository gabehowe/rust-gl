use cgmath::Vector3;
use imgui::Ui;
use noise::{NoiseFn, Vector2};

use crate::engine::{Data, Engine};
use crate::engine::renderable::Renderable;

mod engine;
const FRAME_SECONDS: usize = 60;
static mut FRAMES: [f32; FRAME_SECONDS * 60] = [0.0; 60 * FRAME_SECONDS];

fn main() {
    let mut engine = Engine::new(true);
    let mut renderable = unsafe { Renderable::from_obj("objects/monkey_test.obj", "shaders/base_shader") };
    // let grid_size = 3;
    // for i in -grid_size..grid_size + 1 {
    //     for j in -grid_size..grid_size + 1 {
    let size = 20.;

    let vertices_per_unit = 0.1;
    let converted_size: f32 = size / vertices_per_unit;
    println!("{:?}", converted_size.round() as u32);
    let grid_verts = create_grid(converted_size.round() as u32, converted_size.round() as u32, vertices_per_unit, Vector2::new(-size / 2., -size / 2.));

    let mut grid = Renderable::new(grid_verts.0, grid_verts.1, grid_verts.2, unsafe { engine::renderable::Shader::load_from_path("shaders/pos_shader") });
    engine.add_renderable(grid);
    //     }
    // }

    // Renderable::new(vertices, indices, vec![], unsafe {Shader::load_from_path("shaders/orientation_shader")}),
    renderable.uniform_scale(0.1);
    renderable.translate(0.0, 1.0, 0.0);
    engine.add_renderable(renderable);
    while engine.should_keep_running() {
        engine.update(callback);
    }
}

fn callback(imgui: &mut Ui) {
    // imgui.show_demo_window(&mut true);
    // let mut new_frames = [0.0; 60*FRAME_SECONDS];
    // for i in 1..(60 * FRAME_SECONDS) {
    //     unsafe {
    //         new_frames[i - 1] = FRAMES[i];
    //     }
    // }
    // unsafe {
    //     FRAMES = new_frames;
    // }
    // unsafe {
    //     FRAMES[60 * FRAME_SECONDS - 1] = engine.framerate as f32;
    // }
    // // imgui.show_demo_window(&mut true);
    // let w = imgui.window("Window :)");
    // w.build(|| unsafe {
    //     imgui.plot_lines(format!("{:.4} FPS", FRAMES[60 * FRAME_SECONDS - 1]), &FRAMES).build();
    // });
    //
    // let mut n1_g_inf = 5;
    // let mut n2_g_inf = 3;
    //
    // let mut n1_f = 13;
    // let mut n2_f = 20;
    // let mut n3_f = 23;
    //
    // let mut n1_p = 30;
    // let mut n2_p = 45;
    // let mut n3_p = 5;
    //
    // w.build(|| {
    //     imgui.label_text("Frame Rate", &format!("{:.4}", "Jim"));
    //     imgui.slider("n1_g_inf", 0, 40, &mut n1_g_inf);
    //     imgui.slider("n2_g_inf", 0, 40, &mut n2_g_inf);
    //
    //     imgui.slider("n1_f", 0, 50, &mut n1_f);
    //     imgui.slider("n2_f", 0, 50, &mut n2_f);
    //     imgui.slider("n3_f", 0, 50, &mut n3_f);
    //
    //     imgui.slider("n1_p", 0, 100, &mut n1_p);
    //     imgui.slider("n2_p", 0, 100, &mut n2_p);
    //     imgui.slider("n3_p", 0, 100, &mut n3_p);
    // });
    //
    // for i in 0..data.renderables.len() {
    //     let renderable = data.get_renderable_mut(i);
    //
    //     unsafe {
    //         renderable.shader.use_shader();
    //         renderable.shader.set_float(n1_g_inf as f32, "n1_g_inf");
    //         renderable.shader.set_float((n2_g_inf as f32) / 40., "n2_g_inf");
    //         renderable.shader.set_float(n1_f as f32, "n1_f");
    //         renderable.shader.set_float(n2_f as f32, "n2_f");
    //         renderable.shader.set_float(n3_f as f32, "n3_f");
    //         renderable.shader.set_float((n1_p as f32) / 100., "n1_p");
    //         renderable.shader.set_float((n2_p as f32) / 100., "n2_p");
    //         renderable.shader.set_float((n3_p as f32) / 100., "n3_p");
    //     }
    // }
    // // let offset = unsafe {glfwGetTime()} as f32;
    // // let perlin = Perlin::new(1);
    // // let renderable = data.get_renderable_mut(1);
    // // for i in 0..renderable.vertices.len() {
    // //     let noise = perlin.get([((renderable.vertices[i].x + offset) / 5.) as f64, ((renderable.vertices[i].z + offset) / 5.) as f64, 0.]) as c_float;
    // //     renderable.vertices[i].y = noise / 2.;
    // // }
    // // println!("{:?}", renderable.vertices[90]);
    // // unsafe { renderable.update_vertex_buffer() };
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
