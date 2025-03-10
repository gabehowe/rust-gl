extern crate gl;
extern crate glfw;

use std::error::Error;
use std::ptr;

use cgmath::{InnerSpace, Vector3};
use gl::{
    COLOR_BUFFER_BIT, DEBUG_OUTPUT, DEBUG_OUTPUT_SYNCHRONOUS, DEBUG_SEVERITY_NOTIFICATION,
    DEBUG_SOURCE_API, DEBUG_TYPE_ERROR, DEPTH_BUFFER_BIT, DEPTH_TEST, FILL, FRONT_AND_BACK,
};
use glfw::ffi::*;
use glfw::{
    fail_on_errors, Action, Context, CursorMode, Glfw, GlfwReceiver, Key, PWindow, SwapInterval,
    WindowEvent, WindowHint,
};
use image::{ImageBuffer, Rgba};
use imgui::*;
use obj::raw::material::{Material, MtlColor};
use renderable::Renderable;
use transformation::Camera;
use util::debug_log;
use crate::engine::shader::Shader;

pub mod renderable;
pub mod transformation;
pub mod util;
pub(crate) mod shader;

const HEIGHT: u32 = 1000;
const WIDTH: u32 = HEIGHT * 16 / 9;
const MOVESPEED: f32 = 0.025;
const ROTATIONSPEED: f32 = 0.025;
const CLEARCOLOR: (f32, f32, f32, f32) = (1.0, 0.0, 0.0, 1.0);

pub struct Data {
    pub(crate) renderables: Vec<Renderable>,
    pub camera: Camera,
    wireframe_shader: Box<Shader>,
}

impl Data {
    fn render(&mut self, wireframe: bool) {
        // Safety: We know that the key is a valid key because we are using the glfw::Key enum.
        unsafe {
            gl::ClearColor(CLEARCOLOR.0, CLEARCOLOR.1, CLEARCOLOR.2, CLEARCOLOR.3);
            gl::Clear(COLOR_BUFFER_BIT | DEPTH_BUFFER_BIT);
        }
        self.camera.update_buffers(); // Only needs to be updated if it changes. Optimization?
                                      // unsafe {renderables[0].shader.set_vec3(Vector3::new(255.0, 0.0, 0.0), "ourColor");}
                                      // unsafe {renderables[1].shader.set_vec3(Vector3::new(0.0, 0.0, 255.0), "ourColor");}
        for i in &mut *self.renderables {
            i.render(if wireframe {
                Some(&mut self.wireframe_shader)
            } else {
                None
            });
        }
    }
    pub(crate) fn add_renderable(&mut self, mut renderable: Renderable) -> Result<usize, ()> {
        unsafe {
            renderable.shader.bind_matrices()?;
        }
        self.renderables.push(renderable);
        Ok(self.renderables.len() - 1)
    }

    fn handle_input(&mut self, window: &PWindow) {
        let mut speed = MOVESPEED;
        if window.get_key(Key::LeftShift) == Action::Press {
            speed *= 3.0;
        }
        if window.get_key(Key::D) == Action::Press {
            self.camera.pos += self
                .camera
                .front
                .cross(Vector3::new(0.0, 1.0, 0.0))
                .normalize()
                * speed;
        }
        if window.get_key(Key::A) == Action::Press {
            self.camera.pos -= self
                .camera
                .front
                .cross(Vector3::new(0.0, 1.0, 0.0))
                .normalize()
                * speed;
        }

        if window.get_key(Key::S) == Action::Press {
            self.camera.pos -= self.camera.front * speed;
        }
        if window.get_key(Key::W) == Action::Press {
            self.camera.pos += self.camera.front * speed;
        }

        if window.get_key(Key::E) == Action::Press {
            self.camera.translate(Vector3::new(0.0, speed, 0.0));
        }
        if window.get_key(Key::Q) == Action::Press {
            self.camera.translate(Vector3::new(0.0, -speed, 0.0));
        }

        if window.get_key(Key::Left) == Action::Press {
            self.camera.yaw -= ROTATIONSPEED;
        }
        if window.get_key(Key::Right) == Action::Press {
            self.camera.yaw += ROTATIONSPEED;
        }
        if window.get_key(Key::Up) == Action::Press {
            self.camera.pitch += ROTATIONSPEED;
        }
        if window.get_key(Key::Down) == Action::Press {
            self.camera.pitch -= ROTATIONSPEED;
        }
    }

