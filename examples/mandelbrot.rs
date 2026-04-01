use cgmath::{Vector3, Vector4, Zero, vec3};
use glfw::CursorMode;
use imgui::{Condition, Ui};
use rust_gl::shader::{SetValue};

use rust_gl::renderable::{Renderable};
use rust_gl::transformation::Transformable;
use rust_gl::{Data, Engine};


fn main() {
    let mut engine = Engine::new(true, "main window").expect("Failed to create engine");
    engine.set_cursor_mode(CursorMode::Normal);
    
    let size = 20.;

    let vertices_per_unit = 0.1;
    let converted_size: f32 = size / vertices_per_unit;
    println!("{:?}", converted_size.round() as u32);

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

    let mandel_shader = &engine.data.shader_manager.load_from_path("shaders/mandelbrot").expect("Failed to load mandelbrot shader.");
    let mut mandelbrot_canvas = Renderable::new(
        vec![vec3(-1., -1., 0.), vec3(1., -1., 0.), vec3(1., 1., 0.), vec3(-1., 1., 0.)],
        vec![0, 1, 2, 3, 0, 2], None,
        mandel_shader);
    engine.data.add_renderable(Box::from(mandelbrot_canvas)).expect("Couldn't add renderable.");

    let mut staggered_frametime = 0.0;
    let mut last_update = 0.0;
    let mut mandelbrot_value: [f32; 4] = [-0.8, 0.8, 0.8, -0.3];
    while engine.should_keep_running() {
        let pos = engine.data.camera.pos;

        if engine.event_handler.last_frame_time - last_update > 1.0 {
            last_update = engine.event_handler.last_frame_time;
            staggered_frametime = engine.frametime;
        }
        engine.update(|imgui: &mut Ui, frametime: f64, data: &mut Data| {
            mandel_shader.borrow_mut().set(mandelbrot_value, "bounds").expect("Couldn't set!");
            imgui
                .window("info")
                .size([300.0, 200.0], Condition::Always)
                .build(|| {
                    imgui.label_text("framerate", format!("{:0.1} {:0.4}", 1.0/staggered_frametime, staggered_frametime * 1000.0));
                    imgui.label_text("pos", format!("{:0.2} {:0.2} {:0.2}", pos.x, pos.y, pos.z));
                    imgui.label_text("objs", format!("sh {} | objs {}", data.shader_manager.count(), data.renderables.len()));
                    imgui.input_float4("mandelbrot_bounds", &mut mandelbrot_value).build()
                });
        });
    }
}
