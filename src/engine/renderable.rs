use std::ffi::{c_float, c_uint, CStr, CString};
use std::fs::File;
use std::io::BufReader;
use std::mem::{size_of, size_of_val, transmute};
use std::os::raw::c_void;
use std::path::Path;
use std::ptr::null;

use cgmath::{Array, Euler, Matrix, Matrix4, One, Rad, Vector3, Zero};
use gl::types::{GLfloat, GLint, GLsizei, GLuint};
use gl::{ARRAY_BUFFER, ELEMENT_ARRAY_BUFFER, FALSE, FLOAT, FRAGMENT_SHADER, NO_ERROR, STATIC_DRAW, TRIANGLES, UNSIGNED_INT, VERTEX_SHADER};
use obj::{load_obj, Obj, Vertex};
use crate::engine::transformation::Transformation;

use crate::engine::util::load_file;

trait FromVertex {
    fn from_vertex(vertex: &Vertex) -> Self;
}

impl FromVertex for Vector3<f32> {
    fn from_vertex(vertex: &Vertex) -> Self {
        Vector3::new(vertex.position[0], vertex.position[1], vertex.position[2])
    }
}

pub struct Shader {
    path: String,
    vert: u32,
    frag: u32,
    program: u32,
    geo: u32
}

impl Shader {
    pub unsafe fn load_from_path(path: &str) -> Shader {
        let mut ret = Shader {
            path: path.to_owned(),
            vert: gl::CreateShader(VERTEX_SHADER),
            frag: gl::CreateShader(FRAGMENT_SHADER),
            program: gl::CreateProgram(),
            geo: 0
        };
        ret.compile();
        return ret;
    }
    unsafe fn compile_subshader(&mut self, source: CString, id: u32){
        gl::ShaderSource(id, 1, &source.as_ptr(), null());
        gl::CompileShader(id);
        let mut success = 0;
        gl::GetShaderiv(id, gl::COMPILE_STATUS, &mut success);
        if success == 0 {
            let mut log_len = 0_i32;
            let mut v: Vec<u8> = Vec::with_capacity(1024);
            gl::GetShaderInfoLog(id, 1024, &mut log_len, v.as_mut_ptr().cast());
            v.set_len(log_len.try_into().unwrap());
            println!("{:?}", source);
            panic!("Shader Compile Error: {}", String::from_utf8_lossy(&v));
        }
        gl::AttachShader(self.program, id);
    }
    pub unsafe fn compile(&mut self) {
        let mut vert_string = self.path.clone();
        vert_string.push_str(".vert");

        let mut frag_string = self.path.clone();
        frag_string.push_str(".frag");
        let vert_source = load_file(vert_string);
        let frag_source = load_file(frag_string);

        self.compile_subshader(vert_source, self.vert);
        self.compile_subshader(frag_source, self.frag);
        let geo_string = format!("{}.geo", self.path);
        if Path::new(&geo_string).exists() {
            self.geo = gl::CreateShader(gl::GEOMETRY_SHADER);
            self.compile_subshader(load_file(geo_string), self.geo);
        }

        gl::LinkProgram(self.program);

        let mut success = 0;
        gl::GetProgramiv(self.program, gl::LINK_STATUS, &mut success);
        if success == 0 {
            let mut log_len = 0_i32;
            let mut v: Vec<u8> = Vec::with_capacity(1024);
            gl::GetProgramInfoLog(self.program, 1024, &mut log_len, v.as_mut_ptr().cast());
            v.set_len(log_len.try_into().unwrap());
            panic!("Shader Compile Error: {}", String::from_utf8_lossy(&v));
        }
        // gl::DeleteProgram(self.vert);
        // gl::DeleteProgram(self.frag);
    }

    unsafe fn get_shader_error(&mut self) -> String {
        let mut v: Vec<u8> = Vec::with_capacity(1024);
        let mut log_len = 0_i32;
        gl::GetShaderInfoLog(self.frag, 1024, &mut log_len, v.as_mut_ptr().cast());
        v.set_len(log_len.try_into().unwrap());
        let ret_str = String::from_utf8(v).expect("Jimbo jones");
        return ret_str;
    }
    pub unsafe fn use_shader(&mut self) {
        gl::UseProgram(self.program);
    }

    pub unsafe fn bind_matrices(&mut self) {
        let block_name = CString::new("Matrices").unwrap();
        let cast = block_name.into_raw();
        let index = gl::GetUniformBlockIndex(self.program, cast.cast());
        gl::UniformBlockBinding(self.program, index, 0);
    }
    unsafe fn get_uniform_location(&mut self, name: &str) -> GLint {
        let block_name = CString::new(name).unwrap();
        let casted = block_name.into_raw();
        let location = gl::GetUniformLocation(self.program, casted);
        if (location == -1) {
            let error = self.get_shader_error();

            panic!("Couldn't find location {}, {}, {}", name, error, self.path);
        }
        return location;
    }
    pub unsafe fn set_mat4(&mut self, matrix4: Matrix4<f32>, name: &str) {
        let location = self.get_uniform_location(name);
        gl::UniformMatrix4fv(location, 1, FALSE, transmute(&matrix4[0][0]))
    }
    pub unsafe fn set_vec3(&mut self, vector3: Vector3<f32>, name: &str) {
        let location = self.get_uniform_location(name);
        gl::Uniform3fv(location, 1, vector3.as_ptr().cast())
    }
}

