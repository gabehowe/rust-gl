/// Chapel Example
/// Loads the chapel GLB and renders it using a dynamically generated shader.
use cgmath::{perspective, vec3, Deg, Vector3, Zero};
use glfw::CursorMode;
use imgui::{Condition, Ui};
use rust_gl::shader::SetValue;

use rust_gl::renderable::{Renderable, RenderableGroup};
use rust_gl::transformation::Transformable;
use rust_gl::{Data, Engine};


fn main() {
    let mat = perspective(Deg(100.0), 16. / 9., 0.01, 1000.0);

    println!("{:?}", mat);
    let mut engine = Engine::new(true, "main window").expect("Failed to create engine");
    engine.set_cursor_mode(CursorMode::Normal);

    let mut mesh = RenderableGroup::from_gltf("objects/chapel/chapel scan.gltf", "shaders/base_shader", &mut engine.data.shader_manager).expect("couldn't load mesh");
    mesh.uniform_scale(0.05);
    mesh.translate(5.0, 0.0,0.0);
    engine.add_renderable(Box::from(mesh)).expect("Failed to add renderable");
    let px_grid = (
        vec![
            vec3(0.0, 0.0, 0.0),
            vec3(0.0, 0.0, 0.1),
            vec3(0.0, 0.1, 0.0),
            vec3(0.1, 0.0, 0.0),
        ],
        vec![1, 0, 2, 0, 3],
        vec![Vector3::zero(), Vector3::zero(), Vector3::zero(), Vector3::zero()],
    );
    let mut debug_axes = Renderable::new(
        px_grid.0,
        px_grid.1,
        Some(px_grid.2),
        &engine.data.shader_manager.load_from_path("shaders/orientation_shader").expect("Failed to load shader."),
    );
    debug_axes.shader.borrow_mut().set([ 1.0f32, 0.0f32, 0.0f32 ], "ourColor").expect("Couldn't set color for debug axes.");
    debug_axes.draw_type = gl::LINES;
    engine.data.add_renderable(Box::from(debug_axes)).expect("Couldn't add renderable.");

    let renderable = engine
        .data
        .add_renderable_from_obj("objects/chapel.obj", "shaders/base_shader" ).expect("Failed to load renderable!");
    // TODO: Implement async loading
    renderable.borrow_mut().uniform_scale(0.1);
    renderable.borrow_mut().translate(20., 0.0, 0.0);
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
                    imgui.label_text("framerate", format!("{:0.1} {:0.4}", 1.0/staggered_frametime, staggered_frametime * 1000.0));
                    imgui.label_text("pos", format!("{:0.2} {:0.2} {:0.2}", pos.x, pos.y, pos.z));
                    imgui.label_text("objs", format!("sh {} | objs {}", data.shader_manager.count(), data.renderables.len()))
                });
        
        });
        renderable.borrow_mut().rotate(0.0, 0.00, 0.1 * engine.frametime as f32);
    }
    println!("done!");
}
