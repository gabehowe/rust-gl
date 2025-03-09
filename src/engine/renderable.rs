use std::collections::HashMap;
use std::error::Error;
use std::ffi::{c_float, c_uint, CString};
use std::fs::File;
use std::io::BufReader;
use std::mem::{size_of, transmute};
use std::os::raw::c_void;
use std::path::Path;
use std::ptr::null;

use cgmath::{Euler, Matrix, Matrix2, Matrix3, Matrix4, One, Rad, Vector2, Vector3, Zero};
use gl::types::{GLenum, GLfloat, GLint, GLsizei, GLuint};
use gl::{
    ARRAY_BUFFER, ELEMENT_ARRAY_BUFFER, FALSE, FLOAT, FRAGMENT_SHADER, STATIC_DRAW, TEXTURE_2D,
    TEXTURE_WRAP_S, TEXTURE_WRAP_T, TRIANGLES, UNSIGNED_INT, VERTEX_SHADER,
};
use glfw::ffi::glfwGetTime;
use image::open;
use obj::raw::material::{Material, MtlColor};
use obj::raw::{parse_mtl, parse_obj};
use obj::{FromRawVertex, TexturedVertex, Vertex};
use log::debug;
use crate::engine::transformation::Transformation;
use crate::engine::util::{find_gl_error, load_file, GLFunctionError};
pub trait SetValue<T> {
    /// Sets a value based on the type of the value.
    fn set(&mut self, value: T, name: &str) -> Result<(), String>;
}

impl SetValue<Matrix4<f32>> for Shader {
    fn set(&mut self, value: Matrix4<f32>, name: &str) -> Result<(), String> {
        unsafe {
            gl::UniformMatrix4fv(
                self.get_uniform_location(name)?,
                1,
                FALSE,
                transmute(&value[0][0]),
            )
        };
        match find_gl_error() {
            None => Ok(()),
            Some(e) => Err(e.to_string()),
        }
    }
}
impl SetValue<Matrix3<f32>> for Shader {
    fn set(&mut self, mut value: Matrix3<f32>, name: &str) -> Result<(), String> {
        unsafe {
            gl::UniformMatrix3fv(
                self.get_uniform_location(name)?,
                1,
                false.into(),
                value.as_mut_ptr(),
            )
        };
        match find_gl_error() {
            None => Ok(()),
            Some(e) => Err(e.to_string()),
        }
    }
}
impl SetValue<Matrix2<f32>> for Shader {
    fn set(&mut self, mut value: Matrix2<f32>, name: &str) -> Result<(), String> {
        unsafe {
            gl::UniformMatrix2fv(
                self.get_uniform_location(name)?,
                1,
                false.into(),
                value.as_mut_ptr(),
            )
        };
        Ok(())
    }
}