pub struct Renderable {
    pub(crate) vertices: Vec<Vector3<c_float>>,
    indices: Vec<c_uint>,
    pub shader: Shader,
    vertex_array: GLuint,
    vertex_buffer: GLuint,
    element_buffer: GLuint,
    rotation: Vector3<f32>,
    translation: Vector3<f32>,
    scale: Vector3<f32>,
    normals: Vec<Vector3<f32>>,
}

impl Renderable {
    pub(crate) fn new(vertices: Vec<Vector3<f32>>, indices: Vec<u32>, normals: Vec<Vector3<f32>>, shader: Shader) -> Renderable {
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
        };
        unsafe {
            gl::GenBuffers(1, &mut ret.vertex_buffer);
            gl::GenVertexArrays(1, &mut ret.vertex_array);
            gl::GenBuffers(1, &mut ret.element_buffer);

            gl::BindVertexArray(ret.vertex_array);

            let vertex_data = ret.build_vertex_data();
            gl::BindBuffer(ARRAY_BUFFER, ret.vertex_buffer);
            let size = (vertex_data.len() * size_of::<GLfloat>()) as isize;
            gl::BufferData(
                ARRAY_BUFFER,
                size,
                transmute(&vertex_data[0]),
                STATIC_DRAW,
            );

            gl::BindBuffer(ELEMENT_ARRAY_BUFFER, ret.element_buffer);
            gl::BufferData(
                ELEMENT_ARRAY_BUFFER,
                (ret.indices.len() * size_of::<GLuint>()) as isize,
                transmute(&ret.indices[0]),
                STATIC_DRAW,
            );

            gl::VertexAttribPointer(0, 3, FLOAT, FALSE, 2 * (3 * size_of::<GLfloat>()) as GLsizei, 0 as *const _);
            gl::EnableVertexAttribArray(0);

            gl::VertexAttribPointer(1, 3, FLOAT, FALSE, 2 * (3 * size_of::<GLfloat>()) as GLsizei, (3 * size_of::<GLfloat>()) as *const _);
            gl::EnableVertexAttribArray(1);
            //
            // gl::EnableVertexAttribArray(1);
            // gl::VertexAttribPointer(1, 3, FLOAT, FALSE, (6 * size_of::<GLfloat>()) as GLsizei, (3 * size_of::<GLfloat>()) as *const _);
            gl::BindBuffer(ARRAY_BUFFER, 0);

            gl::BindVertexArray(0);
        }
        return ret;
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
        }
        return vertex_data;
    }
    pub unsafe fn update_vertex_buffer(&mut self) {
        let vertex_data = self.build_vertex_data();
        gl::BindBuffer(ARRAY_BUFFER, self.vertex_buffer);
        gl::BufferSubData(ARRAY_BUFFER, 0, (vertex_data.len() * size_of::<GLfloat>()) as isize, transmute(&vertex_data[0]));
        gl::BindBuffer(ARRAY_BUFFER, 0);
    }
    pub unsafe fn from_obj(path: &str, shaderpath: &str) -> Renderable {
        let input = BufReader::new(File::open(path).expect("Jimbo jones again!"));
        let obj: Obj = load_obj(input).expect("Jimb jones the third");
        // let mut verts: Vec<f32> = Vec::new();
        // let mut normals = Vec::new();
        // for i in obj.vertices.iter() {
        //     verts.push(i.position[0]);
        //     verts.push(i.position[1]);
        //     verts.push(i.position[2]);
        //
        //     normals.push(i.normal[0]);
        //     normals.push(i.normal[1]);
        //     normals.push(i.normal[2]);
        // }
        // verts = obj.vertices.iter().map(|x| x.position.iter().flatten()).collect();
        // for i in obj.vertices.iter() {
        //     verts.push(i.position[0]);
        //     verts.push(i.position[1]);
        //     verts.push(i.position[2]);
        //
        // }
        let mut indices = Vec::new();
        for i in obj.indices.iter() {
            indices.push(*i as u32);
        }
        let shader = Shader::load_from_path(shaderpath);
        let mut ret = Renderable::new(obj.vertices.iter().map(|x| Vector3::from_vertex(x)).collect(), indices, obj.vertices.iter().map(|x| Vector3::from_vertex(x)).collect(), shader);
        return ret;
    }
    unsafe fn build_model(&mut self) -> Matrix4<f32> {
        let mut model = Matrix4::one();
        model = model.scale(self.scale.x, self.scale.y, self.scale.z);
        model = model * Matrix4::from_translation(self.translation);
        model = model * Matrix4::from(Euler::new(Rad(self.rotation.x), Rad(self.rotation.z), Rad(self.rotation.y)));
        // println!("{:?}", model);
        return model;
    }
    pub unsafe fn render(&mut self) {
        let model = self.build_model();
        self.shader.use_shader();
        self.shader.set_mat4(model, "model");
        gl::BindBuffer(ARRAY_BUFFER, self.vertex_buffer);
        gl::BindVertexArray(self.vertex_array);
        gl::BindBuffer(ELEMENT_ARRAY_BUFFER, self.element_buffer);

        gl::DrawElements(
            TRIANGLES,
            (self.indices.len() * size_of::<GLuint>()) as GLsizei,
            UNSIGNED_INT,
            0 as *const c_void,
        );
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
