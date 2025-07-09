/*
Store vector objects
Render the vector objects
*/
#![feature(generic_const_exprs)]
#![allow(incomplete_features, unused)]

use crate::glutil::GLObject;
use crate::renderable::{InstancedObject, MeshData, Render, Renderable};
use crate::shader::{
    MaybeColorTexture, NarrowingMaterial, SetValue, Shader, ShaderManager, ShaderPtr,
};
use crate::transformation::{Transform, Transformable};
use crate::{derive_transformable, new_renderable_ptr, Engine, RenderablePtr};
use cgmath::num_traits::{AsPrimitive, Pow, ToPrimitive};
use cgmath::{vec3, InnerSpace, Matrix3, Matrix4, SquareMatrix, Vector2, Vector3};
use gl::types::{GLsizei, GLuint};
use itertools::Itertools;
use obj::Vertex;
use std::any::Any;
use std::cell::{RefCell, RefMut};
use std::cmp::min_by;
use std::error::Error;
use std::f32;
use std::f32::consts::PI;
use std::ffi::c_float;
use std::ptr::null;
use std::sync::Arc;

#[derive(Clone)]
pub struct Object2d {
    pub color: [f32; 4],
    pub transform: Transform,
}

impl Object2d {
    pub fn new(color: [f32; 4], transform: Transform) -> Self {
        Self { color, transform }
    }
}
derive_transformable!(Object2d);

const verts: [Vector3<f32>; 4] = [
    vec3(0.0, 0.0, 0.0),
    vec3(1.0, 0.0, 0.0),
    vec3(0.0, 1.0, 0.0),
    vec3(1.0, 1.0, 0.0)
];
pub struct Draw {
    rectangles: Vec<Object2d>,
    circles: Vec<Object2d>,
    size: Vector2<u32>,
    shader: ShaderPtr,
    rectangle_mesh: RenderablePtr,
    circle_mesh: RenderablePtr,
}
const CIRCLE_RESOLUTION: usize = 50;

impl Draw {
    pub fn new(width: usize, height: usize, engine: &mut Engine) -> Draw {
        // Create primitives for each of the shapes we want to draw.
        let shader = engine.data.shader_manager.register(
            Shader::from_source(
                include_str!("../shaders/drawing_shader.vert"),
                include_str!("../shaders/drawing_shader.frag"),
                "",
            )
                .unwrap(),
        );
        let mut rectangle = MeshData::new(
            verts.to_vec(),
            vec![0, 1, 3, 2],
            None,
            None,
        );
        let mut points: Vec<Vector3<f32>> = Vec::new();
        for i in 1..CIRCLE_RESOLUTION {
            let angle = f32::consts::TAU * (i as f32) / (CIRCLE_RESOLUTION as f32);
            let mut vect = vec3(angle.cos(), angle.sin(), 0.0);
            points.push(vect);
        }
        let mut circle = MeshData::new(
            points,
            (0..CIRCLE_RESOLUTION as u32).map(|i| i * 3).collect(),
            None,
            None,
        );
        let mut circle_instanced = Arc::from(RefCell::from(Box::from(InstancedObject::new(
            circle.vertices.clone(),
            circle.indices.clone(),
            None, // Default normals
            &shader,
            vec![],
            vec![],
        )) as Box<dyn Render>));
        engine.data.add_renderable_rc(&circle_instanced);

        let mut rectangle_instanced = InstancedObject::new(
            rectangle.vertices.clone(),
            rectangle.indices.clone(),
            None, // Default normals
            &shader,
            vec![],
            vec![]);
        let refr = Arc::from(RefCell::from(Box::from(rectangle_instanced) as Box<dyn Render>));
        engine.data.add_renderable_rc(&refr);
        Draw {
            rectangles: vec![],
            circles: vec![],
            size: Vector2::new(100, 100),
            shader,
            rectangle_mesh: refr,
            circle_mesh: circle_instanced,
        }
    }
    pub fn clear(&mut self) {
        self.rectangles.clear();
        self.circles.clear();
    }

    pub fn rectangle(&mut self, point1: Vector2<f32>, point2: Vector2<f32>, color: [f32; 4]) {
        let mut rect = Object2d::new(color, Transform::with_position(point1.extend(0.0)));
        // rect.rotate(-3.14159/2.0, 0.0, 0.0);
        // println!("{:?}", rect.transform.mat());
        self.add_object(rect, "rectangle");
    }
    pub fn fill(&mut self, color: [f32; 4]) {
        self.rectangle(
            Vector2::new(0.0, 0.0),
            self.size.map(|it| it.to_f32().unwrap()),
            color,
        );
    }
    pub fn line(&mut self, p1: Vector2<f32>, p2: Vector2<f32>, width: f32, color: [f32; 4]) {
        // let lpoint = if p1.x <= p2.x { p1 } else { p2 };
        // let rpoint = if lpoint == p1 { p2 } else { p1 };
        let lpoint = p2;
        let rpoint = p1;
        let slope = (rpoint.y - lpoint.y) / (rpoint.x - lpoint.x);
        let length = (p2 - p1).magnitude();
        let angle = (lpoint.y - rpoint.y).atan2(lpoint.x - rpoint.x);
        let perp_angle = (angle + PI / 2.0);
        let scaled_width = width * 0.01;

        let mut line = Object2d::new(color, Transform::with_position(lpoint.extend(0.0)));
        line.scale(length, scaled_width, 1.0);
        line.rotate(0.0, angle, 0.0);
        line.translate(rpoint.x - lpoint.x, rpoint.y - lpoint.y, 0.0);
        line.translate(perp_angle.cos() * -scaled_width/2.0, perp_angle.sin() * -scaled_width/2.0, 0.0);
        self.add_object(line, "rectangle")
    }
    pub fn circle(&mut self, center: Vector2<f32>, radius: f32, color: [f32; 4]) {
        let mut circle = Object2d::new(color, Transform::with_position(center.extend(0.0)));
        circle.set_uniform_scale(radius * 0.01);
        circle.set_translation(center.x, center.y, 0.0);
        self.add_object(circle, "circle");
    }
    pub fn update(&mut self) -> Result<(), Box<dyn Error>> {
        // Render rectangles using instanced rendering
        if !self.rectangles.is_empty() {
            let transforms: Vec<Transform> = self
                .rectangles
                .iter()
                .map(|r| r.transform.clone())
                .collect();
            let colors: Vec<[f32; 4]> = self.rectangles.iter().map(|r| r.color).collect();
            let mut bv = self.rectangle_mesh.try_borrow_mut()?;
            let vb = bv
                .as_any_mut()
                .downcast_mut::<InstancedObject>()
                .expect("couldn't downcast!");
            vb.set_data(transforms, colors);
            // vb.render(None)?;
        }

        // Render circles using instanced rendering
        if !self.circles.is_empty() {
            let transforms: Vec<Transform> =
                self.circles.iter().map(|c| c.transform.clone()).collect();
            let colors: Vec<[f32; 4]> = self.circles.iter().map(|c| c.color).collect();
            let mut vb = self.circle_mesh.borrow_mut();
            let mut vb = vb
                .as_any_mut()
                .downcast_mut::<InstancedObject>()
                .expect("couldn't downcast!");
            vb.set_data(transforms, colors);
            // vb.render(None)?; // TODO: remove this because it's immediately overwritten by the engine render function.
        }

        Ok(())
    }
    fn add_object(&mut self, object: Object2d, shape: &str) {
        match shape {
            "rectangle" => self.rectangles.push(object),
            "circle" => self.circles.push(object),
            _ => (),
        }
    }
}