impl SetValue<f32> for Shader {
    fn set(&mut self, value: f32, name: &str) -> Result<(), String> {
        unsafe { gl::Uniform1f(self.get_uniform_location(name)?, value) };
        match find_gl_error() {
            None => Ok(()),
            Some(e) => Err(e.to_string()),
        }
    }
}
impl SetValue<i32> for Shader {
    fn set(&mut self, value: i32, name: &str) -> Result<(), String> {
        unsafe { gl::Uniform1i(self.get_uniform_location(name)?, value) }
        match find_gl_error() {
            None => Ok(()),
            Some(e) => Err(e.to_string()),
        }
    }
}
impl SetValue<Vec<i32>> for Shader {
    fn set(&mut self, mut value: Vec<i32>, name: &str) -> Result<(), String> {
        match value.len() {
            1 => unsafe { gl::Uniform1iv(self.get_uniform_location(name)?, 1, value.as_mut_ptr()) },
            2 => unsafe { gl::Uniform2iv(self.get_uniform_location(name)?, 1, value.as_mut_ptr()) },
            3 => unsafe { gl::Uniform3iv(self.get_uniform_location(name)?, 1, value.as_mut_ptr()) },
            4 => unsafe { gl::Uniform4iv(self.get_uniform_location(name)?, 1, value.as_mut_ptr()) },
            _ => return Err("Incorrectly sized vector ".to_owned() + name),
        }
        Ok(())
    }
}
impl SetValue<Vec<u32>> for Shader {
    fn set(&mut self, mut value: Vec<u32>, name: &str) -> Result<(), String> {
        match value.len() {
            1 => unsafe {
                gl::Uniform1uiv(self.get_uniform_location(name)?, 1, value.as_mut_ptr())
            },
            2 => unsafe {
                gl::Uniform2uiv(self.get_uniform_location(name)?, 1, value.as_mut_ptr())
            },
            3 => unsafe {
                gl::Uniform3uiv(self.get_uniform_location(name)?, 1, value.as_mut_ptr())
            },
            4 => unsafe {
                gl::Uniform4uiv(self.get_uniform_location(name)?, 1, value.as_mut_ptr())
            },
            _ => return Err("Incorrectly sized vector ".to_owned() + name),
        }

        match find_gl_error() {
            None => Ok(()),
            Some(e) => Err(e.to_string()),
        }
    }
}
impl SetValue<Vec<f32>> for Shader {
    fn set(&mut self, mut value: Vec<f32>, name: &str) -> Result<(), String> {
        match value.len() {
            1 => unsafe { gl::Uniform1fv(self.get_uniform_location(name)?, 1, value.as_mut_ptr()) },
            2 => unsafe { gl::Uniform2fv(self.get_uniform_location(name)?, 1, value.as_mut_ptr()) },
            3 => unsafe { gl::Uniform3fv(self.get_uniform_location(name)?, 1, value.as_mut_ptr()) },
            4 => unsafe { gl::Uniform4fv(self.get_uniform_location(name)?, 1, value.as_mut_ptr()) },
            _ => return Err("Incorrectly sized vector ".to_owned() + name),
        }
        Ok(())
    }
}

trait FromVertex<T> {
    fn from_vertex(vertex: &T) -> Self;
}
impl FromVertex<Vertex> for Vector3<f32> {
    fn from_vertex(vertex: &Vertex) -> Self {
        Vector3::new(vertex.position[0], vertex.position[1], vertex.position[2])
    }
}
impl FromVertex<TexturedVertex> for Vector3<f32> {
    fn from_vertex(vertex: &TexturedVertex) -> Self {
        Vector3::new(vertex.position[0], vertex.position[1], vertex.position[2])
    }
}

fn from_color(color: Option<MtlColor>) -> Vec<f32> {
    if let Some(MtlColor::Rgb(r, g, b)) = color {
        return vec![r, g, b];
    }
    Vec::new()
}
// A struct to build shaders depending on the options provided in the material.

pub struct Shader {
    path: String,
    vert: u32,
    frag: u32,
    program: u32,
    geo: u32,
    optionals: i32,
    textures: HashMap<String, u32>,
    vector_values: HashMap<String, Vec<f32>>,
    values: HashMap<String, f32>,
    debug_sources: Vec<CString>,
}

