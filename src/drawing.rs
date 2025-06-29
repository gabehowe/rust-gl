/*
Store vector objects
Render the vector objects
*/
#![feature(generic_const_exprs)]
#![allow(incomplete_features, unused)]

use crate::renderable::{MeshData, Render, Renderable, InstancedObject};
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
use std::cell::RefCell;
use std::cmp::min_by;
use std::error::Error;
use std::f32;
use std::ffi::c_float;
use std::ptr::null;
use std::sync::Arc;
use crate::glutil::GLObject;

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

pub struct Draw {
    rectangles: Vec<Object2d>,
    circles: Vec<Object2d>,
    size: Vector2<u32>,
    shader: ShaderPtr,
    rectangle_mesh: MeshData,
    circle_mesh: MeshData,
}
const CIRCLE_RESOLUTION: usize = 50;

impl Draw {
    pub fn new(width: usize, height: usize, engine: &mut Engine) -> Draw {
        // Create primitives for each of the shapes we want to draw.
        let mut rectangle = MeshData::new(
            vec![
                vec3(0.0, 0.0, 0.0),
                vec3(0.0, 1.0, 0.0),
                vec3(1.0, 1.0, 0.0),
                vec3(1.0, 0.0, 0.0),
            ],
            vec![0, 1, 2, 3, 0],
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
        rectangle.init();
        circle.init();
        Draw {
            rectangles: vec![],
            circles: vec![],
            size: Vector2::new(100, 100),
            shader: engine.data.shader_manager.register(
                Shader::from_source(
                    include_str!("../shaders/drawing_shader.vert"),
                    include_str!("../shaders/drawing_shader.frag"),
                    "",
                )
                .unwrap(),
            ),
            rectangle_mesh: rectangle,
            circle_mesh: circle,
        }
    }
    pub fn clear(&mut self) {
        self.rectangles.clear();
        self.circles.clear();
    }
    fn add_object(&mut self, object: Object2d, shape: &str) {
        match shape {
            "rectangle" => self.rectangles.push(object),
            "circle" => self.circles.push(object),
            _ => (),
        }
    }
    pub fn rectangle(
        &mut self,
        point1: Vector2<f32>,
        point2: Vector2<f32>,
        color: [f32; 4],
    ) {
        let rect = Object2d::new(color, Transform::with_position(point1.extend(0.0)));
        self.add_object(rect, "rectangle");
    }
    pub fn fill(&mut self, color: [f32; 4]) {
        self.rectangle(
            Vector2::new(0.0, 0.0),
            self.size.map(|it| it.to_f32().unwrap()),
            color,
        );
    }
    pub fn line(
        &mut self,
        p1: Vector2<f32>,
        p2: Vector2<f32>,
        width: f32,
        color: [f32; 4],
    ) {
        let lpoint = if p1.x <= p2.x { p1 } else { p2 };
        let rpoint = if lpoint == p1 { p2 } else { p1 };
        let slope = (rpoint.y - lpoint.y) / (rpoint.x - lpoint.x);
        let length = (p2 - p1).magnitude();
        let angle = slope.atan();

        let mut line = Object2d::new(color, Transform::with_position(lpoint.extend(0.0)));
        line.scale(length, width * 0.01, 1.0);
        line.rotate(0.0, 0.0, angle);
        line.translate(lpoint.x, lpoint.y, 0.0);
        self.add_object(line, "rectangle")
    }
    pub fn circle(
        &mut self,
        center: Vector2<f32>,
        radius: f32,
        color: [f32; 4],
    ) {
        let mut circle = Object2d::new(color, Transform::with_position(center.extend(0.0)));
        circle.set_uniform_scale(radius * 0.01);
        circle.set_translation(center.x, center.y, 0.0);
        self.add_object(circle, "circle");
    }
    pub fn render(&mut self) -> Result<(), Box<dyn Error>> {
        // Render rectangles using instanced rendering
        if !self.rectangles.is_empty() {
            let transforms: Vec<Transform> = self.rectangles.iter().map(|r| r.transform.clone()).collect();
            let colors: Vec<[f32; 4]> = self.rectangles.iter().map(|r| r.color).collect();
            let mut rectangle_instanced = InstancedObject::new(
                self.rectangle_mesh.vertices.clone(),
                self.rectangle_mesh.indices.clone(),
                vec![Vector3::new(0.0, 0.0, 1.0); self.rectangle_mesh.vertices.len()], // Default normals
                &self.shader,
                transforms,
                colors,
            );
            rectangle_instanced.render()?;
        }
        
        // Render circles using instanced rendering
        if !self.circles.is_empty() {
            let transforms: Vec<Transform> = self.circles.iter().map(|c| c.transform.clone()).collect();
            let colors: Vec<[f32; 4]> = self.circles.iter().map(|c| c.color).collect();
            let mut circle_instanced = InstancedObject::new(
                self.circle_mesh.vertices.clone(),
                self.circle_mesh.indices.clone(),
                vec![Vector3::new(0.0, 0.0, 1.0); self.circle_mesh.vertices.len()], // Default normals
                &self.shader,
                transforms,
                colors,
            );
            // TODO: Fix the memory leak this will inevitably cause by repeatedly creating new objects.
            circle_instanced.render()?;
        }
        
        Ok(())
    }
}
