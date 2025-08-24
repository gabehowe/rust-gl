//! # Rust OpenGL Engine
//!
//! This library provides a simple 3D rendering engine built with OpenGL.
//! It handles window creation, rendering, camera controls, and basic UI integration.
#![warn(
    clippy::all,
    // clippy::restriction,
    clippy::pedantic,
    clippy::nursery,
    clippy::cargo,
)]
#![allow(clippy::missing_errors_doc, clippy::cast_precision_loss, clippy::cast_possible_truncation, clippy::cast_sign_loss, clippy::cast_possible_wrap)]
#![feature(const_vec_string_slice)]
extern crate alloc;
extern crate gl;

use alloc::rc::Rc;
// Public re-exports
pub use cgmath;
pub use glfw;
pub use imgui;

// Standard library imports
use core::cell::RefCell;
use core::error::Error;
use core::ptr;

// Graphics and rendering imports
use cgmath::{InnerSpace, Vector3};
use gl::{
    COLOR_BUFFER_BIT, DEBUG_OUTPUT, DEBUG_OUTPUT_SYNCHRONOUS, DEBUG_SEVERITY_NOTIFICATION,
    DEBUG_SOURCE_API, DEBUG_TYPE_ERROR, DEPTH_BUFFER_BIT, DEPTH_TEST, FILL, FRONT_AND_BACK,
};
use glfw::ffi::{glfwGetTime, glfwSetInputMode, CURSOR, CURSOR_DISABLED};
use glfw::{
    fail_on_errors, Action, Context, CursorMode, Glfw, GlfwReceiver, Key, PWindow, SwapInterval,
    WindowEvent, WindowHint,
};
use image::{ImageBuffer, Rgba};
use imgui::Ui;

// Module declarations
pub mod drawing;
mod glutil;
pub mod renderable;
pub mod shader;
pub mod transformation;
pub mod util;

// Internal module imports
use crate::shader::ShaderPtr;
use renderable::{Render, Renderable};
use shader::{MaybeColorTexture, NarrowingMaterial, ShaderManager};
use transformation::Camera;
use util::debug_log;

//
// Constants
//

pub const HEIGHT: usize = 1000;
#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss, clippy::cast_precision_loss)]
pub const WIDTH: usize = (HEIGHT as f32 * (16.0 / 9.0)) as usize;
/// Camera movement speed multiplier
const MOVESPEED: f32 = 2.5;
/// Camera rotation speed multiplier
const ROTATIONSPEED: f32 = 2.5;
/// Default clear color for rendering (RGBA)
pub(crate) const CLEARCOLOR: (f32, f32, f32, f32) = (0.1, 0.0, 0.0, 1.0);

//
// Type definitions and helpers
//

/// A reference-counted, mutable renderable object
type RenderablePtr = Rc<RefCell<Box<dyn Render>>>;

/// Creates a new reference-counted renderable pointer
fn new_renderable_ptr<T: Render + 'static>(renderable: T) -> RenderablePtr {
    Rc::new(RefCell::new(Box::new(renderable)))
}

//
// Core structures
//

/// Contains all the rendering data and scene objects
///
/// This structure holds the collection of renderable objects, camera,
/// shaders, and other rendering state information.
pub struct Data {
    /// Collection of objects to render in the scene
    pub renderables: Vec<RenderablePtr>,
    /// Camera for viewing the scene
    pub camera: Camera,
    /// Shader used for wireframe rendering
    wireframe_shader: ShaderPtr,
    /// Manager for all shaders in the scene
    pub shader_manager: ShaderManager,
    /// Optional framebuffer texture for rendering to texture
    pub frame_buffer_texture: Option<(u32, u32)>,
    /// Whether to clear the screen before rendering
    pub should_clear: bool,
}

/// Implementation of the Data structure
impl Data {
    /// Updates shader state
    ///
    /// Calls the shader manager's update method to refresh shaders if needed.
    /// # Errors
    /// Returns an error if the shader manager fails to update.
    fn update(&mut self) -> Result<(), Box<dyn Error>> {
        self.shader_manager.update()
    }