impl Shader {
    pub fn load_from_path(path: &str) -> Result<Shader, GLFunctionError> {
        let mut vert_string = path.to_owned().clone();
        vert_string.push_str(".vert");

        let mut frag_string = path.to_owned().clone();
        frag_string.push_str(".frag");
        let vert_source = load_file(vert_string);
        let frag_source = load_file(frag_string);

        let geo_string = format!("{}.geo", path);
        let mut geo_source = Default::default();
        if Path::new(&geo_string).exists() {
            geo_source = load_file(geo_string);
        }
        let mut ret = Shader {
            path: path.to_owned(),
            vert: Self::create_shader(VERTEX_SHADER)?,
            frag: Self::create_shader(FRAGMENT_SHADER)?,
            program: Self::create_program()?,
            geo: 0,
            optionals: 0,
            textures: HashMap::new(),
            vector_values: HashMap::from([
                ("ambient".to_owned(), vec![0.; 3]),
                ("diffuse".to_owned(), vec![0.; 3]),
                ("specular".to_owned(), vec![0.; 3]),
                ("emissive".to_owned(), vec![0.; 3]),
            ]),
            values: HashMap::new(),
            debug_sources: vec![vert_source.clone(), frag_source.clone(), geo_source.clone()],
        };
        ret.compile(vert_source, frag_source, geo_source);
        Ok(ret)
    }
    pub fn load_from_mtl(
        mtl: Material,
        mtl_dir: &str,
        base_path: &str,
    ) -> Result<Shader, Box<dyn Error>> {
        let mut ret = Shader {
            path: base_path.to_owned(),
            vert: Self::create_shader(VERTEX_SHADER)?,
            frag: Self::create_shader(FRAGMENT_SHADER)?,
            program: Self::create_program()?,
            geo: 0,
            optionals: 0,
            textures: HashMap::new(),
            vector_values: HashMap::new(),
            values: Default::default(),
            debug_sources: vec![],
        };
        if mtl.ambient.is_some() {
            ret.vector_values
                .insert("ambient".to_owned(), from_color(mtl.ambient));
        }
        if mtl.diffuse.is_some() {
            ret.vector_values
                .insert("diffuse".to_owned(), from_color(mtl.diffuse));
        }
        if mtl.specular.is_some() {
            ret.vector_values
                .insert("specular".to_owned(), from_color(mtl.specular));
        }
        if mtl.emissive.is_some() {
            ret.vector_values
                .insert("emissive".to_owned(), from_color(mtl.emissive));
        }
        if mtl.optical_density.is_some() {
            //let ior = mtl.optical_density.unwrap();
            // let specular = ((ior-1.0)/(ior+1.0)).powf(2.0)/0.08;
            let specular = 256.0;
            ret.values.insert("specular_exponent".to_owned(), specular);
        }

        if mtl.diffuse_map.is_some() {
            let path = mtl.diffuse_map.clone().unwrap().file.clone().to_owned();
            ret.textures.insert(
                "diffuse".to_owned(),
                Self::load_texture((mtl_dir.to_owned() + "/" + &path).as_str()),
            );
        }

        if mtl.ambient_map.is_some() {
            let path = mtl.ambient_map.clone().unwrap().file.clone().to_owned();
            ret.textures.insert(
                "ambient".to_owned(),
                Self::load_texture((mtl_dir.to_owned() + "/" + &path).as_str()),
            );
        }

        if mtl.specular_map.is_some() {
            let path = mtl.specular_map.clone().unwrap().file.clone().to_owned();
            ret.textures.insert(
                "specular".to_owned(),
                Self::load_texture((mtl_dir.to_owned() + "/" + &path).as_str()),
            );
        }

        if mtl.emissive_map.is_some() {
            let path = mtl.emissive_map.clone().unwrap().file.clone().to_owned();
            ret.textures.insert(
                "emissive".to_owned(),
                Self::load_texture((mtl_dir.to_owned() + "/" + &path).as_str()),
            );
        }
        let mut vert_string = base_path.to_owned().clone();
        vert_string.push_str(".vert");
        let mut vert_source = load_file(vert_string).to_str().unwrap().to_owned();
        ret.debug_sources.push(CString::new(vert_source.clone()).unwrap());
        let mut frag_string = base_path.to_owned().clone();
        frag_string.push_str(".frag");
        let mut frag_source = load_file(frag_string).to_str().unwrap().to_owned();
        ret.debug_sources.push(CString::new(frag_source.clone()).unwrap());
        ret.debug_sources.push(CString::new("").unwrap());
        let mut locations =
            "layout (location = 0) in vec3 aPos;\nlayout (location = 1) in vec3 aNormal;\n"
                .to_owned();
        let mut passthroughs = "";
        let mut outs = "vec3 Normal;\nvec3 FragPos;\n".to_owned();
        let uniforms = "uniform mat4 model;";
        let std140s =
            "layout (std140) uniform Matrices {vec3 cameraPos;\nmat4 view;\nmat4 projection;\n};";
        if mtl.ambient_map.clone().is_some()
            || mtl.diffuse_map.clone().is_some()
            || mtl.specular_map.clone().is_some()
            || mtl.emissive_map.is_some()
        {
            locations += "layout (location = 2) in vec2 aTexCoord;";
            passthroughs = "vs_out.TexCoord = aTexCoord;";
            outs += "vec2 TexCoord;";
        }
        vert_source = vert_source.replace("#proccessed", "");
        vert_source = vert_source.replace("//T: LOCATIONS", locations.as_str());
        vert_source = vert_source.replace("//T: PASSTHROUGHS", passthroughs);
        vert_source = vert_source.replace(
            "//T: OUT",
            format!("out VS_OUT {{\n{}}} vs_out;", outs).as_str(),
        );
        vert_source = vert_source.replace("//T: UNIFORMS", uniforms);
        vert_source = vert_source.replace("//T: STD140", std140s);

        frag_source = frag_source.replace("#proccessed", "");
        frag_source = frag_source.replace(
            "//T: IN",
            format!("in VS_OUT {{\n{}}} fs_in;", outs).as_str(),
        );
        frag_source = frag_source.replace("//T: OUT", "out vec4 FragColor;");
        frag_source = frag_source.replace("//T: STD140", std140s);
        let mut textures = "".to_owned();
        let mut uniforms = "".to_owned();
        let mut logic = "".to_owned();
        for i in ["ambient", "diffuse", "specular", "emissive"] {
            if ret.textures.contains_key(i) {
                logic.push_str("vec4 ");
                logic.push_str(i);
                logic.push_str(" = texture(");
                logic.push_str(i);
                logic.push_str(", fs_in.TexCoord);\n");
            } else {
                uniforms.push_str("uniform vec4 ");
                uniforms.push_str(i);
                uniforms.push_str(";\n");
            }
        }
        if ret.values.contains_key("specular_exponent") {
            uniforms.push_str("uniform float specular_exponent;\n");
        }
        for i in 0..ret.textures.len() {
            let texture_name = ret.textures.keys().nth(i).unwrap();
            let mut texture = format!("layout (binding={i}) uniform sampler2D ").to_owned();
            texture.push_str(&texture_name.clone());
            texture.push_str(";\n");
            textures.push_str(&texture);
        }
        frag_source = frag_source.replace("//T: TEXTURES", textures.as_str());
        frag_source = frag_source.replace("//T: LOGIC", logic.as_str());
        frag_source = frag_source.replace("//T: UNIFORMS", uniforms.as_str());
        ret.compile(
            CString::new(vert_source).expect("Failed to create CString"),
            CString::new(frag_source).expect("Failed to create CString"),
            CString::new("").expect("Failed to create CString"),
        );
        ret.use_shader();
        for (i, v) in ret.vector_values.clone() {
            let ov = v.clone();
            let vector = vec![ov[0], ov[1], ov[2], 1.0];
            let os = i.clone();
            // println!("{}", i);
            ret.set(vector, os.as_str())?;
        }
        for (i, v) in ret.values.clone() {
            let os = i.clone();
            ret.set(v, os.as_str())?;
        }
        Self::clear_shader();

        Ok(ret)
    }
    fn create_shader(shader_type: GLenum) -> Result<u32, GLFunctionError> {
        let shader = unsafe { gl::CreateShader(shader_type) };
        if shader == 0 {
            Err(find_gl_error().unwrap_or_default())
        } else {
            Ok(shader)
        }
    }
    fn create_program() -> Result<GLuint, GLFunctionError> {
        let program = unsafe { gl::CreateProgram() };
        if program == 0 {
            Err(find_gl_error().unwrap_or_default())
        } else {
            Ok(program)
        }
    }
    fn compile_subshader(&mut self, source: CString, id: u32) {
        unsafe {
            gl::ShaderSource(id, 1, &source.as_ptr(), null());
            gl::CompileShader(id);
        }
        let mut success = 0;
        unsafe {
            gl::GetShaderiv(id, gl::COMPILE_STATUS, &mut success);
        }
        if success == 0 {
            let mut log_len = 0_i32;
            let mut v: Vec<u8> = Vec::with_capacity(1024);
            // Safety: These both have the same buffer size.
            unsafe {
                gl::GetShaderInfoLog(id, 1024, &mut log_len, v.as_mut_ptr().cast());
            }
            v.resize(log_len.try_into().unwrap(), 0u8);
            println!("{:?}", source);
            panic!("Shader Compile Error: {}", String::from_utf8_lossy(&v));
        }
        unsafe {
            gl::AttachShader(self.program, id);
        }
    }
    pub fn compile(&mut self, vert_source: CString, frag_source: CString, geo_source: CString) {
        if geo_source.to_str().unwrap() != "" {
            self.geo = unsafe { gl::CreateShader(gl::GEOMETRY_SHADER) };
            self.compile_subshader(geo_source, self.geo);
        }
        // println!("{:?}", self.geo);
        // println!("{:?}", vert_source.to_str().unwrap().replace("\\n", "\r\n"));
        // println!("{:?}", frag_source.to_str().unwrap().replace("\\n", "\r\n"));

        self.compile_subshader(vert_source, self.vert);
        self.compile_subshader(frag_source, self.frag);

        unsafe {
            gl::LinkProgram(self.program);
        }

        let mut success = 0;
        unsafe {
            gl::GetProgramiv(self.program, gl::LINK_STATUS, &mut success);
        }
        if success == 0 {
            let mut log_len = 0_i32;
            let mut v: Vec<u8> = Vec::with_capacity(1024);
            unsafe {
                gl::GetProgramInfoLog(self.program, 1024, &mut log_len, v.as_mut_ptr().cast())
            };
            v.resize(log_len as usize, 0u8);
            panic!("Shader Compile Error: {}", String::from_utf8_lossy(&v));
        }
        // gl::DeleteProgram(self.vert);
        // gl::DeleteProgram(self.frag);
        self.check_optionals()
    }