    pub fn get_renderable(&self, index: usize) -> &Renderable {
        &self.renderables[index]
    }
    pub fn get_renderable_mut(&mut self, index: usize) -> &mut Renderable {
        &mut self.renderables[index]
    }
}

pub(crate) struct Engine {
    glfw: Glfw,
    window: PWindow,
    events: GlfwReceiver<(f64, WindowEvent)>,
    pub frametime: f64,
    pub data: Data,
    pub event_handler: EventHandler,
    pub frame_index: u32,
}

impl Engine {
    pub fn new(imgui: bool) -> Result<Engine, Box<dyn Error>> {
        let (glfw, mut window, events) = init_gflw();
        let camera = Engine::init_gl();
        unsafe {
            // dunno why these are here.
            gl::GetString(gl::VERSION);
            gl::GetString(gl::RENDERER);
        }
        let mut event_handler = EventHandler::raw();
        if imgui {
            event_handler = EventHandler::new(&mut window);
        }

        Ok(Engine {
            glfw,
            window,
            events,
            frametime: 0.0,
            data: Data {
                renderables: vec![],
                camera,
                wireframe_shader: Box::from(Shader::load_from_mtl(
                    Material {
                        ambient: Some(MtlColor::Rgb(0.7, 0.3, 0.0)),
                        diffuse: Some(MtlColor::Rgb(0.0, 0.0, 0.0)),
                        specular: Some(MtlColor::Rgb(0.0, 0.0, 0.0)),
                        emissive: Some(MtlColor::Rgb(0.0, 0.0, 0.0)),
                        transmission_filter: None,
                        illumination_model: None,
                        dissolve: None,
                        specular_exponent: Some(0.0),
                        optical_density: Some(0.0),
                        ambient_map: None,
                        diffuse_map: None,
                        specular_map: None,
                        emissive_map: None,
                        dissolve_map: None,
                        bump_map: None,
                    },
                    "objects",
                    "shaders/base_shader",
                )?),
            },
            event_handler,
            frame_index: 0,
        })
    }
    pub fn write_to_file(&self, path: &str) {
        let (width, height) = self.window.get_size();
        let mut data = vec![0u8; (width * height * 4) as usize];
        unsafe {
            gl::ReadPixels(
                0,
                0,
                width,
                height,
                gl::RGBA,
                gl::UNSIGNED_BYTE,
                data.as_mut_ptr().cast(),
            );
        }
        let img = ImageBuffer::<Rgba<u8>, _>::from_raw(width as u32, height as u32, data)
            .expect("Failed to create ImageBuffer.");

        image::imageops::flip_vertical(&img)
            .save(path)
            .expect("Failed to save image.");
    }

    fn init_gl() -> Camera {
        // unsafe { gl::PolygonMode(FRONT_AND_BACK, LINE); }
        let mut camera = Camera::new();
        unsafe {
            camera.initialize_buffers();
            gl::CullFace(gl::BACK);
            gl::Enable(gl::TEXTURE_2D);
            gl::LineWidth(0.1);
            gl::Enable(gl::BLEND);
            gl::BlendFunc(gl::SRC_ALPHA, gl::ONE_MINUS_SRC_ALPHA);
        }
        camera
    }

    pub(crate) fn should_keep_running(&self) -> bool {
        !self.window.should_close()
    }

