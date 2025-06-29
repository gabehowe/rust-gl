/*
Store vector objects
Render the vector objects
*/
#![feature(generic_const_exprs)]
#![allow(incomplete_features, unused)]

use std::cell::RefCell;
use crate::{derive_transformable, new_renderable_ptr, Engine, RenderablePtr};
use crate::renderable::{MeshData, Render, Renderable};
use crate::shader::{
    MaybeColorTexture, NarrowingMaterial, SetValue, Shader, ShaderManager, ShaderPtr,
};
use crate::transformation::{Transform, Transformable};
use cgmath::num_traits::{AsPrimitive, Pow, ToPrimitive};
use cgmath::{vec3, InnerSpace, Matrix3, Matrix4, SquareMatrix, Vector2, Vector3};
use gl::types::{GLsizei, GLuint};
use itertools::Itertools;
use obj::Vertex;
use std::cmp::min_by;
use std::error::Error;
use std::f32;
use std::ffi::c_float;
use std::ptr::null;
use std::sync::Arc;

derive_transformable!(Object2d);
pub struct Object2d { // TODO: make not public
    color: [f32; 4],
    pub transform: Transform,
    shader: ShaderPtr,
    mesh_data: Arc<MeshData>,
}

impl Render for Object2d {
    fn render(&mut self, shader_override: Option<ShaderPtr>) -> Result<(), Box<dyn Error>> {
        let mut shader = self.shader.try_borrow_mut()?;
        shader.use_();
        shader.set(self.transform.mat(), "model");
        shader.set(self.color.to_vec(), "color");
        unsafe {
            gl::BindVertexArray(self.mesh_data.vertex_array);
            gl::DrawElements(
                gl::TRIANGLE_FAN,
                self.mesh_data.indices.len() as GLsizei,
                gl::UNSIGNED_INT,
                null(),
            );
            gl::BindVertexArray(0);
        }
        Shader::clear_shader();
        Ok(())
    }
    fn is(&self) -> bool {
        todo!()
    }
    fn set_is(&mut self, val: bool) {
        todo!()
    }
}

pub struct Draw {
    objects: Vec<Arc<RefCell<Box<dyn Render>>>>, // TODO: Make not public
    size: Vector2<u32>,
    shader: ShaderPtr,
    primitives: Vec<Arc<MeshData>>
}
const CIRCLE_RESOLUTION: usize = 50;

impl Draw {
    pub fn new(width: usize, height: usize, engine: &mut Engine) -> Draw {
        // Create primitives for each of the shapes we want to draw.
        let mut rectangle = MeshData {
            vertices: vec![
                vec3(0.0, 0.0, 0.0),
                vec3(0.0, 1.0, 0.0),
                vec3(1.0, 1.0, 0.0),
                vec3(1.0, 0.0, 0.0),
            ],
            indices: vec![0, 1, 2, 3, 0],
            ..Default::default()
        };
        let mut points: Vec<Vector3<f32>> = Vec::new();
        for i in 0..CIRCLE_RESOLUTION {
            let angle = f32::consts::TAU * (i as f32) / (CIRCLE_RESOLUTION as f32);
            let mut vect = vec3(angle.cos(), angle.sin(), 0.0);
            points.push(vect);
        }
        let mut circle = MeshData {
            vertices: points,
            indices: (0..CIRCLE_RESOLUTION as u32).map(|i| i * 3).collect(),
            ..Default::default()
        };
        rectangle.init();
        circle.init();
        let mut primitives = vec![Arc::new(rectangle), Arc::new(circle)];
        Draw {
            objects: vec![],
            size: Vector2::new(100, 100),
            shader: engine.data.shader_manager.register(
                Shader::from_source(
                    include_str!("../shaders/drawing_shader.vert"),
                    include_str!("../shaders/drawing_shader.frag"),
                    "",
                )
                    .unwrap(),
            ),
            primitives
        }
    }
    pub fn clear(&mut self) {
        self.objects.clear();
    }
    fn add_object(&mut self, mut object: Object2d) -> Arc<RefCell<Box<dyn Render>>> {
        let rptr: Arc<RefCell<Box<dyn Render>>> = new_renderable_ptr(object);
        self.objects.push(rptr.clone());
        rptr
    }
    pub fn rectangle(&mut self, point1: Vector2<f32>, point2: Vector2<f32>, color: [f32; 4]) -> Arc<RefCell<Box<dyn Render>>> {
        let rect = Object2d {
            color,
            shader: self.shader.clone(),
            mesh_data: self.primitives[0].clone(),
            transform: Transform::with_position(point1.extend(0.0)),
        };
        self.add_object(rect)
    }
    pub fn fill(&mut self, color: [f32; 4]) {
        self.rectangle(
            Vector2::new(0.0, 0.0),
            self.size.map(|it| it.to_f32().unwrap()),
            color,
        );
    }
    pub fn line(&mut self, p1: Vector2<f32>, p2: Vector2<f32>, width: f32, color: [f32; 4]) -> Arc<RefCell<Box<dyn Render>>> {
        let lpoint = if p1.x <= p2.x { p1 } else { p2 };
        let rpoint = if lpoint == p1 { p2 } else { p1 };
        let slope = (rpoint.y - lpoint.y) / (rpoint.x - lpoint.x);
        let length = (p2 - p1).magnitude();
        let angle = slope.atan();

        let mut line = Object2d {
            color,
            shader: self.shader.clone(),
            mesh_data: self.primitives[0].clone(),
            transform: Transform::with_position(lpoint.extend(0.0)),
        };
        line.scale(length,  width * 0.01, 1.0);
        line.rotate(0.0, angle, 0.0);
        line.translate(lpoint.x, lpoint.y, 0.0);
        self.add_object(line)
    }
    pub fn circle(&mut self, center: Vector2<f32>, radius: f32, color: [f32; 4]) -> Arc<RefCell<Box<dyn Render>>> {
        let mut circle = Object2d {
            color,
            shader: self.shader.clone(),
            mesh_data: self.primitives[1].clone(),
            transform: Transform::with_position(center.extend(0.0)),
        };
        circle.set_uniform_scale(radius * 0.01);
        circle.set_translation(center.x, center.y, 0.0);
        self.add_object(circle)
    }
    pub fn render(&mut self) -> Result<(), Box<dyn Error>> {
        self.objects
            .iter_mut()
            .try_for_each(|object| object.borrow_mut().render(None))
    }
}