    unsafe fn get_shader_error(&mut self) -> String {
        let mut v: Vec<u8> = Vec::with_capacity(1024);
        let mut log_len = 0_i32;
        gl::GetShaderInfoLog(self.frag, 1024, &mut log_len, v.as_mut_ptr().cast());
        v.set_len(log_len.try_into().unwrap());
        String::from_utf8(v).expect("Couldn't convert to string.")
    }
    fn update(&mut self) -> Result<(), String> {
        self.try_runtime_recompile();
        self.update_optionals()?;
        Ok(())
    }
    fn try_runtime_recompile(&mut self) {
        let mut vert_string = self.path.to_owned().clone();
        vert_string.push_str(".vert");

        let mut frag_string = self.path.to_owned().clone();
        frag_string.push_str(".frag");
        let vert_source = load_file(vert_string);
        let frag_source = load_file(frag_string);

        let geo_string = format!("{}.geo", self.path);
        let mut geo_source = Default::default();
        if Path::new(&geo_string).exists() {
            geo_source = load_file(geo_string);
        }

        if vert_source != CString::new(self.debug_sources[0].clone()).unwrap()
            || frag_source != CString::new(self.debug_sources[1].clone()).unwrap()
            || geo_source != CString::new(self.debug_sources[2].clone()).unwrap()
        {
            if vert_source.to_str().unwrap().contains("#proccessed") {
                return
            } else {
                self.program = Self::create_program().unwrap();
                self.vert = Self::create_shader(VERTEX_SHADER).unwrap();
                self.frag = Self::create_shader(FRAGMENT_SHADER).unwrap();
                self.compile(vert_source, frag_source, geo_source);
            }
        }
    }
    pub fn use_shader(&mut self) {
        unsafe { gl::UseProgram(self.program) };
    }
    pub fn clear_shader() {
        unsafe { gl::UseProgram(0) };
    }

