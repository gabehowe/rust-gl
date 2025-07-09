extern crate gl;

pub use cgmath;
pub use glfw;
pub use imgui;
use std::cell::RefCell;

// use transformation::Transformation;
use crate::shader::ShaderPtr;
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
use renderable::Render;
use renderable::Renderable;
use shader::{MaybeColorTexture, NarrowingMaterial, ShaderManager};
use std::error::Error;
use std::ptr;
use std::sync::Arc;
use transformation::Camera;
use util::debug_log;

pub mod drawing;
mod glutil;
pub mod renderable;
pub mod shader;
pub mod transformation;
pub mod util;

pub const HEIGHT: usize = 1000;
pub const WIDTH: usize = HEIGHT * 16 / 9;
const MOVESPEED: f32 = 2.5;
const ROTATIONSPEED: f32 = 2.5;
pub(crate) const CLEARCOLOR: (f32, f32, f32, f32) = (0.1, 0.0, 0.0, 1.0);
type RenderablePtr = Arc<RefCell<Box<dyn Render>>>;
fn new_renderable_ptr<T: Render + 'static>(renderable: T) -> RenderablePtr {
    Arc::new(RefCell::new(Box::new(renderable)))
}
pub struct Data {
    pub renderables: Vec<RenderablePtr>,
    pub camera: Camera,
    wireframe_shader: ShaderPtr,
    pub shader_manager: ShaderManager,
    pub frame_buffer_texture: Option<(u32, u32)>,
    pub should_clear: bool,
}

impl Data {
    fn update(&mut self) -> Result<(), Box<dyn Error>> {
        self.shader_manager.update()
    }
    fn render(&mut self, wireframe: bool) -> Result<(), Box<dyn Error>> {
        // Safety: We know that the key is a valid key because we are using the glfw::Key enum.
        unsafe {
            if self.should_clear {
                gl::ClearColor(CLEARCOLOR.0, CLEARCOLOR.1, CLEARCOLOR.2, CLEARCOLOR.3);
                gl::Clear(COLOR_BUFFER_BIT | DEPTH_BUFFER_BIT);
            }
        }
        self.camera.update_buffers(); // Only needs to be updated if it changes. TODO: Optimization?
        for i in self.renderables.iter_mut().map(|x| x.try_borrow_mut()) {
            i?.render(wireframe.then(|| self.wireframe_shader.clone()))?;
        }
        Ok(())
    }
    pub fn add_renderable(
        &mut self,
        renderable: Box<dyn Render>,
    ) -> Result<RenderablePtr, Box<dyn Error>> {
        let arc = Arc::new(RefCell::new(renderable));
        self.renderables.push(arc.clone());
        Ok(arc.clone())
    }
    pub fn add_renderable_rc(&mut self, rc: &RenderablePtr) {
        self.renderables.push(rc.clone());
    }
    pub fn add_renderable_from_obj(
        &mut self,
        path: &str,
        shaderpath: &str,
    ) -> Result<RenderablePtr, Box<dyn Error>> {
        let renderable = Renderable::from_obj(path, shaderpath, &mut self.shader_manager)?;
        self.add_renderable(Box::from(renderable))
    }

    fn handle_input(&mut self, window: &PWindow, frametime: f64) {
        let mut speed = MOVESPEED * frametime as f32;
        let rot_speed = ROTATIONSPEED * frametime as f32;
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
            self.camera.yaw -= rot_speed;
        }
        if window.get_key(Key::Right) == Action::Press {
            self.camera.yaw += rot_speed;
        }
        if window.get_key(Key::Up) == Action::Press {
            self.camera.pitch += rot_speed;
        }
        if window.get_key(Key::Down) == Action::Press {
            self.camera.pitch -= rot_speed;
        }
    }

    pub fn get_renderable(&self, index: usize) -> &RenderablePtr {
        &self.renderables[index]
    }
    pub fn get_renderable_mut(&mut self, index: usize) -> &mut RenderablePtr {
        &mut self.renderables[index]
    }
    pub fn create_framebuffer_texture(&mut self) {
        let mut buff = 0;
        unsafe {
            gl::GenFramebuffers(1, &mut buff);
            gl::BindFramebuffer(gl::FRAMEBUFFER, buff);
        }
        let mut texture = 0;
        unsafe {
            gl::GenTextures(1, &mut texture);
            gl::BindTexture(gl::TEXTURE_2D, texture);
            gl::TexImage2D(
                gl::TEXTURE_2D,
                0,
                gl::RGB as i32,
                WIDTH as i32 * 3,
                HEIGHT as i32 * 3,
                0,
                gl::RGB,
                gl::UNSIGNED_BYTE,
                ptr::null(),
            );
            gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MIN_FILTER, gl::LINEAR as i32);
            gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MAG_FILTER, gl::LINEAR as i32);
            gl::FramebufferTexture2D(
                gl::FRAMEBUFFER,
                gl::COLOR_ATTACHMENT0,
                gl::TEXTURE_2D,
                texture,
                0,
            );
            gl::BindFramebuffer(gl::FRAMEBUFFER, 0);
        }
        self.frame_buffer_texture = Some((buff, texture));
    }
}

pub struct Engine {
    glfw: Glfw,
    window: PWindow,
    events: GlfwReceiver<(f64, WindowEvent)>,
    pub frametime: f64,
    pub data: Data,
    pub event_handler: EventHandler,
    pub frame_index: u32,
    pub size: [usize; 2],
}