    pub(crate) fn update<F>(&mut self, mut imgui_callback: F)
    where
        F: FnMut(&mut Ui, f64, &mut Data),
    {
        self.frame_index += 1;
        
        self.data.handle_input(&self.window);
        self.data.render(self.event_handler.wireframe);
        // Allow for disabling imgui
        if self.event_handler.imgui.is_some() && self.event_handler.show_imgui {
            let imgui_glfw_ref = self.event_handler.imgui_glfw.as_mut().unwrap();
            let imgui_ref = self.event_handler.imgui.as_mut().unwrap();
            let frame = imgui_ref.frame();
            imgui_callback(frame, self.frametime, &mut self.data);
            imgui_glfw_ref.draw(frame, &mut self.window);
            imgui_glfw_ref.get_renderer().render(imgui_ref);
        }

        self.window.swap_buffers();
        self.process_glfw_events();

        self.event_handler.current_frame_time = unsafe { glfwGetTime() };
        self.frametime = self.event_handler.current_frame_time - self.event_handler.last_frame_time;
        self.event_handler.last_frame_time = self.event_handler.current_frame_time;
        // println!("Frame rate: {}", frame_rate);
    }
    fn process_glfw_events(&mut self) {
        self.glfw.poll_events();
        for (_, event) in glfw::flush_messages(&self.events) {
            if self.event_handler.imgui.is_some() && self.event_handler.show_imgui {
                self.event_handler
                    .imgui_glfw
                    .as_mut()
                    .unwrap()
                    .handle_event(self.event_handler.imgui.as_mut().unwrap(), &event);
            }
            match event {
                WindowEvent::Key(Key::Escape, _, Action::Press, _) => {
                    self.window.set_should_close(true);
                }
                WindowEvent::Key(Key::K, _, Action::Press, _) => {
                    if self.window.get_cursor_mode() == CursorMode::Disabled {
                        self.window.set_cursor_mode(CursorMode::Normal)
                    } else {
                        self.window.set_cursor_mode(CursorMode::Disabled)
                    }
                    self.window.make_current()
                }
                WindowEvent::Key(Key::F12, _, Action::Press, _) => {
                    let formatted = format!(
                        "{}.png",
                        std::time::SystemTime::now()
                            .duration_since(std::time::UNIX_EPOCH)
                            .unwrap()
                            .as_millis()
                    );
                    self.write_to_file(formatted.as_str());
                }
                WindowEvent::Key(Key::F3, _, Action::Press, _) => {
                    self.event_handler.show_imgui = !self.event_handler.show_imgui
                }
                WindowEvent::Key(Key::F2, _, Action::Press, _) => {
                    if !self.event_handler.wireframe {
                        unsafe { gl::PolygonMode(FRONT_AND_BACK, gl::LINE) };
                    } else {
                        unsafe {
                            gl::PolygonMode(FRONT_AND_BACK, FILL);
                        }
                    }
                    println!("Wireframe: {}", self.event_handler.wireframe);
                    self.event_handler.wireframe = !self.event_handler.wireframe;
                }
                WindowEvent::CursorPos(x, y) => {
                    // println!("{}, {}", x, y);
                    if self.window.get_cursor_mode() == CursorMode::Disabled {
                        self.data.camera.handle_mouse(x, y);
                    }
                    // unsafe { glfwSetCursorPos(window.window_ptr(), (WIDTH / 2) as c_double, HEIGHT as c_double / 2.0); }
                }
                _ => {}
            }
        }
    }
}

pub struct EventHandler {
    wireframe: bool,
    current_frame_time: f64,
    last_frame_time: f64,
    imgui: Option<imgui::Context>,
    imgui_glfw: Option<imgui_glfw_rs::ImguiGLFW>,
    show_imgui: bool,
}
impl EventHandler {
    fn new(window: &mut PWindow) -> EventHandler {
        // let mut renderer = imgui_opengl_renderer::Renderer::new(&mut ctx, |s| self.window.get_proc_address(s) as _);

        let mut imgui = imgui::Context::create();
        let imgui_glfw = imgui_glfw_rs::ImguiGLFW::new(&mut imgui, window);
        let window_size = window.get_size();
        println!("Window Size: {:?}", window_size);
        imgui.io_mut().display_size = [window_size.0 as f32, window_size.1 as f32];
        EventHandler {
            last_frame_time: unsafe { glfwGetTime() },
            imgui: Some(imgui),
            imgui_glfw: Some(imgui_glfw),
            ..Self::raw()
        }
    }
    fn raw() -> Self {
        EventHandler {
            wireframe: false,
            current_frame_time: 0.0,
            last_frame_time: 0.0,
            imgui: None,
            imgui_glfw: None,
            show_imgui: true,
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
    window.set_char_polling(true);
    window.set_cursor_pos_polling(true);
    window.set_mouse_button_polling(true);
    window.set_scroll_polling(true);
    glfw.set_swap_interval(SwapInterval::None);

    gl::load_with(|s| glfw.get_proc_address_raw(s));
    unsafe {
        glfwSetInputMode(window.window_ptr(), CURSOR, CURSOR_DISABLED);
        gl::Enable(DEPTH_TEST);
        gl::Enable(DEBUG_OUTPUT);
        gl::Enable(DEBUG_OUTPUT_SYNCHRONOUS);
        // gl::Enable(PROGRAM_POINT_SIZE);
        gl::DebugMessageControl(
            DEBUG_SOURCE_API,
            DEBUG_TYPE_ERROR,
            DEBUG_SEVERITY_NOTIFICATION,
            0,
            ptr::null(),
            gl::TRUE,
        );
        gl::DebugMessageCallback(Some(debug_log), ptr::null());
    }
    (glfw, window, events)
}
