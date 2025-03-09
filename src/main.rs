use cgmath::num_traits::Pow;
use cgmath::{perspective, Deg, Vector3};
use imgui::{Condition, Ui};
use noise::Vector2;

use crate::engine::renderable::{Renderable, SetValue, Shader};
use crate::engine::{Data, Engine};

mod engine;
const FRAME_SECONDS: usize = 60;
static mut FRAMES: [f32; FRAME_SECONDS * 60] = [0.0; 60 * FRAME_SECONDS];

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

    let grid = Renderable::new(
        grid_verts.0,
        grid_verts.1,
        grid_verts.2,
        Shader::load_from_path("shaders/mandelbrot").expect("Failed to load shader."),
    );
    // engine.add_renderable(grid);

    let px_grid = create_grid(2, 2, 2.0, Vector2::new(-1.0, -1.0));
    let mut px = Renderable::new(
        px_grid.0,
        px_grid.1,
        px_grid.2,
        Shader::load_from_path("shaders/mandelbrot").expect("Failed to load shader."),
    );
    px.translate(0.0, 0.0, 0.0);
    px.rotate(0.5 * std::f32::consts::PI, 0.0, 0.0);

    // Renderable::new(vertices, indices, vec![], unsafe {Shader::load_from_path("shaders/orientation_shader")}),

    let mut renderable =
        unsafe { Renderable::from_obj("objects/chapel.obj", "shaders/base_shader") }
            .expect("Failed to create renderable.");
    renderable.uniform_scale(0.1);
    // renderable.translate(0.0, 10.0, 0.0);
    let _bounding_box = get_bounding_box(&renderable);
    engine.add_renderable(renderable);
    engine.add_renderable(px);
    let mut val_scale = 1.;
    while engine.should_keep_running() {
        engine.update(|imgui: &mut Ui, frametime: f64, data: &mut Data| {
            // return ();
            // imgui.show_demo_window(&mut true);
            /*            imgui
                            .window("info")
                            .size([300.0, 300.0], Condition::Always)
                            .build(|| {
                                imgui.label_text("Framerate", (1.0 / frametime).to_string());
                                //imgui.input_float4("Mandelbrot Bounds", &mut mandelbrot_bounds).build();
                                imgui.slider("x off", -4.0, 4.0, &mut offset[0]);
                                imgui.slider("y off", -4.0, 4.0, &mut offset[1]);
                                imgui.slider("mx off", -4.0, 4.0, &mut minor_offset[0]);
                                imgui.slider("my off", -4.0, 4.0, &mut minor_offset[1]);
                                imgui.slider("scale", 0.0, 100.0, &mut val_scale);
                            });
            */
            let scale = f32::pow(1.15, val_scale);
            /*            for i in 0..2 {
                            data.renderables[i].shader.use_shader();
                            data.renderables[i]
                                .shader
                                .set(
                                    vec![
                                        offset[0] - offset[0] / scale
                                            + mandelbrot_bounds[0] / scale
                                            + minor_offset[0] / 10.,
                                        offset[0] - offset[0] / scale
                                            + mandelbrot_bounds[1] / scale
                                            + minor_offset[0] / 10.,
                                        offset[1] - offset[1] / scale
                                            + mandelbrot_bounds[2] / scale
                                            + minor_offset[1] / 10.,
                                        offset[1] - offset[1] / scale
                                            + mandelbrot_bounds[3] / scale
                                            + minor_offset[1] / 10.,
                                    ],
                                    "bounds",
                                )
                                .unwrap_or_else(|_| println!("Couldn't set mandelbrot bounds. "));
                        }
            */
        });

        // let translation = velocity * engine.frametime;
        //
        // velocity += acceleration * engine.frametime;
        //
        // if engine.data.renderables[1].translation.y < 1.0 {
        //     velocity.y = 0.0;
        // }
        // engine.data.renderables[1].translate(
        //     translation.x as f32,
        //     translation.y as f32,
        //     translation.z as f32,
        // );
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