impl Engine {
    pub fn add_renderable(
        &mut self,
        renderable: Box<dyn Render>,
    ) -> Result<RenderablePtr, Box<dyn Error>> {
        self.data.add_renderable(renderable)
    }
    pub fn add_renderable_rc(&mut self, renderable: &RenderablePtr) {
        self.data.add_renderable_rc(renderable)
    }
    pub fn new(imgui: bool, window_name: &str) -> Result<Engine, Box<dyn Error>> {
        let (glfw, mut window, events) = init_gflw(window_name);
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
        let mut shader_manager = ShaderManager::new();
        let mat = NarrowingMaterial {
            diffuse: Some(MaybeColorTexture::RGBA([0.0, 1.0, 0.0, 1.0])),
            emissive: None,
            specular: None,
            metallic: None,
            roughness: None,
            ambient_scaling: None,
            normal: None,
        };
        let wireframe_id = shader_manager.register(mat.into_shader(
            include_str!("../shaders/base_shader.vert").to_string(),
            include_str!("../shaders/base_shader.frag").to_string(),
        )?);
        Ok(Engine {
            glfw,
            window,
            events,
            frametime: 0.0,
            data: Data {
                renderables: Vec::new(),
                camera,
                shader_manager,
                wireframe_shader: wireframe_id,
                frame_buffer_texture: None,
                should_clear: true,
            },
            event_handler,
            frame_index: 0,
            size: [WIDTH, HEIGHT],
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
    pub fn set_cursor_mode(&mut self, cursor_mode: CursorMode) {
        self.window.set_cursor_mode(cursor_mode);
    }

    fn init_gl() -> Camera {
        // unsafe { gl::PolygonMode(FRONT_AND_BACK, LINE); }
        let mut camera = Camera::new();
        unsafe {
            camera.initialize_buffers();
            // gl::CullFace(gl::BACK);
            gl::Enable(gl::TEXTURE_2D);
            gl::LineWidth(0.1);
            gl::Enable(gl::BLEND);
            gl::BlendFunc(gl::SRC_ALPHA, gl::ONE_MINUS_SRC_ALPHA);
        }
        camera
    }

    pub fn should_keep_running(&self) -> bool {
        !self.window.should_close()
    }

    pub fn update<F>(&mut self, mut imgui_callback: F)
    where
        F: FnMut(&mut Ui, f64, &mut Data),
    {
        self.frame_index += 1;

        self.data.handle_input(&self.window, self.frametime);
        self.data
            .render(self.event_handler.wireframe)
            .expect("failed to render.");
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
        unsafe {
            gl::Flush();
        }
        self.event_handler.events.clear();
        self.process_glfw_events();

        self.event_handler.current_frame_time = unsafe { glfwGetTime() };
        self.frametime = self.event_handler.current_frame_time - self.event_handler.last_frame_time;
        self.event_handler.last_frame_time = self.event_handler.current_frame_time;
        if self.event_handler.current_frame_time - self.event_handler.last_tick > 1.0 {
            self.event_handler.last_tick = self.event_handler.current_frame_time;
            self.data.update().expect("Failed to update shaders.");
        }
        // let frame_rate = 1.0 / self.frametime;
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
                WindowEvent::FramebufferSize(width, height) => unsafe {
                    gl::Viewport(0, 0, width, height);
                    self.size = [width as usize, height as usize];
                    self.data
                        .camera
                        .update_projection((width as f32) / (height as f32))
                },
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
                WindowEvent::Key(Key::F11, _, Action::Press, _) => {
                    let mut fullscreen = false;
                    self.window.with_window_mode(|wm| {
                        if let glfw::WindowMode::FullScreen(_) = wm {
                            fullscreen = true
                        }
                    });
                    match fullscreen {
                        false => self.glfw.with_primary_monitor(|_, m| {
                            let monitor = m.unwrap();
                            let workarea = monitor.get_workarea();
                            self.window.set_monitor(
                                glfw::WindowMode::FullScreen(monitor),
                                0,
                                0,
                                workarea.2 as u32,
                                workarea.3 as u32,
                                None,
                            );
                        }),
                        true => {
                            self.window.set_monitor(
                                glfw::WindowMode::Windowed,
                                250,
                                250,
                                WIDTH as u32,
                                HEIGHT as u32,
                                None,
                            );
                        }
                    }
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
    pub fn get_time(&mut self) -> f64 {
        self.glfw.get_time()
    }
}

pub struct EventHandler {
    wireframe: bool,
    current_frame_time: f64,
    pub last_frame_time: f64,
    imgui: Option<imgui::Context>,
    imgui_glfw: Option<imgui_glfw_rs::ImguiGLFW>,
    show_imgui: bool,
    last_tick: f64,
    pub events: Vec<WindowEvent>,
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
            last_tick: 0.0,
            events: Vec::new(),
        }
    }
}

fn init_gflw(window_name: &str) -> (Glfw, PWindow, GlfwReceiver<(f64, WindowEvent)>) {
    use glfw::fail_on_errors;
    let mut glfw = glfw::init(fail_on_errors!()).unwrap();

    let (mut window, events) = glfw
        .create_window(
            WIDTH as u32,
            HEIGHT as u32,
            window_name,
            glfw::WindowMode::Windowed,
        )
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
    glfw.set_swap_interval(SwapInterval::Sync(1));

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
