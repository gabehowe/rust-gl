extern crate gl;
extern crate glfw;

use std::f32::consts::PI;
use std::io::Read;
use std::mem::size_of;
use std::os::raw::c_double;
use std::ptr;

use cgmath::{Rad, Vector3};
use gl::{COLOR_BUFFER_BIT, DEBUG_OUTPUT, DEBUG_OUTPUT_SYNCHRONOUS, DEBUG_SEVERITY_NOTIFICATION, DEBUG_SOURCE_API, DEBUG_TYPE_ERROR, DEPTH_BUFFER_BIT, DEPTH_TEST, FLOAT, FRONT_AND_BACK, LINE, PROGRAM_POINT_SIZE};
use gl::types::{GLfloat, GLsizei};
use glfw::{Action, Context, CursorMode, fail_on_errors, Glfw, GlfwReceiver, Key, PWindow, WindowEvent, WindowHint};
use glfw::ffi::*;
use imgui::*;
use crate::buffer::ShaderMemoryManager;

use crate::renderable::{Renderable, Shader};
use crate::transformation::Camera;
use crate::util::debug_log;

mod renderable;
mod transformation;
mod util;
mod buffer;

const HEIGHT: u32 = 1000;
const WIDTH: u32 = HEIGHT * 16 / 9;
const MOVESPEED: f32 = 0.025;
const ROTATIONSPEED: f32 = 0.025;


unsafe fn handle_input(glfw: &PWindow, camera: &mut Camera) {
    if glfwGetKey(glfw.window_ptr(), KEY_W) == PRESS {
        camera.translate(Vector3::new(0.0, 0.0, MOVESPEED));
    }
    if glfwGetKey(glfw.window_ptr(), KEY_S) == PRESS {
        camera.translate(Vector3::new(0.0, 0.0, -MOVESPEED));
    }

    if glfwGetKey(glfw.window_ptr(), KEY_D) == PRESS {
        camera.translate(Vector3::new(-MOVESPEED, 0.0, 0.0));
    }
    if glfwGetKey(glfw.window_ptr(), KEY_A) == PRESS {
        camera.translate(Vector3::new(MOVESPEED, 0.0, 0.0));
    }

    if glfwGetKey(glfw.window_ptr(), KEY_E) == PRESS {
        camera.translate(Vector3::new(0.0, MOVESPEED, 0.0));
    }
    if glfwGetKey(glfw.window_ptr(), KEY_Q) == PRESS {
        camera.translate(Vector3::new(0.0, -MOVESPEED, 0.0));
    }


    if glfwGetKey(glfw.window_ptr(), KEY_LEFT) == PRESS {
        camera.rot.y += ROTATIONSPEED;
    }
    if glfwGetKey(glfw.window_ptr(), KEY_RIGHT) == PRESS {
        camera.rot.y -= ROTATIONSPEED;
    }
    if glfwGetKey(glfw.window_ptr(), KEY_UP) == PRESS {
        camera.rot.x -= ROTATIONSPEED;
    }
    if glfwGetKey(glfw.window_ptr(), KEY_DOWN) == PRESS {
        camera.rot.x += ROTATIONSPEED;
    }
}


fn init_gflw() -> (Glfw, PWindow, GlfwReceiver<(f64, WindowEvent)>) {
    use glfw::fail_on_errors;
    let mut glfw = glfw::init(fail_on_errors!()).unwrap();

    let (mut window, events) = glfw
        .create_window(WIDTH, HEIGHT, "OpenGL", glfw::WindowMode::Windowed)
        .expect("Failed to create GLFW window.");

    glfw.window_hint(WindowHint::ContextVersionMajor(4));
    glfw.window_hint(WindowHint::ContextVersionMinor(6));
    glfw.window_hint(WindowHint::OpenGlProfile(glfw::OpenGlProfileHint::Core));
    glfw.window_hint(WindowHint::OpenGlDebugContext(true));

    window.make_current();
    window.set_key_polling(true);
    window.set_cursor_pos_polling(true);

    gl::load_with(|s| glfw.get_proc_address_raw(s));
    unsafe {
        // glfwSetInputMode(window.window_ptr(), CURSOR, CURSOR_DISABLED);
        gl::Enable(DEPTH_TEST);
        gl::Enable(DEBUG_OUTPUT);
        gl::Enable(DEBUG_OUTPUT_SYNCHRONOUS);
        gl::Enable(PROGRAM_POINT_SIZE);
        gl::DebugMessageControl(DEBUG_SOURCE_API, DEBUG_TYPE_ERROR, DEBUG_SEVERITY_NOTIFICATION, 0, ptr::null(), gl::TRUE);
        gl::DebugMessageCallback(Some(debug_log), ptr::null())
    }
    return (glfw, window, events);
}

