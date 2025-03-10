use std::error::Error;
use std::ffi::{c_float, c_uint};
use std::fs::File;
use std::io::BufReader;
use std::mem::size_of;
use std::path::Path;
use std::ptr::null;

use crate::engine::shader::{FromVertex, SetValue, Shader};
use crate::engine::transformation::Transformation;
use cgmath::{Euler, Matrix, Matrix4, One, Rad, Vector2, Vector3, Zero};
use gl::types::{GLenum, GLfloat, GLsizei, GLuint};
use gl::{
    ARRAY_BUFFER, ELEMENT_ARRAY_BUFFER, FALSE, FLOAT, STATIC_DRAW
    , TRIANGLES, UNSIGNED_INT,
};
use obj::raw::{parse_mtl, parse_obj};
use obj::{FromRawVertex, TexturedVertex};

pub struct Renderable {
    pub(crate) vertices: Vec<Vector3<c_float>>,
    indices: Vec<c_uint>,
    pub shader: Shader,
    vertex_array: GLuint,
    vertex_buffer: GLuint,
    element_buffer: GLuint,
    pub rotation: Vector3<f32>,
    pub translation: Vector3<f32>,
    pub scale: Vector3<f32>,
    normals: Vec<Vector3<f32>>,
    tex_coords: Vec<Vector2<f32>>,
    pub draw_type: GLenum
}

impl Renderable {
    pub(crate) fn new_with_tex(
        vertices: Vec<Vector3<f32>>,
        indices: Vec<u32>,
        normals: Vec<Vector3<f32>>,
        tex_coords: Vec<Vector2<f32>>,
        shader: Shader,
    ) -> Renderable {
        Renderable {
            tex_coords,
            ..Renderable::new(vertices, indices, normals, shader)
        }
        // TODO: Should probably use Result here or smth.
    }

