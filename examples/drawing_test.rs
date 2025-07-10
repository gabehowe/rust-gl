use cgmath::{vec2, Vector2};
use rand::random;
use rust_gl::drawing::Draw;
use rust_gl::Engine;

const fn display_size_adjustment(inp: Vector2<f32>, size: [usize; 2]) -> Vector2<f32> {
    vec2(
        2.0 * (inp.x / size[1] as f32) - size[0] as f32 / size[1] as f32,
        2.0 * (1.0 - inp.y / size[1] as f32) - 1.0,
    )
}

fn do_circles(mut draw_func: impl FnMut(Vector2<f32>, [f32; 4]) -> ()) {
    for _ in 0..200000 {
        let x: f32 = random();
        let y: f32 = random();
        // let pos = display_size_adjustment(vec2(x, y), size);
        draw_func(vec2(x, y), [random(), random(), random(), 1.0]);
    }
}
fn main() {
    let mut engine = Engine::new(true, "drawing test").expect("Failed to create engine!");
    engine.set_cursor_mode(glfw::CursorMode::Normal);

    let mut draw = Draw::new(1, 1, &mut engine);

    do_circles(|pos, color| {draw.circle(pos*2.0 - vec2(1.0, 1.0), 0.5, color)});
    // 1000000 circles with avg frametime of 0.5s
    let mut circles: Vec<(Vector2<f32>, [f32; 4])> = vec![];
    do_circles(|pos, color| {
        circles.push((pos, color));
    });

    while engine.should_keep_running() {
        draw.update().unwrap();
        let frametime = engine.frametime;
        engine.update(|imgui, _, _| {
            imgui
                .window("info")
                .size([300., 90.], imgui::Condition::Always)
                .build(|| {
                    imgui.label_text("frametime", format!("{:.5}", frametime));
                });

            let dl = imgui.get_background_draw_list();
            // for (pos, color) in &circles {
            //     let pos = vec2(
            //         pos.x * imgui.io().display_size[0],
            //         pos.y * imgui.io().display_size[1],
            //     );
            //     let color = color.map(|c| c);
            //     dl.add_circle(<Vector2<f32> as Into<[f32; 2]>>::into(pos), 5.0, color)
            //         .filled(true)
            //         .build();
            // }
        });
    }
}