fn init_gl() -> (Vec<Renderable>, Camera) {
    // unsafe { gl::PolygonMode(FRONT_AND_BACK, LINE); }
    let vertices = vec![-0.5f32, -0.5, 0.0,
                        0.5, -0.5, 0.0,
                        0.0, 0.5, 0.0];
    let indices: Vec<u32> = vec![0, 1, 2, 0, 6, 2, 3, 4, 5];
    let arrow_vertices = vec![
        0.25f32, 1.0, 0.0,
        -0.25, 1.0, 0.0,
        -0.25, 0.5, 0.0,
        -0.5, 0.5, 0.0,
        0., 0., 0.0,
        0.5, 0.5, 0.0,
        0.25, 0.5, 0.0,
        0.25, 1.0, 0.0,
    ];
    let mut renderables = vec![
        unsafe { Renderable::from_obj("objects/cube.obj", "shaders/pos_shader") },
        // Renderable::new(vertices, indices, vec![], unsafe {Shader::load_from_path("shaders/orientation_shader")}),
    ];
    renderables[0].uniform_scale(0.1);

    let mut camera = Camera::new();
    let mut shaders: Vec<&mut Shader> = (&mut renderables)
        .iter_mut()
        .map(|it| &mut it.shader)
        .collect();
    unsafe {
        camera.initialize_buffers(&mut shaders);
    }
    return (renderables, camera);
}

unsafe fn render(renderables: &mut Vec<Renderable>, camera: &mut Camera) {
    gl::ClearColor(0.0, 0.0, 0.2, 1.0);
    gl::Clear(COLOR_BUFFER_BIT | DEPTH_BUFFER_BIT);
    camera.update_buffers();
    // unsafe {renderables[0].shader.set_vec3(Vector3::new(255.0, 0.0, 0.0), "ourColor");}
    // unsafe {renderables[1].shader.set_vec3(Vector3::new(0.0, 0.0, 255.0), "ourColor");}

    for i in &mut *renderables {
        i.render();
    }
}

fn process_events(
    mut glfw: Glfw,
    mut window: PWindow,
    events: GlfwReceiver<(f64, WindowEvent)>,
    renderables: &mut Vec<Renderable>,
    camera: &mut Camera,
) {
    while !window.should_close() {
        unsafe {
            handle_input(&window, camera);
            render(renderables, camera);
        }

        window.swap_buffers();
        glfw.poll_events();
        for (_, event) in glfw::flush_messages(&events) {
            // println!("{:?}", event);
            match event {
                WindowEvent::Key(Key::Escape, _, Action::Press, _) => {
                    window.set_should_close(true);
                }
                WindowEvent::Key(Key::K, _, Action::Press, _) => unsafe {
                    if window.get_cursor_mode() == CursorMode::Disabled {
                        window.set_cursor_mode(CursorMode::Normal)
                    } else {
                        window.set_cursor_mode(CursorMode::Disabled)
                    }
                    window.make_current()
                }
                WindowEvent::CursorPos(x, y) => unsafe {
                    // println!("{}, {}", x, y);
                    camera.handle_mouse(x, y);
                    unsafe { glfwSetCursorPos(window.window_ptr(), (WIDTH / 2) as c_double, HEIGHT as c_double / 2.0); }
                }
                _ => {}
            }
        }
    }
}


fn main() {
    let (glfw, mut window, events) = init_gflw();
    // let mut shader_manager = ShaderMemoryManager::new();
    let (mut renderables, mut camera) = init_gl();
    // let mut obj = unsafe { Renderable::from_obj("objects/cube.obj", "shaders/orientation_shader") };
    // obj.scale(0.1, 0.1, 0.1);
    // renderables.push(obj);
    process_events(glfw, window, events, &mut renderables, &mut camera);
}
