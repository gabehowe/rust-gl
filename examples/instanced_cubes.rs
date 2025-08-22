use cgmath::{vec3, Vector3};
use glfw::CursorMode;
use imgui::{Condition, Ui};
use obj::raw::parse_obj;
use obj::{FromRawVertex, TexturedVertex};
use rust_gl::shader::{FromVertex, SetValue};
use std::fs::File;
use std::io::BufReader;
use gl::TRIANGLES;
use rust_gl::renderable::{InstancedObject, Renderable};
use rust_gl::transformation::{Transform, Transformable};
use rust_gl::{Data, Engine};

fn main() {
    let mut engine = Engine::new(true, "main window").expect("Failed to create engine");
    engine.set_cursor_mode(CursorMode::Normal);

    let size = 20.;

    let vertices_per_unit = 0.1;
    let converted_size: f32 = size / vertices_per_unit;
    println!("{:?}", converted_size.round() as u32);
    let input = BufReader::new(File::open("objects/cube.obj").expect("Jimbo jones again!"));
    let obj = parse_obj(input).expect("Jimb jones the third");
    // let parsed_obj: Obj<TexturedVertex> = Obj::new(obj).expect("Jimbo jones the fourth");
    let (vertices, indices): (Vec<TexturedVertex>, Vec<u32>) = FromRawVertex::<u32>::process(
        obj.positions,
        obj.normals.clone(),
        obj.tex_coords.clone(),
        obj.polygons,
    )
    .expect("couldn't process cube mesh");
    let vtx: Vec<Vector3<f32>> = vertices.iter().map(Vector3::from_vertex).collect();

    let mut cube = InstancedObject::new(
        vtx.clone(),
        indices.clone(),
        None,
        &engine
            .data
            .shader_manager
            .load_from_path("shaders/drawing_shader")
            .expect("jimbo"),
        vec![
            Transform::new(),
            Transform::with_position(vec3(-5.0, 0.0, 0.0)),
            Transform::with_position(vec3(0.0, 6.0, 0.0)),
        ],
        vec![
            [1.0, 0.0, 1.0, 1.0],
            [0.0, 1.0, 1.0, 1.0],
            [0.0, 1.0, 0.0, 1.0],
        ],
    );
    cube.set_draw_type(TRIANGLES);
    let cube = engine
        .add_renderable(Box::from(cube))
        .expect("Failed to add renderable");
    // cube.borrow_mut().translate(0.0, 5.0, -30.0);

    let shader = &engine
        .data
        .shader_manager
        .load_from_path("shaders/pos_shader")
        .expect("jimbo");
    let ocube = engine
        .add_renderable(Box::from(Renderable::new(vtx, indices, None, shader)))
        .expect("Failed to add renderable");
    ocube.borrow_mut().translate(0.0, 0.0, 1.0);

    let mut debug_axes = Renderable::new(
        vec![
            vec3(0.0, 0.0, 0.0),
            vec3(0.0, 0.0, 0.1),
            vec3(0.0, 0.1, 0.0),
            vec3(0.1, 0.0, 0.0),
        ],
        vec![1, 0, 2, 0, 3],
        None,
        &engine
            .data
            .shader_manager
            .load_from_path("shaders/orientation_shader")
            .expect("Failed to load shader."),
    );
    debug_axes
        .shader
        .borrow_mut()
        .set(vec![1.0f32, 0.0f32, 0.0f32], "ourColor")
        .expect("Couldn't set color for debug axes.");
    debug_axes.draw_type = gl::LINES;
    debug_axes.translate(0.0, 0.0, 0.0);
    engine
        .data
        .add_renderable(Box::from(debug_axes))
        .expect("Couldn't add renderable.");

    let mut staggered_frametime = 0.0;
    let mut last_update = 0.0;
    while engine.should_keep_running() {
        let pos = engine.data.camera.pos;

        if engine.event_handler.last_frame_time - last_update > 1.0 {
            last_update = engine.event_handler.last_frame_time;
            staggered_frametime = engine.frametime;
        }
        engine.update(|imgui: &mut Ui, frametime: f64, data: &mut Data| {
            imgui
                .window("info")
                .size([300.0, 100.0], Condition::Always)
                .build(|| {
                    imgui.label_text(
                        "framerate",
                        format!(
                            "{:0.1} {:0.4}",
                            1.0 / staggered_frametime,
                            staggered_frametime * 1000.0
                        ),
                    );
                    imgui.label_text("pos", format!("{:0.2} {:0.2} {:0.2}", pos.x, pos.y, pos.z));
                    imgui.label_text(
                        "objs",
                        format!(
                            "sh {} | objs {}",
                            data.shader_manager.count(),
                            data.renderables.len()
                        ),
                    )
                });
        });
    }
}
