use cgmath::{perspective, vec3, Deg, Vector2, Vector3, Zero};
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
    let size = 20.;

    let vertices_per_unit = 0.1;
    let converted_size: f32 = size / vertices_per_unit;
    println!("{:?}", converted_size.round() as u32);
    let grid_verts = RenderableGroup::create_grid(
        converted_size.round() as u32,
        converted_size.round() as u32,
        vertices_per_unit,
        Vector2::new(-size / 2., -size / 2.),
    );

    let mut mesh = RenderableGroup::from_gltf("objects/chapel/chapel scan.gltf", "shaders/base_shader", &mut engine.data.shader_manager).expect("couldn't load mesh");
    mesh.uniform_scale(0.05);
    mesh.translate(5.0, 0.0,0.0);
    engine.add_renderable(Box::from(mesh)).expect("Failed to add renderable");
    let grid = Renderable::new(
        grid_verts.0,
        grid_verts.1,
        Some(grid_verts.2),
        &engine.data.shader_manager.load_from_path("shaders/pos_shader").expect("Failed to load shader."),
    );
    // engine.data.add_renderable(Box::from(grid));
    let screen_pts = 0;
    let screen_pts = (
        vec![
            vec3(-1.0, -1.0,0.0),
            vec3(-1.0, 1.0, 0.0),
            vec3(1.0, 1.0, 0.0),
            vec3(1.0, -1.0, 0.0),
        ],
        vec![0,1,2,2,3,0],
        vec![Vector3::zero(), Vector3::zero(), Vector3::zero(), Vector3::zero()],
    );
    engine.data.create_framebuffer_texture();
    let screen_pts = Renderable::new(
        screen_pts.0,
        screen_pts.1,
        Some(screen_pts.2),
        &engine.data.shader_manager.load_from_path("shaders/screen_shader").expect("Failed to load shader."),
    );
    // engine.data.shader_manager.get_mut(screen_pts.shader).unwrap().textures.insert("screen".to_string(), engine.data.frame_buffer_texture.unwrap().1);
    // let screen_pts = engine.data.add_renderable(Box::from(screen_pts)).unwrap();
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
    debug_axes.shader.borrow_mut().set(vec![ 1.0f32, 0.0f32, 0.0f32 ], "ourColor").expect("Couldn't set color for debug axes.");
    debug_axes.draw_type = gl::LINES;
    debug_axes.translate(0.0, 0.0, 0.0);
    engine.data.add_renderable(Box::from(debug_axes)).expect("Couldn't add renderable.");

    // Renderable::new(vertices, indices, vec![], unsafe {Shader::load_from_path("shaders/orientation_shader")}),

    // let _bounding_box = get_bounding_box(&renderable);


    let renderable = engine
        .data
        .add_renderable_from_obj("objects/chapel.obj", "shaders/base_shader" ).expect("Couldn't create object.");
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
        // if engine.data.frame_buffer_texture.is_some() {
        //     unsafe {
        //         gl::BindFramebuffer(gl::FRAMEBUFFER, 0);
        //         gl::ClearColor(crate::engine::CLEARCOLOR.0, crate::engine::CLEARCOLOR.1, crate::engine::CLEARCOLOR.2, crate::engine::CLEARCOLOR.3);
        //         gl::Clear(COLOR_BUFFER_BIT | DEPTH_BUFFER_BIT);
        //     }
        //     // let reference = engine.data.renderables.get_mut(screen_pts).unwrap();
        //     // reference.set_is(true);
        //     // reference.render(&mut engine.data.shader_manager, None);
        // }
        // engine.data.renderables.get_mut(1).unwrap().rotate(0.0, 0.00, 0.01);
        renderable.borrow_mut().rotate(0.0, 0.00, 0.1 * engine.frametime as f32);
    }
}