    fn check_optionals(&mut self) {
        if self.get_uniform_location("time").is_ok() {
            self.optionals |= 1;
        }
    }
    fn update_optionals(&mut self) -> Result<(), String> {
        if self.optionals & 1 == 1 {
            self.set(unsafe { glfwGetTime() } as f32, "time")?;
        }
        Ok(())
    }
    unsafe fn setup_textures() {
        gl::TexParameteri(TEXTURE_2D, TEXTURE_WRAP_S, gl::REPEAT as i32);
        gl::TexParameteri(TEXTURE_2D, TEXTURE_WRAP_T, gl::REPEAT as i32);
        gl::TexParameteri(TEXTURE_2D, gl::TEXTURE_MIN_FILTER, gl::LINEAR as i32);
        gl::TexParameteri(TEXTURE_2D, gl::TEXTURE_MAG_FILTER, gl::LINEAR as i32);
        // gl::GenTextures(1, &mut self.texture);
        // gl::BindTexture(TEXTURE_2D, self.texture);
    }
    fn load_texture(path: &str) -> u32 {
        let img = open(path).expect("Jimbo jones");
        let height = img.height();
        let width = img.width();
        let data = img.to_rgb8().into_raw();
        Self::create_texture(
            &data[0] as *const u8 as *const c_void,
            width as i32,
            height as i32,
        )
    }
    fn create_texture(data: *const c_void, width: i32, height: i32) -> u32 {
        let mut texture = 0;
        unsafe {
            gl::GenTextures(1, &mut texture);
            gl::BindTexture(TEXTURE_2D, texture);
            Self::setup_textures();
            gl::TexImage2D(
                TEXTURE_2D,
                0,
                gl::RGB as i32,
                width,
                height,
                0,
                gl::RGB,
                gl::UNSIGNED_BYTE,
                data,
            );
            // gl::GenerateMipmap(TEXTURE_2D);
            gl::BindTexture(TEXTURE_2D, 0);
        }

        texture
    }
    fn use_textures(&mut self) {
        let vals = self.textures.values().cloned().collect::<Vec<u32>>();
        if vals.is_empty() {
            return;
        }
        unsafe { gl::BindTextures(0, self.textures.len() as GLsizei, vals.as_ptr()) };
    }

    pub unsafe fn bind_matrices(&mut self) {
        let block_name = CString::new("Matrices").unwrap();
        let cast = block_name.into_raw();
        let index = gl::GetUniformBlockIndex(self.program, cast.cast());
        gl::UniformBlockBinding(self.program, index, 0);
    }

    unsafe fn is_used(&self) -> bool {
        let mut value = 0;
        gl::GetIntegerv(gl::CURRENT_PROGRAM, &mut value);
        value == self.program as i32
    }

    fn get_uniform_location(&mut self, name: &str) -> Result<GLint, String> {
        let block_name = CString::new(name).unwrap();
        let casted = block_name.into_raw();
        let location = unsafe { gl::GetUniformLocation(self.program, casted) };
        if location == -1 {
            Err(format!("Uniform {} not found", name))
        } else {
            Ok(location)
        }
    }
}

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
            self.shader.use_textures();
            self.shader.update().expect("Shader failed to update.");
            self.shader
                .set(model, "model")
                .expect("Couldn't set shader");
        }

        unsafe {
            gl::BindVertexArray(self.vertex_array);

            gl::DrawElements(
                TRIANGLES,
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
