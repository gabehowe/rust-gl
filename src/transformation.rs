extern crate proc_macro;
use std::mem::size_of;
use std::ptr::null;

use crate::util::find_gl_error;
use cgmath::{perspective, Array, Deg, EuclideanSpace, Euler, Matrix, Matrix4, One, Point3, Rad, SquareMatrix, Vector2, Vector3, Zero};
use gl::types::{GLsizeiptr, GLuint};
use gl::{STATIC_DRAW, UNIFORM_BUFFER};
use imgui::sys::cty::c_double;

#[macro_export]
macro_rules! derive_transformable {
    ($obj:ty) => {
        impl Transformable for $obj {
            fn scale(&mut self, x: f32, y: f32, z: f32) {
                self.transform.scale(x, y, z);
            }
            fn uniform_scale(&mut self, scale: f32) {
                self.transform.uniform_scale(scale);
            }
            fn rotate(&mut self, x: f32, y: f32, z: f32) {
                self.transform.rotate(x, y, z);
            }
            fn translate(&mut self, x: f32, y: f32, z: f32) {
                self.transform.translate(x, y, z);
            }
            fn set_scale(&mut self, x: f32, y: f32, z: f32) {
                self.transform.set_scale(x, y, z);
            }
            fn set_rotation(&mut self, x: f32, y: f32, z: f32) {
                self.transform.set_rotation(x, y, z);
            }
            fn set_translation(&mut self, x: f32, y: f32, z: f32) {
                self.transform.set_translation(x, y, z);
            }
            fn set_uniform_scale(&mut self, scale: f32) {
                self.transform.set_uniform_scale(scale);
            }

        }
    };
}

#[derive(Clone, Debug)]
pub struct Transform {
    pub position: Vector3<f32>,
    pub rotation: Vector3<f32>,
    pub scale: Vector3<f32>,
}

impl Transform {
    pub fn new() -> Self {
        Transform {
            position: Vector3::zero(),
            rotation: Vector3::zero(),
            scale: Vector3::new(1.0, 1.0, 1.0),
        }
    }

    pub fn with_position(position: Vector3<f32>) -> Self {
        Transform {
            position,
            rotation: Vector3::zero(),
            scale: Vector3::new(1.0, 1.0, 1.0),
        }
    }

    pub fn with_scale(scale: Vector3<f32>) -> Self {
        Transform {
            position: Vector3::zero(),
            rotation: Vector3::zero(),
            scale,
        }
    }

    pub fn mat(&self) -> Matrix4<f32> {
        let translation =Matrix4::from_translation(self.position);
        let scale = Matrix4::from_nonuniform_scale(self.scale.x, self.scale.y, self.scale.z);
        let rotation = Matrix4::from(Euler::new(
                Rad(self.rotation.x),
                Rad(self.rotation.z),
                Rad(self.rotation.y),
            ));
        translation * rotation * scale
    }
}

impl Default for Transform {
    fn default() -> Self {
        Self::new()
    }
}

impl Transformable for Transform {
    fn scale(&mut self, x: f32, y: f32, z: f32) {
        self.scale.x *= x;
        self.scale.y *= y;
        self.scale.z *= z;
    }

    fn uniform_scale(&mut self, scale: f32) {
        self.scale *= scale;
    }

    fn rotate(&mut self, x: f32, y: f32, z: f32) {
        self.rotation += Vector3::new(x, y, z);
    }

    fn translate(&mut self, x: f32, y: f32, z: f32) {
        self.position += Vector3::new(x, y, z);
    }

    fn set_scale(&mut self, x: f32, y: f32, z: f32) {
        self.scale.x = x;
        self.scale.y = y;
        self.scale.z = z;
    }
    fn set_uniform_scale(&mut self, scale: f32) {
        self.scale.x = scale;
        self.scale.y = scale;
        self.scale.z = scale;
    }
    fn set_rotation(&mut self, x: f32, y: f32, z: f32) {
        self.rotation.x = x;
        self.rotation.y = y;
        self.rotation.z = z;
    }
    fn set_translation(&mut self, x: f32, y: f32, z: f32) {
        self.position.x = x;
        self.position.y = y;
        self.position.z = z;
    }
}