    pub(crate) fn new(
        vertices: Vec<Vector3<f32>>,
        indices: Vec<u32>,
        normals: Vec<Vector3<f32>>,
        shader: Shader,
    ) -> Renderable {
        let mut ret = Renderable {
            vertices,
            indices,
            shader,
            vertex_array: 0,
            vertex_buffer: 0,
            element_buffer: 0,
            rotation: Vector3::zero(),
            translation: Vector3::zero(),
            scale: Vector3::new(1., 1., 1.),
            normals,
            tex_coords: Vec::new(),
            draw_type: TRIANGLES
        };
        unsafe {
            ret.gen_buffers();

            gl::BindVertexArray(ret.vertex_array);

            ret.init_array_buffers();

            ret.gen_vertex_attrib_arrays();
            //
            // gl::EnableVertexAttribArray(1);
            // gl::VertexAttribPointer(1, 3, FLOAT, FALSE, (6 * size_of::<GLfloat>()) as GLsizei, (3 * size_of::<GLfloat>()) as *const _);

            gl::BindBuffer(ARRAY_BUFFER, 0);
            gl::BindVertexArray(0);
        }
        ret
    }
    unsafe fn gen_vertex_attrib_arrays(&mut self) {
        let mut stride = 2 * (3 * size_of::<GLfloat>()) as GLsizei;
        if !self.tex_coords.is_empty() {
            stride = (2 * (3 * size_of::<GLfloat>()) + (2 * size_of::<GLfloat>())) as GLsizei;
        }
        gl::VertexAttribPointer(0, 3, FLOAT, FALSE, stride, null());
        gl::EnableVertexAttribArray(0);
        gl::VertexAttribPointer(
            1,
            3,
            FLOAT,
            FALSE,
            stride,
            (3 * size_of::<GLfloat>()) as *const _,
        );
        gl::EnableVertexAttribArray(1);
        if !self.tex_coords.is_empty() {
            gl::VertexAttribPointer(
                2,
                2,
                FLOAT,
                FALSE,
                stride,
                (6 * size_of::<GLfloat>()) as *const _,
            );
            gl::EnableVertexAttribArray(2);
        }
    }
    fn gen_buffers(&mut self) {
        unsafe {
            gl::GenBuffers(1, &mut self.vertex_buffer);
            gl::GenVertexArrays(1, &mut self.vertex_array);
            gl::GenBuffers(1, &mut self.element_buffer);
        }
    }
    fn init_array_buffers(&mut self) {
        let mut vertex_data = self.build_vertex_data();
        unsafe {
            gl::BindBuffer(ARRAY_BUFFER, self.vertex_buffer);
            let size = (vertex_data.len() * size_of::<GLfloat>()) as isize;
            gl::BufferData(
                ARRAY_BUFFER,
                size,
                vertex_data.as_mut_ptr().cast(),
                STATIC_DRAW,
            );

            gl::BindBuffer(ELEMENT_ARRAY_BUFFER, self.element_buffer);
            gl::BufferData(
                ELEMENT_ARRAY_BUFFER,
                (self.indices.len() * size_of::<GLuint>()) as isize,
                self.indices.as_mut_ptr().cast(),
                STATIC_DRAW,
            );
        }
    }
    fn build_vertex_data(&mut self) -> Vec<c_float> {
        let mut vertex_data = Vec::new();
        for i in 0..self.vertices.len() {
            vertex_data.push(self.vertices[i].x);
            vertex_data.push(self.vertices[i].y);
            vertex_data.push(self.vertices[i].z);

            vertex_data.push(self.normals[i].x);
            vertex_data.push(self.normals[i].y);
            vertex_data.push(self.normals[i].z);

            if !self.tex_coords.is_empty() {
                vertex_data.push(self.tex_coords[i].x);
                vertex_data.push(self.tex_coords[i].y);
            }
        }
        vertex_data
    }
    pub unsafe fn update_vertex_buffer(&mut self) {
        let vertex_data = self.build_vertex_data();
        gl::BindBuffer(ARRAY_BUFFER, self.vertex_buffer);
        gl::BufferSubData(
            ARRAY_BUFFER,
            0,
            (vertex_data.len() * size_of::<GLfloat>()) as isize,
            vertex_data.as_ptr().cast(),
        );
        gl::BindBuffer(ARRAY_BUFFER, 0);
    }
    unsafe fn enable_texture(&mut self) {
        // self.shader.setup_textures();
    }
    pub unsafe fn from_obj(path: &str, shaderpath: &str) -> Result<Renderable, Box<dyn Error>> {
        let path_dir = Path::new(path).parent().expect("Jimbo jones the second");
        let input = BufReader::new(File::open(path).expect("Jimbo jones again!"));
        let obj = parse_obj(input).expect("Jimb jones the third");
        // let parsed_obj: Obj<TexturedVertex> = Obj::new(obj).expect("Jimbo jones the fourth");
        let (vertices, indices) = FromRawVertex::<u32>::process(
            obj.positions,
            obj.normals,
            obj.tex_coords.clone(),
            obj.polygons,
        )
        .map_err(|_| "Couldn't process vertices")?;

        let raw_mtl = parse_mtl(BufReader::new(
            File::open((path_dir.to_str().unwrap().to_owned()) + "/" + &obj.material_libraries[0])
                .map_err(|_| {
                    format!(
                        "Cannot find file {}",
                        path_dir.to_str().unwrap().parse::<String>().unwrap()
                            + "/"
                            + &obj.material_libraries[0]
                    )
                })?,
        ))
        .map_err(|_| "Couldn't parse mtl!")?;
        let new_shader = Shader::load_from_mtl(
            raw_mtl
                .materials
                .get("Material.001")
                .expect("Jimbo jones the seventh")
                .clone(),
            path_dir.to_str().unwrap(),
            shaderpath,
        );
        // let new_shader = Shader::load_from_path("shaders/comp_base_shader");
        Ok(Renderable::new_with_tex(
            vertices.iter().map(Vector3::from_vertex).collect(),
            indices,
            vertices.iter().map(Vector3::from_vertex).collect(),
            vertices
                .iter()
                .map(|x: &TexturedVertex| Vector2::new(x.texture[0], x.texture[1]))
                .collect(),
            new_shader?,
        ))
        // let mut ret = Renderable::new(vertices.iter().map(|x| Vector3::from_tex_vertex(x)).collect(), indices, vertices.iter().map(|x| Vector3::from_tex_vertex(x)).collect(),  new_shader);
    }
    fn build_model(&mut self) -> Matrix4<f32> {
        let mut model = Matrix4::one();
        model = model.scale(self.scale.x, self.scale.y, self.scale.z);
        model = model * Matrix4::from_translation(self.translation);
        model = model
            * Matrix4::from(Euler::new(
                Rad(self.rotation.x),
                Rad(self.rotation.z),
                Rad(self.rotation.y),
            ));
        // println!("{:?}", model);
        model
    }
    pub fn render(&mut self, shader_override: Option<&mut Box<Shader>>) {
        let model = self.build_model();
        if shader_override.is_some() {
            let shader: &mut Box<Shader> = shader_override.unwrap();
            shader.use_shader();
            shader.update().expect("Shader failed to update.");
            shader
                .set(model, "model")
                .expect("Couldn't update shader model.");
        } else {
            self.shader.use_shader();
            self.shader.update().expect("Shader failed to update.");
            self.shader
                .set(model, "model")
                .expect("Couldn't set shader");
        }

        unsafe {
            gl::BindVertexArray(self.vertex_array);

            gl::DrawElements(
                self.draw_type,
                (self.indices.len() * size_of::<GLuint>()) as GLsizei,
                UNSIGNED_INT,
                null(),
            );
            gl::BindVertexArray(0); // Cleanup
        }
        Shader::clear_shader();
    }

    pub fn scale(&mut self, x: f32, y: f32, z: f32) {
        self.scale.x *= x;
        self.scale.y *= y;
        self.scale.z *= z;
    }
    pub fn uniform_scale(&mut self, scale: f32) {
        self.scale *= scale;
    }
    pub fn rotate(&mut self, x: f32, y: f32, z: f32) {
        self.rotation += Vector3::new(x, y, z);
    }
    pub fn translate(&mut self, x: f32, y: f32, z: f32) {
        self.translation += Vector3::new(x, y, z)
    }
}