    /// Renders all objects in the scene
    ///
    /// Clears the screen if needed, updates camera buffers, and renders each object.
    /// If wireframe is true, uses the wireframe shader instead of the object's shader.
    /// # Errors
    /// Returns an error if any renderable fails to render.
    fn render(&mut self, wireframe: bool, clear_color: (f32, f32, f32, f32)) -> Result<(), Box<dyn Error>> {
        // Safety: We know that the key is a valid key because we are using the glfw::Key enum.
        unsafe {
            if self.should_clear {
                gl::ClearColor(clear_color.0, clear_color.1, clear_color.2, clear_color.3);
                gl::Clear(COLOR_BUFFER_BIT | DEPTH_BUFFER_BIT);
            }
        }
        self.camera.update_buffers()?; // Only needs to be updated if it changes. TODO: Optimization?
        for i in self.renderables.iter_mut().map(|x| x.try_borrow_mut()) {
            i?.render(wireframe.then(|| self.wireframe_shader.clone()))?;
        }
        Ok(())
    }

    /// Adds a renderable object to the scene
    ///
    /// Takes ownership of the renderable and returns a reference-counted pointer to it.
    /// # Errors
    /// Returns an error if the renderable cannot be added (e.g., if it fails to borrow).
    pub fn add_renderable(
        &mut self,
        renderable: Box<dyn Render>,
    ) -> Result<RenderablePtr, Box<dyn Error>> {
        let arc = Rc::new(RefCell::new(renderable));
        self.renderables.push(arc.clone());
        Ok(arc.clone())
    }

    /// Adds a reference to an existing renderable to the scene
    pub fn add_renderable_rc(&mut self, rc: &RenderablePtr) {
        self.renderables.push(rc.clone());
    }

    /// Creates a renderable from an OBJ file and adds it to the scene
    ///
    /// Loads the model from the specified path and applies the shader from shaderpath.
    /// # Errors
    /// Returns an error if the OBJ file cannot be loaded or the shader fails to compile.
    pub fn add_renderable_from_obj(
        &mut self,
        path: &str,
        shaderpath: &str,
    ) -> Result<RenderablePtr, Box<dyn Error>> {
        let renderable = Renderable::from_obj(path, shaderpath, &mut self.shader_manager)?;
        self.add_renderable(Box::from(renderable))
    }