/*pub trait Transformation {
    fn scale(&mut self, x: f32, y: f32, z: f32) -> Matrix4<f32>;
    fn uniform_scale(&mut self, scale: f32) -> Matrix4<f32>;
    fn rotate(&mut self, x: f32, y: f32, z: f32) -> Matrix4<f32>;
    fn translate(&mut self, x: f32, y: f32, z: f32) -> Matrix4<f32>;
}
*/
pub trait Transformable {
    fn scale(&mut self, x: f32, y: f32, z: f32);
    fn uniform_scale(&mut self, scale: f32);
    fn rotate(&mut self, x: f32, y: f32, z: f32);
    fn translate(&mut self, x: f32, y: f32, z: f32);
    fn set_scale(&mut self, x: f32, y: f32, z: f32);
    fn set_uniform_scale(&mut self, scale: f32);
    fn set_rotation(&mut self, x: f32, y: f32, z: f32);
    fn set_translation(&mut self, x: f32, y: f32, z: f32);
}

/*impl Transformation for Matrix4<f32> {
    fn scale(&mut self, x: f32, y: f32, z: f32) -> Matrix4<f32> {
        *self * Matrix4::from_nonuniform_scale(x, y, z)
    }
    fn uniform_scale(&mut self, scale: f32) -> Matrix4<f32> {
        *self * Matrix4::from_scale(scale)
    }
    fn rotate(&mut self, x: f32, y: f32, z: f32) -> Matrix4<f32> {
        *self * Matrix4::from(Euler::new(Rad(x), Rad(y), Rad(z)))
    }
    fn translate(&mut self, x: f32, y: f32, z: f32) -> Matrix4<f32> {
        *self * Matrix4::from_translation(Vector3::new(x, y, z))
    }
}
*/
pub struct Camera {
    pub pos: Vector3<f32>,
    pub rot: Vector2<f32>,
    pub projection: Matrix4<f32>,
    uniform_buffer: GLuint,
    last_mouse: (f64, f64),
    pub(crate) front: Vector3<f32>,
    pub(crate) pitch: f32,
    pub(crate) yaw: f32,
}

impl Default for Camera {
    fn default() -> Self {
        Self::new()
    }
}

impl Camera {
    pub fn new() -> Camera {
        Camera {
            pos: Vector3::new(0f32, 0f32, 0f32),
            rot: Vector2::new(0.0, 0.00),
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
        }
    }
    /// # Safety
    /// ??
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
            2 * size_of::<Matrix4<f32>>() as GLsizeiptr,
        );
    }

    fn get_view_matrix(&mut self) -> Matrix4<f32> {
        self.update_vectors();
        Matrix4::look_at_rh(
            Point3::from_vec(self.pos),
            Point3::from_vec(self.pos + self.front),
            Vector3::unit_y(),
        )
    }
    pub fn update_projection(&mut self, aspect: f32) {
        self.projection = perspective(Deg(100.0), aspect, 0.01, 1000.0);
    }

    pub fn update_vectors(&mut self) {
        self.front = Vector3::new(
            self.pitch.cos() * self.yaw.cos(),
            self.pitch.sin(),
            self.pitch.cos() * self.yaw.sin(),
        );
    }

    pub fn update_buffers(&mut self) {
        let mut offset = 0;
        unsafe {
            gl::BindBuffer(UNIFORM_BUFFER, self.uniform_buffer);
            gl::BufferSubData(
                UNIFORM_BUFFER,
                offset,
                size_of::<Vector3<f32>>() as GLsizeiptr,
                self.pos.as_ptr().cast(),
            );
            offset += 16;
            gl::BufferSubData(
                UNIFORM_BUFFER,
                offset,
                size_of::<Matrix4<f32>>() as GLsizeiptr,
                self.get_view_matrix().as_ptr().cast(),
            );
            offset += size_of::<Matrix4<f32>>() as GLsizeiptr;
            gl::BufferSubData(
                UNIFORM_BUFFER,
                offset,
                size_of::<Matrix4<f32>>() as GLsizeiptr,
                self.projection.as_ptr().cast(),
            );
            gl::BindBuffer(UNIFORM_BUFFER, 0);
        }
        find_gl_error().unwrap();
        // println!("{:?}", std::mem::size_of::<Matrix4<f32>>());
        // gl::GetBufferSubData(UNIFORM_BUFFER, offset, (1 * std::mem::size_of::<Matrix4<f32>>()) as GLsizeiptr, transmute(&self.pos[0]));
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
        let sensitivity = 0.001;

        self.pitch -= delta_y as f32 * sensitivity;
        self.yaw += delta_x as f32 * sensitivity;

        self.update_vectors();
    }
    pub fn translate(&mut self, vector3: Vector3<f32>) {
        self.pos += vector3;
    }
}
