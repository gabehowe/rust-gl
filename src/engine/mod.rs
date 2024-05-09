extern crate gl;
extern crate glfw;

use std::io::Read;
use std::ptr;

use cgmath::{InnerSpace, Vector3};
use gl::{COLOR_BUFFER_BIT, DEBUG_OUTPUT, DEBUG_OUTPUT_SYNCHRONOUS, DEBUG_SEVERITY_NOTIFICATION, DEBUG_SOURCE_API, DEBUG_TYPE_ERROR, DEPTH_BUFFER_BIT, DEPTH_TEST, FILL, FRONT_AND_BACK, LINE, PROGRAM_POINT_SIZE};
use glfw::{Action, Context, CursorMode, fail_on_errors, Glfw, GlfwReceiver, Key, PWindow, WindowEvent, WindowHint};
use glfw::ffi::*;
use imgui::*;
use renderable::{Renderable, Shader};
use transformation::Camera;
use util::debug_log;

pub mod util;
pub mod transformation;
pub mod renderable;

const HEIGHT: u32 = 1000;
const WIDTH: u32 = HEIGHT * 16 / 9;
const MOVESPEED: f32 = 0.025;
const ROTATIONSPEED: f32 = 0.025;

pub struct Data {
    renderables: Vec<Renderable>,
    camera: Camera,
}

impl Data {
    unsafe fn render(&mut self) {
        gl::ClearColor(0.0, 0.0, 0.2, 1.0);
        gl::Clear(COLOR_BUFFER_BIT | DEPTH_BUFFER_BIT);
        self.camera.update_buffers();
        // unsafe {renderables[0].shader.set_vec3(Vector3::new(255.0, 0.0, 0.0), "ourColor");}
        // unsafe {renderables[1].shader.set_vec3(Vector3::new(0.0, 0.0, 255.0), "ourColor");}

        for i in &mut *self.renderables {
            i.render();
        }
    }

    unsafe fn handle_input(&mut self, window: &PWindow) {
        if glfwGetKey(window.window_ptr(), KEY_D) == PRESS {
            self.camera.pos += self.camera.front.cross(Vector3::new(0.0, 1.0, 0.0)).normalize() * MOVESPEED;
        }
        if glfwGetKey(window.window_ptr(), KEY_A) == PRESS {
            self.camera.pos -= self.camera.front.cross(Vector3::new(0.0, 1.0, 0.0)).normalize() * MOVESPEED;
        }

        if glfwGetKey(window.window_ptr(), KEY_S) == PRESS {
            self.camera.pos -= self.camera.front * MOVESPEED;
        }
        if glfwGetKey(window.window_ptr(), KEY_W) == PRESS {
            self.camera.pos += self.camera.front * MOVESPEED;
        }

        if glfwGetKey(window.window_ptr(), KEY_E) == PRESS {
            self.camera.translate(Vector3::new(0.0, MOVESPEED, 0.0));
        }
        if glfwGetKey(window.window_ptr(), KEY_Q) == PRESS {
            self.camera.translate(Vector3::new(0.0, -MOVESPEED, 0.0));
        }


        if glfwGetKey(window.window_ptr(), KEY_LEFT) == PRESS {
            self.camera.yaw -= ROTATIONSPEED;
        }
        if glfwGetKey(window.window_ptr(), KEY_RIGHT) == PRESS {
            self.camera.yaw += ROTATIONSPEED;
        }
        if glfwGetKey(window.window_ptr(), KEY_UP) == PRESS {
            self.camera.pitch += ROTATIONSPEED;
        }
        if glfwGetKey(window.window_ptr(), KEY_DOWN) == PRESS {
            self.camera.pitch -= ROTATIONSPEED;
        }
    }

    pub fn get_renderable(&self, index: usize) -> &Renderable {
        return &self.renderables[index];
    }
    pub fn get_renderable_mut(&mut self, index: usize) -> &mut Renderable {
        return &mut self.renderables[index];
    }
}


pub(crate) struct Engine {
    glfw: Glfw,
    window: PWindow,
    events: GlfwReceiver<(f64, WindowEvent)>,
    pub data: Data,
    pub callback: fn(&mut Data),
}

impl Engine {
    pub fn new() -> Engine {
        let (glfw, window, events) = init_gflw();
        let camera = Engine::init_gl();
        Engine {
            glfw,
            window,
            events,
            data: Data {
                renderables: vec![],
                camera,
            },
            callback: |_| {},
        }
    }
    pub(crate) fn add_renderable(&mut self, mut renderable: Renderable) {
        unsafe { renderable.shader.bind_matrices(); }
        self.data.renderables.push(renderable);
    }

    fn init_gl() -> Camera {
        // unsafe { gl::PolygonMode(FRONT_AND_BACK, LINE); }

        let mut camera = Camera::new();
        unsafe {
            camera.initialize_buffers();
        }
        return camera;
    }

    pub(crate) fn run(&mut self) {
        self.process_events();
    }

    fn process_events(&mut self) {
        let mut wireframe = false;
        while !self.window.should_close() {
            unsafe {
                self.data.handle_input(&self.window);
                self.data.render();
                (self.callback)(&mut self.data);
            }

            self.window.swap_buffers();
            self.glfw.poll_events();
            for (_, event) in glfw::flush_messages(&self.events) {
                // println!("{:?}", event);
                match event {
                    WindowEvent::Key(Key::Escape, _, Action::Press, _) => {
                        self.window.set_should_close(true);
                    }
                    WindowEvent::Key(Key::K, _, Action::Press, _) => unsafe {
                        if self.window.get_cursor_mode() == CursorMode::Disabled {
                            self.window.set_cursor_mode(CursorMode::Normal)
                        } else {
                            self.window.set_cursor_mode(CursorMode::Disabled)
                        }
                        self.window.make_current()
                    }
                    WindowEvent::Key(Key::F2, _, Action::Press, _) => {
                        if !wireframe {
                            unsafe {gl::PolygonMode(FRONT_AND_BACK, gl::LINE) };
                        } else {
                            unsafe { gl::PolygonMode(FRONT_AND_BACK, FILL);}
                        }
                        println!("Wireframe: {}", wireframe);
                        wireframe = !wireframe;
                    }
                    WindowEvent::CursorPos(x, y) => unsafe {
                        // println!("{}, {}", x, y);
                        self.data.camera.handle_mouse(x, y);
                        // unsafe { glfwSetCursorPos(window.window_ptr(), (WIDTH / 2) as c_double, HEIGHT as c_double / 2.0); }
                    }
                    _ => {}
                }
            }
        }
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
    window.set_framebuffer_size_polling(true);
    window.set_key_polling(true);
    window.set_cursor_pos_polling(true);

    gl::load_with(|s| glfw.get_proc_address_raw(s));
    unsafe {
        // glfwSetInputMode(window.window_ptr(), CURSOR, CURSOR_DISABLED);
        gl::Enable(DEPTH_TEST);
        gl::Enable(DEBUG_OUTPUT);
        gl::Enable(DEBUG_OUTPUT_SYNCHRONOUS);
        // gl::Enable(PROGRAM_POINT_SIZE);
        gl::DebugMessageControl(DEBUG_SOURCE_API, DEBUG_TYPE_ERROR, DEBUG_SEVERITY_NOTIFICATION, 0, ptr::null(), gl::TRUE);
        gl::DebugMessageCallback(Some(debug_log), ptr::null())
    }
    return (glfw, window, events);
}






