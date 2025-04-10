use crate::engine::shader::SetValue;
use cgmath::num_traits::Pow;
use cgmath::{perspective, Deg, Vector3, Zero};
use imgui::{Condition, Ui};
use noise::Vector2;

use crate::engine::renderable::{Mesh, Renderable};
use crate::engine::transformation::Transformable;
use crate::engine::{Data, Engine};

mod engine;

fn get_bounding_box(ren: &Renderable) -> (Vector3<f32>, Vector3<f32>) {
    let mut min = Vector3::new(0f32, 0f32, 0f32);
    let mut max = Vector3::new(0f32, 0f32, 0f32);
    for i in ren.vertices.clone() {
        for x in 0..3 {
            if i[x] < min[x] {
                min[x] = i[x];
            }
            if i[x] > max[x] {
                max[x] = i[x];
            }
        }
    }
    for x in 0..3 {
        min[x] *= ren.scale[x];
        max[x] *= ren.scale[x];
    }
    (min, max)
}

fn main() {
    let mat = perspective(Deg(100.0), 16. / 9., 0.01, 1000.0);

    println!("{:?}", mat);
    let mut engine = Engine::new(true).expect("Failed to create engine.");
    // let grid_size = 3;
    // for i in -grid_size..grid_size + 1 {
    //     for j in -grid_size..grid_size + 1 {
    let size = 20.;

    let vertices_per_unit = 0.1;
    let converted_size: f32 = size / vertices_per_unit;
    println!("{:?}", converted_size.round() as u32);
    let grid_verts = create_grid(
        converted_size.round() as u32,
        converted_size.round() as u32,
        vertices_per_unit,
        Vector2::new(-size / 2., -size / 2.),
    );

    let mesh = Mesh::from_gltf("objects/lplychapel.glb","shaders/base_shader", &mut engine.data.shader_manager).expect("couldn't load mesh");
    engine.data.renderables.push(Box::from(mesh));
    let grid = Renderable::new(
        grid_verts.0,
        grid_verts.1,
        grid_verts.2,
        engine.data.shader_manager.load_from_path("shaders/pos_shader").expect("Failed to load shader."),
    );
    engine.data.add_renderable(Box::from(grid));
    let px_grid = (
        vec![
            Vector3::new(0.0, 0.0, 0.0),
            Vector3::new(0.0, 0.0, 0.1),
            Vector3::new(0.0, 0.1, 0.0),
            Vector3::new(0.1, 0.0, 0.0),
        ],
        vec![1, 0, 2, 0, 3],
        vec![Vector3::zero(), Vector3::zero(), Vector3::zero(), Vector3::zero()],
    );
    let mut debug_axes = Renderable::new(
        px_grid.0,
        px_grid.1,
        px_grid.2,
        engine.data.shader_manager.load_from_path("shaders/orientation_shader").expect("Failed to load shader."),
    );
    engine.data.shader_manager.get_mut(debug_axes.shader).unwrap().set(vec![ 1.0f32, 0.0f32, 0.0f32 ], "ourColor").expect("Couldn't set thing");
    debug_axes.draw_type = gl::LINES;
    debug_axes.translate(0.0, 0.0, 0.0);
    engine.data.add_renderable(Box::from(debug_axes));

    // Renderable::new(vertices, indices, vec![], unsafe {Shader::load_from_path("shaders/orientation_shader")}),

    // let _bounding_box = get_bounding_box(&renderable);


    let renderable = engine
        .data
        .add_renderable_from_obj("objects/chapel.obj", "shaders/base_shader" ).expect("Couldn't create object.");
    engine
        .data
        .get_renderable_mut(renderable)
        .uniform_scale(0.1);
    engine
        .data
        .get_renderable_mut(renderable)
        .translate(20., 0.0, 0.0);
    let mut staggered_frametime = 0.0;
    let mut last_update = 0.0;
    while engine.should_keep_running() {
        let pos = engine.data.camera.pos;
        if (engine.event_handler.last_frame_time - last_update > 1.0) {
            last_update = engine.event_handler.last_frame_time;
            staggered_frametime = engine.frametime;
        }
        engine.update(|imgui: &mut Ui, frametime: f64, data: &mut Data| {
            imgui
                .window("info")
                .size([300.0, 100.0], Condition::Always)
                .build(|| {
                    imgui.label_text("framerate", format!("{:0.1} {:0.4}", 1.0/staggered_frametime, staggered_frametime * 1000.0));
                    imgui.label_text("pos", format!("{:0.2} {:0.2} {:0.2}", pos.x, pos.y, pos.z));
                    imgui.label_text("objs", format!("sh {} | objs {}", data.shader_manager.count(), data.renderables.len()))
                });
        });
        // engine.data.renderables.get_mut(1).unwrap().rotate(0.0, 0.00, 0.01);
        engine
            .data
            .get_renderable_mut(renderable)
            .rotate(0.0, 0.00, 0.1 * engine.frametime as f32);
    }
}

fn create_grid(
    width: u32,
    length: u32,
    scale: f32,
    pos: Vector2<f32>,
) -> (Vec<Vector3<f32>>, Vec<u32>, Vec<Vector3<f32>>) {
    let mut vertices = Vec::new();
    let mut indices = Vec::new();
    let mut normals = Vec::new();
    let mut offset = 0;

    for i in 0..width {
        for j in 0..length {
            vertices.push(Vector3::new(
                (i as f32 * scale) + pos.x,
                0.0,
                j as f32 * scale + pos.y,
            ));
            normals.push(Vector3::new(0.0, 1.0, 0.0));
            if i != 0 && j != 0 {
                indices.push(offset - length - 1);
                indices.push(offset - length);
                indices.push(offset);
                indices.push(offset - 1);
                indices.push(offset - length - 1);
                indices.push(offset);
            }
            offset += 1;
        }
    }
    (vertices, indices, normals)
}