    /// Processes keyboard input for camera movement
    ///
    /// Handles WASD for movement, QE for up/down, and arrow keys for rotation.
    /// Movement speed is affected by frametime for consistent movement regardless of framerate.
    fn handle_input(&mut self, window: &PWindow, frametime: f32) {
        let mut speed = MOVESPEED * frametime;
        let rot_speed = ROTATIONSPEED * frametime;
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

    /// Gets a reference to a renderable by index
    #[must_use]
    pub fn get_renderable(&self, index: usize) -> &RenderablePtr {
        &self.renderables[index]
    }

    /// Gets a mutable reference to a renderable by index
    pub fn get_renderable_mut(&mut self, index: usize) -> &mut RenderablePtr {
        &mut self.renderables[index]
    }

    /// Creates a framebuffer texture for rendering to texture
    ///
    /// This allows rendering the scene to a texture that can be used elsewhere.
    #[allow(clippy::cast_possible_wrap, clippy::cast_possible_truncation)]
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

/// Main engine class that manages the rendering loop and window
///
/// The Engine is responsible for initializing OpenGL, creating the window,
/// managing the render loop, and handling input events.
pub struct Engine {
    /// GLFW instance for window management
    glfw: Glfw,
    /// Main application window
    window: PWindow,
    /// Time elapsed since the last frame in seconds
    pub frametime: f64,
    /// Rendering data including scene objects and camera
    pub data: Data,
    /// Handler for window and input events
    pub event_handler: EventHandler,
    /// Current frame number since engine start
    pub frame_index: u32,
    /// Current window size [width, height]
    pub size: [usize; 2],
    pub clear_color: (f32, f32,f32,f32)
}

/// Implementation of the Engine structure
impl Engine {
    /// Adds a renderable object to the scene
    ///
    /// Convenience method that delegates to `Data::add_renderable`
    /// # Errors
    /// Returns an error if the renderable cannot be added (e.g., if it fails to borrow).
    pub fn add_renderable(
        &mut self,
        renderable: Box<dyn Render>,
    ) -> Result<RenderablePtr, Box<dyn Error>> {
        self.data.add_renderable(renderable)
    }

    /// Adds a reference to an existing renderable to the scene
    ///
    /// Convenience method that delegates to `Data::add_renderable_rc`
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
            clear_color: CLEARCOLOR
        })
    }

    /// Captures the current frame and saves it to a file
    ///
    /// Reads the framebuffer pixels and saves them as an image at the specified path.
    /// # Panics
    /// If the image cannot be created or saved.
    #[allow(clippy::cast_sign_loss)]
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

    /// Sets the cursor mode (normal, hidden, disabled)
    pub fn set_cursor_mode(&mut self, cursor_mode: CursorMode) {
        self.window.set_cursor_mode(cursor_mode);
    }

    /// Checks if the engine should continue running
    ///
    /// Returns false if the window should close.
    pub fn should_keep_running(&self) -> bool {
        !self.window.should_close()
    }

    /// Performs a single frame update
    ///
    /// Handles input, renders the scene, updates `ImGui` if enabled,
    /// processes events, and calculates frame time.
    ///
    /// The `imgui_callback` parameter is a function that will be called to build the UI.
    /// # Panics
    /// When rendering fails or if `ImGui` is enabled but not properly initialized.
    #[allow(clippy::cast_possible_truncation)]
    pub fn update<F>(&mut self, mut imgui_callback: F)
    where
        F: FnMut(&mut Ui, f64, &mut Data),
    {
        self.frame_index += 1;
        self.event_handler.current_frame_time = self.get_time();

        self.data.handle_input(&self.window, self.frametime as f32);
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
        self.process_glfw_events();
        self.window.swap_buffers();
        unsafe {
            gl::Flush();
        }

        self.frametime = self.get_time() - self.event_handler.current_frame_time;
        self.event_handler.last_frame_time = self.event_handler.current_frame_time;
        if self.event_handler.current_frame_time - self.event_handler.last_tick > 1.0 {
            self.event_handler.last_tick = self.event_handler.current_frame_time;
            self.data.update().expect("Failed to update shaders.");
        }
        // let frame_rate = 1.0 / self.frametime;
        // println!("Frame rate: {}", frame_rate);
    }

    /// Gets the current GLFW time in seconds
    pub fn get_time(&mut self) -> f64 {
        self.glfw.get_time()
    }

    /// Gets the current cursor position
    ///
    /// Returns (x, y) coordinates relative to the window.
    pub fn get_cursor_pos(&self) -> (f64, f64) {
        self.window.get_cursor_pos()
    }
    /// Creates a new Engine instance
    ///
    /// Initializes GLFW, OpenGL, and creates a window with the given name.
    /// If imgui is true, initializes the `ImGui` UI system.
    /// # Errors
    /// Returns an error if GLFW initialization fails or if the shader cannot be created.
    pub fn new(imgui: bool, window_name: &str) -> Result<Self, Box<dyn Error>> {
        let (glfw, mut window, events) = init_gflw(window_name);
        let camera = Self::init_gl();
        let event_handler = if imgui {
            EventHandler::new(&mut window, events)
        } else {
            EventHandler::raw(events)
        };
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
        Ok(Self {
            glfw,
            window,
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

    /// Initializes OpenGL settings and creates a camera
    ///
    /// Sets up blending, texturing, and other OpenGL state.
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
        self.event_handler.current_frame_time = self.get_time();

        self.data.handle_input(&self.window, self.frametime);
        self.data
            .render(self.event_handler.wireframe, self.clear_color)
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

        self.event_handler.events.clear();
        self.process_glfw_events();
        self.window.swap_buffers();
        unsafe {
            gl::Flush();
        }

        self.frametime = self.get_time() - self.event_handler.current_frame_time ;
        self.event_handler.last_frame_time = self.event_handler.current_frame_time;
        if self.event_handler.current_frame_time - self.event_handler.last_tick > 1.0 {
            self.event_handler.last_tick = self.event_handler.current_frame_time;
            self.data.update().expect("Failed to update shaders.");
        }
        // let frame_rate = 1.0 / self.frametime;
        // println!("Frame rate: {}", frame_rate);
    }

    /// Processes all pending GLFW events
    ///
    /// Handles window events like resizing, key presses, and cursor movement.
    /// Updates the engine state based on these events.
    /// # Panics
    /// If the `ImGui` context is not initialized when it's enabled.
    #[allow(clippy::cast_sign_loss, clippy::cast_precision_loss, clippy::cast_possible_truncation)]
    fn process_glfw_events(&mut self) {
        self.glfw.poll_events();
        for (_, event) in glfw::flush_messages(&self.event_handler.events) {
            // Forward events to ImGui if it's enabled
            if self.event_handler.imgui.is_some() && self.event_handler.show_imgui {
                self.event_handler
                    .imgui_glfw
                    .as_mut()
                    .unwrap()
                    .handle_event(self.event_handler.imgui.as_mut().unwrap(), &event);
            }

            // Handle specific window events
            match event {
                // Handle window resize
                WindowEvent::FramebufferSize(width, height) => unsafe {
                    gl::Viewport(0, 0, width, height);
                    self.size = [width as usize, height as usize];
                    self.data
                        .camera
                        .update_projection((width as f32) / (height as f32));
                },
                // Exit on Escape key
                WindowEvent::Key(Key::Escape, _, Action::Press, _) => {
                    self.window.set_should_close(true);
                }
                // Toggle cursor lock with K key
                WindowEvent::Key(Key::K, _, Action::Press, _) => {
                    if self.window.get_cursor_mode() == CursorMode::Disabled {
                        self.window.set_cursor_mode(CursorMode::Normal);
                    } else {
                        self.window.set_cursor_mode(CursorMode::Disabled);
                    }
                    self.window.make_current();
                }
                // Take screenshot with F12
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
                // Toggle ImGui visibility with F3
                WindowEvent::Key(Key::F3, _, Action::Press, _) => {
                    self.event_handler.show_imgui = !self.event_handler.show_imgui;
                }
                // Toggle wireframe mode with F2
                WindowEvent::Key(Key::F2, _, Action::Press, _) => {
                    unsafe {
                        if self.event_handler.wireframe {
                            gl::PolygonMode(FRONT_AND_BACK, FILL);
                        } else {
                            gl::PolygonMode(FRONT_AND_BACK, gl::LINE);
                        }
                    }
                    println!("Wireframe: {}", self.event_handler.wireframe);
                    self.event_handler.wireframe = !self.event_handler.wireframe;
                }
                // Toggle fullscreen with F11
                WindowEvent::Key(Key::F11, _, Action::Press, _) => {
                    let mut fullscreen = false;
                    self.window.with_window_mode(|wm| {
                        if let glfw::WindowMode::FullScreen(_) = wm {
                            fullscreen = true;
                        }
                    });
                    if fullscreen {
                        self.window.set_monitor(
                            glfw::WindowMode::Windowed,
                            250,
                            250,
                            WIDTH as u32,
                            HEIGHT as u32,
                            None,
                        );
                    } else {
                        self.glfw.with_primary_monitor(|_, m| {
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
                        });
                    }
                }
                // Handle mouse movement for camera control
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

/// Handles window events and input processing
///
/// This structure manages the event loop, processes input events,
/// and handles `ImGui` integration.
pub struct EventHandler {
    /// Whether wireframe rendering is enabled
    wireframe: bool,
    /// Time at the start of the current frame
    current_frame_time: f64,
    /// Time at the start of the previous frame
    pub last_frame_time: f64,
    /// Optional `ImGui` context for UI rendering
    imgui: Option<imgui::Context>,
    /// Optional ImGui-GLFW integration
    imgui_glfw: Option<imgui_glfw_rs::ImguiGLFW>,
    /// Whether to display the `ImGui` interface
    show_imgui: bool,
    /// Time of the last periodic update
    last_tick: f64,
    /// Receiver for GLFW window events
    events: GlfwReceiver<(f64, WindowEvent)>,
}
/// Implementation of the `EventHandler` structure
impl EventHandler {
    /// Creates a new `EventHandler` with `ImGui` support
    ///
    /// Initializes `ImGui` context and GLFW integration.
    #[allow(clippy::cast_sign_loss, clippy::cast_precision_loss)]
    fn new(window: &mut PWindow, events: GlfwReceiver<(f64, WindowEvent)>) -> Self {
        // let mut renderer = imgui_opengl_renderer::Renderer::new(&mut ctx, |s| self.window.get_proc_address(s) as _);

        let mut imgui = imgui::Context::create();
        let imgui_glfw = imgui_glfw_rs::ImguiGLFW::new(&mut imgui, window);
        let window_size = window.get_size();
        println!("Window Size: {window_size:?}");
        imgui.io_mut().display_size = [window_size.0 as f32, window_size.1 as f32];
        Self {
            last_frame_time: unsafe { glfwGetTime() },
            imgui: Some(imgui),
            imgui_glfw: Some(imgui_glfw),
            ..Self::raw(events)
        }
    }

    /// Creates a new `EventHandler` without `ImGui` support
    ///
    /// Used when `ImGui` is not needed for the application.
    const fn raw(events: GlfwReceiver<(f64, WindowEvent)>) -> Self {
        Self {
            wireframe: false,
            current_frame_time: 0.0,
            last_frame_time: 0.0,
            imgui: None,
            imgui_glfw: None,
            show_imgui: true,
            last_tick: 0.0,
            events,
        }
    }
}

/// Initializes GLFW and creates a window with OpenGL context
///
/// This function:
/// 1. Initializes GLFW
/// 2. Creates a window with the specified name
/// 3. Sets up OpenGL context with version 4.6 core profile
/// 4. Configures window event callbacks
/// 5. Sets up OpenGL debug output
/// 6. Enables depth testing and other OpenGL features
///
/// Returns a tuple containing:
/// - The GLFW instance
/// - The window
/// - A receiver for window events
/// # Panics
/// If GLFW fails to initialize or if the window cannot be created.
#[allow(clippy::cast_possible_truncation)]
fn init_gflw(window_name: &str) -> (Glfw, PWindow, GlfwReceiver<(f64, WindowEvent)>) {
    use glfw::fail_on_errors;
    // Initialize GLFW
    let mut glfw = glfw::init(fail_on_errors!()).unwrap();

    // Create a window
    let (mut window, events) = glfw
        .create_window(
            WIDTH as u32,
            HEIGHT as u32,
            window_name,
            glfw::WindowMode::Windowed,
        )
        .expect("Failed to create GLFW window.");

    // Set OpenGL context hints
    glfw.window_hint(WindowHint::ContextVersionMajor(4));
    glfw.window_hint(WindowHint::ContextVersionMinor(6));
    glfw.window_hint(WindowHint::OpenGlProfile(glfw::OpenGlProfileHint::Core));
    glfw.window_hint(WindowHint::OpenGlDebugContext(true));

    // Set up window
    window.make_current();
    window.set_framebuffer_size_polling(true);
    window.set_key_polling(true);
    window.set_char_polling(true);
    window.set_cursor_pos_polling(true);
    window.set_mouse_button_polling(true);
    window.set_scroll_polling(true);
    glfw.set_swap_interval(SwapInterval::Sync(1)); // Enable VSync

    // Load OpenGL functions
    gl::load_with(|s| glfw.get_proc_address_raw(s));

    // Configure OpenGL
    unsafe {
        // Disable cursor initially for camera control
        glfwSetInputMode(window.window_ptr(), CURSOR, CURSOR_DISABLED);

        // Enable depth testing for 3D rendering
        gl::Enable(DEPTH_TEST);

        // Set up debug output
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
