use std::f32::consts::PI;
use std::mem::transmute;
use std::ptr::null;

use cgmath::{Basis3, Deg, EuclideanSpace, Euler, Matrix, Matrix3, Matrix4, One, perspective, Point3, Rad, Rotation, Vector2, Vector3};
use gl::{STATIC_DRAW, UNIFORM_BUFFER};
use gl::types::{GLsizeiptr, GLuint};
use imgui::sys::cty::c_double;

use crate::engine::renderable::Shader;


pub trait Transformation {
    fn scale(&mut self, x: f32, y: f32, z: f32) -> Matrix4<f32>;
    fn uniform_scale(&mut self, scale: f32) -> Matrix4<f32>;
    fn rotate(&mut self, x: f32, y: f32, z: f32) -> Matrix4<f32>;
    fn translate(&mut self, x: f32, y: f32, z: f32) -> Matrix4<f32>;
}

impl Transformation for Matrix4<f32> {
    fn scale(&mut self, x: f32, y: f32, z: f32) -> Matrix4<f32> {
        return &*self * Matrix4::from_nonuniform_scale(x, y, z);
    }
    fn uniform_scale(&mut self, scale: f32) -> Matrix4<f32> {
        return &*self * Matrix4::from_scale(scale);
    }
    fn rotate(&mut self, x: f32, y: f32, z: f32) -> Matrix4<f32> {
        return &*self * Matrix4::from(Euler::new(Rad(x), Rad(y), Rad(z)));
    }
    fn translate(&mut self, x: f32, y: f32, z: f32) -> Matrix4<f32> {
        return &*self * Matrix4::from_translation(Vector3::new(x, y, z));
    }
}

pub struct Camera {
    pub pos: Vector3<f32>,
    pub rot: Vector2<f32>,
    view: Matrix4<f32>,
    projection: Matrix4<f32>,
    uniform_buffer: GLuint,
    last_mouse: (f64, f64),
    pub(crate) front: Vector3<f32>,
    pub(crate) pitch: f32,
    pub(crate) yaw: f32,
}

impl Camera {
    pub fn new() -> Camera {
        let mut ret = Camera {
            pos: Vector3::new(0f32, 0f32, 0f32),
            rot: Vector2::new(0.0, 0.00),
            view: Matrix4::one(), // Matrix4::look_at_rh(
            //     Point3::new(1f32, 1f32, 0f32),
            //     Point3::new(0f32, 0f32, 0f32),
            //     vec3(0f32, 0f32, 1f32),
            // ),
            projection: perspective(Deg(100.0), 16. / 9., 0.01, 1000.0),
            uniform_buffer: 0,
            last_mouse: (-55.5f64, 55.5f64),
            front: Vector3::new(0f32, 0f32, 1f32),
            pitch: 0.0,
            yaw: 0.0,
        };

        return ret;
    }

    pub unsafe fn initialize_buffers(&mut self) {
        gl::GenBuffers(1, &mut self.uniform_buffer);
        gl::BindBuffer(UNIFORM_BUFFER, self.uniform_buffer);
        gl::BufferData(UNIFORM_BUFFER, 16 + 64 * 2, null(), STATIC_DRAW); // 2 * mat4
        gl::BindBuffer(UNIFORM_BUFFER, 0); // release the buffer

        gl::BindBufferRange(
            UNIFORM_BUFFER,
            0,
            self.uniform_buffer,
            0,
            2 * (std::mem::size_of::<Matrix4<f32>> as GLsizeiptr),
        );
    }

    fn get_view_matrix(&mut self) -> Matrix4<f32> {
        self.update_vectors();
        return Matrix4::look_at_rh(Point3::from_vec(self.pos), Point3::from_vec(self.pos + self.front), Vector3::unit_y());
    }

    pub fn update_vectors(&mut self) {
        // self.front = Basis3::from(Euler::new(Rad(self.rot.x), Rad(self.rot.y), Rad(0.))).rotate_vector(Vector3::unit_z());

        self.front = Vector3::new(
            self.pitch.cos() * self.yaw.cos(),
            self.pitch.sin(),
            self.pitch.cos() * self.yaw.sin(),
        );
    }

    pub unsafe fn update_buffers(&mut self) {
        let mut offset = 0;
        gl::BindBuffer(UNIFORM_BUFFER, self.uniform_buffer);
        gl::BufferSubData(
            UNIFORM_BUFFER,
            offset,
            (1 * std::mem::size_of::<Vector3<f32>>()) as GLsizeiptr,
            transmute(&self.pos[0]),
        );
        offset += 16;
        gl::BindBuffer(UNIFORM_BUFFER, self.uniform_buffer);
        gl::BufferSubData(
            UNIFORM_BUFFER,
            offset,
            (1 * std::mem::size_of::<Matrix4<f32>>()) as GLsizeiptr,
            (self.get_view_matrix()).as_ptr().cast(),
        );
        gl::BindBuffer(UNIFORM_BUFFER, 0);
        offset += (1 * std::mem::size_of::<Matrix4<f32>>()) as GLsizeiptr;
        gl::BindBuffer(UNIFORM_BUFFER, self.uniform_buffer);
        gl::BufferSubData(
            UNIFORM_BUFFER,
            offset,
            (1 * std::mem::size_of::<Matrix4<f32>>()) as GLsizeiptr,
            (self.projection).as_ptr().cast(),
        );
        // println!("{:?}", std::mem::size_of::<Matrix4<f32>>());
        // gl::GetBufferSubData(UNIFORM_BUFFER, offset, (1 * std::mem::size_of::<Matrix4<f32>>()) as GLsizeiptr, transmute(&self.pos[0]));
        gl::BindBuffer(UNIFORM_BUFFER, 0);
        // println!("{:?} {:?}", self.view.x, self.projection.x);
        // println!("{:?} {:?}", self.view.y, self.projection.y);
        // println!("{:?} {:?}", self.view.z, self.projection.z);
        // println!("{:?} {:?}", self.view.w, self.projection.w);

    }

    pub fn handle_mouse(&mut self, x: c_double, y: c_double) {
        if self.last_mouse.0 == -55.5 && self.last_mouse.1 == 55.5 {
            self.last_mouse = (x, y);
            return;
        }
        // println!("{} {}", self.last_mouse.0, self.last_mouse.1);
        let delta_x = x - self.last_mouse.0;
        let delta_y = y - self.last_mouse.1;
        // println!("{} {}", delta_x, delta_y);
        self.last_mouse = (x, y);
        let sensitivity = 0.0001;

        self.pitch -= delta_y as f32 * sensitivity;
        self.yaw -= delta_x as f32 * sensitivity;

        self.front = Vector3::new(
            self.pitch.cos() * self.yaw.cos(),
            self.pitch.sin(),
            self.pitch.cos() * self.yaw.sin(),
        );
    }
    pub fn translate(&mut self, vector3: Vector3<f32>) {
        self.pos += vector3;
    }
}

