use crate::util::{find_gl_error, load_file, GLFunctionError};
use cgmath::{Array, Matrix, Matrix2, Matrix3, Matrix4, Vector3, Vector4};
use core::slice::Iter;
use gl::types::{GLenum, GLint, GLsizei, GLsizeiptr, GLuint};
use gl::{
    FALSE, FRAGMENT_SHADER, STATIC_DRAW, TEXTURE_2D, TEXTURE_WRAP_S, TEXTURE_WRAP_T,
    UNIFORM_BUFFER, VERTEX_SHADER,
};
use glfw::ffi::glfwGetTime;
use image::{load_from_memory, open, DynamicImage};
use log::{debug, trace};
use obj::raw::material::{Material, MtlColor};
use obj::{TexturedVertex, Vertex};
use std::cell::{RefCell};
use std::collections::HashMap;
use std::error::Error;
use std::ffi::{CStr, CString};
use std::os::raw::{c_char};
use std::path::Path;
use std::ptr;
use std::ptr::null;
use std::sync::{Arc};

pub trait FromVertex<T> {
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

pub type ShaderPtr = Arc<RefCell<Shader>>;
fn new_shader_ptr(shader: Shader) -> ShaderPtr {
    Arc::new(RefCell::new(shader))
}

#[derive(Debug)]
enum CacheType {
    Matrix4(Matrix4<f32>),
    Matrix3(Matrix3<f32>),
    Matrix2(Matrix2<f32>),
    Float(f32),
    Int(i32),
    UInt(u32),
    VecInt(Vec<i32>),
    VecUInt(Vec<u32>),
    VecFloat(Vec<f32>),
}

fn from_color(color: Option<MtlColor>) -> Vec<f32> {
    if let Some(MtlColor::Rgb(r, g, b)) = color {
        return vec![r, g, b];
    }
    Vec::new()
}
// TODO: create a general structure for things like world buffers.
pub struct ShaderManager {
    pub shaders: Vec<ShaderPtr>,
    pub world_buffer: u32,
}
impl Default for ShaderManager {
    fn default() -> Self {
        Self::new()
    }
}

impl ShaderManager {
    pub fn new() -> ShaderManager {
        let mut ret = ShaderManager {
            shaders: Default::default(),
            world_buffer: 0,
        };
        unsafe {
            gl::GenBuffers(1, &mut ret.world_buffer);
            gl::BindBuffer(UNIFORM_BUFFER, ret.world_buffer);
            gl::BufferData(UNIFORM_BUFFER, 16, null(), STATIC_DRAW); // 1 * vec4
            gl::BindBuffer(UNIFORM_BUFFER, 0); // release the buffer

            gl::BindBufferRange(
                UNIFORM_BUFFER,
                1,
                ret.world_buffer,
                0,
                size_of::<Vector4<f32>>() as GLsizeiptr,
            );
        }
        ret
    }
    pub fn update(&mut self) -> Result<(), Box<dyn Error>> {
        let ambient: Vector4<f32> = Vector4::new(0.0, 0.0, 0.0, 1.0);
        unsafe {
            gl::BindBuffer(UNIFORM_BUFFER, self.world_buffer);
            gl::BufferSubData(
                UNIFORM_BUFFER,
                0,
                size_of::<Vector4<f32>>() as GLsizeiptr,
                ambient.as_ptr().cast(),
            );
            gl::BindBuffer(UNIFORM_BUFFER, 0);
        }
        for shader_ptr in self.iter() {
            shader_ptr.try_borrow_mut()?.try_runtime_recompile();
        }
        Ok(())
    }
    pub fn register(&mut self, shader: Shader) -> ShaderPtr {
        let arc = new_shader_ptr(shader);
        self.shaders.push(arc.clone());
        arc
    }
    pub fn iter(&self) -> Iter<ShaderPtr> {
        self.shaders.iter()
    }
    pub fn load_from_path(&mut self, path: &str) -> Result<ShaderPtr, GLFunctionError> {
        Ok(self.register(Shader::load_from_path(path)?))
    }
    pub fn count(&self) -> usize {
        self.shaders.len()
    }
}
// A struct to build shaders depending on the options provided in the material.
#[derive(Default)]
pub struct Shader {
    path: Option<String>,
    geo: u32,
    optionals: i32,
    pub textures: HashMap<String, u32>,
    vector_values: HashMap<String, Vec<f32>>,
    values: HashMap<String, f32>,
    debug_sources: Vec<CString>,
    program: Option<u32>,
    cache: HashMap<String, CacheType>,
}

impl Shader {
    fn new() -> Shader {
        Self::default()
    }
    pub fn from_source(
        vert_source: &str,
        frag_source: &str,
        geo_source: &str,
    ) -> Result<Shader, GLFunctionError> {
        let mut ret = Self::new();
        ret.program = Some(ret.compile(
            CString::new(vert_source).unwrap(),
            CString::new(frag_source).unwrap(),
            CString::new(geo_source).unwrap(),
        )?);
        Ok(ret)
    }
    pub fn load_from_path<P: AsRef<Path>>(path: P) -> Result<Shader, GLFunctionError> {
        let mut vert_string = path.as_ref().to_str().expect("").to_owned();
        vert_string.push_str(".vert");

        let mut frag_string = path.as_ref().to_str().expect("").to_owned();
        frag_string.push_str(".frag");
        let vert_source = load_file(vert_string);
        let frag_source = load_file(frag_string);

        let geo_string = format!("{}.geo", path.as_ref().to_str().expect("").to_owned());
        let mut geo_source = Default::default();
        if Path::new(&geo_string).exists() {
            geo_source = load_file(geo_string);
        }
        let mut ret = Shader {
            path: Some(path.as_ref().to_str().expect("").to_owned()),
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
            program: None,
            cache: Default::default(),
        };
        ret.program = Some(ret.compile(vert_source, frag_source, geo_source)?);
        ret.check_optionals();
        Ok(ret)
    }
    #[deprecated = "Use NarrowingMaterial instead."]
    pub fn load_from_mtl(
        mtl: Material,
        mtl_dir: &str,
        base_path: &str,
    ) -> Result<Shader, Box<dyn Error>> {
        let mut ret = Shader {
            path: Some(base_path.to_owned()),
            geo: 0,
            optionals: 0,
            textures: HashMap::new(),
            vector_values: HashMap::new(),
            values: Default::default(),
            debug_sources: vec![],
            program: None,
            cache: Default::default(),
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
        ret.debug_sources
            .push(CString::new(vert_source.clone()).unwrap());
        let mut frag_string = base_path.to_owned().clone();
        frag_string.push_str(".frag");
        let mut frag_source = load_file(frag_string).to_str().unwrap().to_owned();
        ret.debug_sources
            .push(CString::new(frag_source.clone()).unwrap());
        ret.debug_sources.push(CString::new("").unwrap());
        let mut locations =
            "layout (location = 0) in vec3 aPos;\nlayout (location = 1) in vec3 aNormal;\n"
                .to_owned();
        let mut passthroughs = "";
        let mut outs = "vec3 Normal;\nvec3 FragPos;\n".to_owned();
        let uniforms = "uniform mat4 model;";
        let std140s =
            "layout (std140) uniform Matrices {vec3 cameraPos;\nmat4 view;\nmat4 projection;\n};\nlayout (std140, binding=1) uniform World {vec4 ambient;};";
        if mtl.ambient_map.clone().is_some()
            || mtl.diffuse_map.clone().is_some()
            || mtl.specular_map.clone().is_some()
            || mtl.emissive_map.is_some()
        {
            locations += "layout (location = 2) in vec2 aTexCoord;";
            passthroughs = "vs_out.TexCoord = aTexCoord;";
            outs += "vec2 TexCoord;";
        }
        vert_source = vert_source.replace("//processed", "");
        vert_source = vert_source.replace("//T: LOCATIONS", locations.as_str());
        vert_source = vert_source.replace("//T: PASSTHROUGHS", passthroughs);
        vert_source = vert_source.replace(
            "//T: OUT",
            format!("out VS_OUT {{\n{}}} vs_out;", outs).as_str(),
        );
        vert_source = vert_source.replace("//T: UNIFORMS", uniforms);
        vert_source = vert_source.replace("//T: STD140", std140s);

        frag_source = frag_source.replace("//processed", "");
        frag_source = frag_source.replace(
            "//T: IN",
            format!("in VS_OUT {{\n{}}} fs_in;", outs).as_str(),
        );
        frag_source = frag_source.replace("//T: OUT", "out vec4 FragColor;");
        frag_source = frag_source.replace("//T: STD140", std140s);
        let mut textures = "".to_owned();
        let mut uniforms = "".to_owned();
        let mut logic = "".to_owned();
        for i in ["diffuse", "emissive"] {
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
        ret.values.insert("specular".to_string(), 1.0);
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
        ret.program = Some(ret.compile(
            CString::new(vert_source).expect("Failed to create CString"),
            CString::new(frag_source).expect("Failed to create CString"),
            CString::new("").expect("Failed to create CString"),
        )?);
        ret.use_();
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

        ret.check_optionals();
        Ok(ret)
    }
    fn create_shader(shader_type: GLenum) -> Result<u32, GLFunctionError> {
        let shader = unsafe { gl::CreateShader(shader_type) };
        if shader == 0 {
            Err(find_gl_error().err().unwrap())
        } else {
            Ok(shader)
        }
    }
    fn create_program() -> Result<GLuint, GLFunctionError> {
        let program = unsafe { gl::CreateProgram() };
        if program == 0 {
            Err(find_gl_error().unwrap_err())
        } else {
            Ok(program)
        }
    }
    fn compile_subshader(
        &mut self,
        program: u32,
        source: CString,
        id: u32,
    ) -> Result<(), GLFunctionError> {
        unsafe {
            gl::ShaderSource(id, 1, &source.as_ptr(), null());
            gl::CompileShader(id);
        }
        let mut success = 0;
        unsafe {
            gl::GetShaderiv(id, gl::COMPILE_STATUS, &mut success);
        }
        if success == 0 {
            find_gl_error().expect("booyah");
            let mut buf_size = 0;
            unsafe {
                gl::GetShaderiv(id, gl::INFO_LOG_LENGTH, &mut buf_size);
            }
            let mut buf: Vec<c_char> = Vec::with_capacity(buf_size as usize);
            // Safety: These both have the same buffer size.
            unsafe {
                gl::GetShaderInfoLog(id, buf_size, ptr::null_mut(), buf.as_mut_ptr().cast());
            }
            trace!("{}", source.into_string().unwrap());
            let mut r = "".to_string();
            unsafe {
                buf.set_len(buf_size as usize);
                r.push_str(CStr::from_ptr(buf.as_ptr()).to_str().unwrap());
            }
            r.push('\n');
            return Err(GLFunctionError::new(format!("Shader Compile Error: {}", r)));
        }
        unsafe {
            gl::AttachShader(program, id);
        }
        Ok(())
    }
    pub fn compile(
        &mut self,
        vert_source: CString,
        frag_source: CString,
        geo_source: CString,
    ) -> Result<u32, GLFunctionError> {
        let vert_program = Self::create_shader(VERTEX_SHADER)?;
        let frag_program = Self::create_shader(FRAGMENT_SHADER)?;
        let program = Self::create_program()?;
        if geo_source.to_str().unwrap() != "" {
            self.geo = unsafe { gl::CreateShader(gl::GEOMETRY_SHADER) };
            self.compile_subshader(program, geo_source, self.geo)?;
        }
        // println!("{:?}", self.geo);
        // println!("{:?}", vert_source.to_str().unwrap().replace("\\n", "\r\n"));
        // println!("{:?}", frag_source.to_str().unwrap().replace("\\n", "\r\n"));

        self.compile_subshader(program, vert_source, vert_program)?;
        self.compile_subshader(program, frag_source, frag_program)?;

        unsafe {
            gl::LinkProgram(program);
        }

        let mut success = 0;
        unsafe { gl::GetProgramiv(program, gl::LINK_STATUS, &mut success); }
        if success == 0 {
            let mut log_len = 0_i32;
            let mut v: Vec<u8> = Vec::with_capacity(1024);
            unsafe { gl::GetProgramInfoLog(program, 1024, &mut log_len, v.as_mut_ptr().cast()) };
            v.resize(log_len as usize, 0u8);
            return Err(GLFunctionError::new(format!(
                "Shader Compile Error: {}",
                String::from_utf8_lossy(&v)
            )));
        }
        Self::bind_matrices(program).expect("Failed to bind matrices");
        Ok(program)
    }

    /*    unsafe fn get_shader_error(&mut self) -> String {
            let mut v: Vec<u8> = Vec::with_capacity(1024);
            let mut log_len = 0_i32;
            gl::GetShaderInfoLog(self.frag, 1024, &mut log_len, v.as_mut_ptr().cast());
            v.set_len(log_len.try_into().unwrap());
            String::from_utf8(v).expect("Couldn't convert to string.")
        }
    */
    pub(crate) fn update(&mut self) -> Result<(), String> {
        self.update_optionals()?;
        Ok(())
    }
    pub fn try_runtime_recompile(&mut self) {
        if let Some(path) = self.path.clone() {
            let mut vert_string: String = path.clone();
            vert_string.push_str(".vert");

            let mut frag_string: String = path.to_owned().clone();
            frag_string.push_str(".frag");
            let vert_source = load_file(vert_string);
            let frag_source = load_file(frag_string);

            let geo_string = format!("{}.geo", path);
            let mut geo_source = Default::default();
            if Path::new(&geo_string).exists() {
                geo_source = load_file(geo_string);
            }

            if (vert_source != self.debug_sources[0]
                || frag_source != self.debug_sources[1]
                || geo_source != self.debug_sources[2])
                && !vert_source.to_str().unwrap().contains("//proccessed")
            {
                let new_progid =
                    self.compile(vert_source.clone(), frag_source.clone(), geo_source.clone());
                if new_progid.is_ok() {
                    self.program = Some(new_progid.unwrap());
                    self.load_cached_uniforms();
                }
                self.debug_sources = vec![vert_source, frag_source, geo_source];
            }
        }
    }

    /**
    Must be called use_ because use is a reserved keyword.
     */
    pub fn use_(&self) {
        if self.program.is_none() {
            return;
        }
        self.use_textures();
        unsafe { gl::UseProgram(self.program.unwrap()) };
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
    /// # Safety
    /// This isn't doing anything crazy?
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
        Self::create_image_texture(img)
    }
    fn create_image_texture(data: DynamicImage) -> u32 {
        let mut texture = 0;
        let img = data.clone();
        let rgba = img.to_rgba8();
        let raw = rgba;
        unsafe {
            gl::GenTextures(1, &mut texture);
            gl::BindTexture(TEXTURE_2D, texture);
            Self::setup_textures();
            Self::set_texture(
                texture as usize,
                &raw,
                img.width() as usize,
                img.height() as usize,
            )
            // gl::GenerateMipmap(TEXTURE_2D);
        }

        texture
    }
    pub(crate) fn set_texture(texture: usize, data: &[u8], width: usize, height: usize) {
        unsafe {
            gl::BindTexture(TEXTURE_2D, texture as GLuint);
            gl::TexImage2D(
                TEXTURE_2D,
                0,
                gl::RGBA as i32,
                width as GLsizei,
                height as GLsizei,
                0,
                gl::RGBA,
                gl::UNSIGNED_BYTE,
                data.as_ptr().cast(),
            );
            gl::BindTexture(TEXTURE_2D, 0);
        }
    }
    pub fn register_create_texture(&mut self, name: &str) -> usize {
        let mut texture = 0;
        unsafe {
            gl::GenTextures(1, &mut texture);
            gl::BindTexture(TEXTURE_2D, texture);
            Self::setup_textures();
            // gl::GenerateMipmap(TEXTURE_2D);
            gl::BindTexture(TEXTURE_2D, 0);
        }

        self.textures.insert(name.to_string(), texture);
        texture as usize
    }
    fn use_textures(&self) {
        let vals = self.textures.values().cloned().collect::<Vec<u32>>();
        if vals.is_empty() {
            return;
        }
        unsafe { gl::BindTextures(0, self.textures.len() as GLsizei, vals.as_ptr()) };
    }

    pub fn bind_matrices(program: u32) -> Result<(), Box<dyn Error>> {
        let block_name = CString::new("Matrices").unwrap();
        let cast = block_name.into_raw();
        unsafe {
            let index = gl::GetUniformBlockIndex(program, cast.cast());
            gl::UniformBlockBinding(program, index, 0);
        }

        Ok(())
    }

    fn is_used(&self) -> Result<bool, ()> {
        if self.program.is_none() {
            return Err(());
        }
        let mut value = 0;
        unsafe { gl::GetIntegerv(gl::CURRENT_PROGRAM, &mut value) };
        Ok(value == self.program.unwrap() as i32)
    }

    fn get_uniform_location(&self, name: &str) -> Result<GLint, String> {
        if self.program.is_none() {
            return Err("No program".to_owned());
        }
        let block_name = CString::new(name).unwrap();
        let casted = block_name.into_raw();
        let location = unsafe { gl::GetUniformLocation(self.program.unwrap(), casted) };
        if location == -1 {
            Err(format!("Uniform {} not found", name))
        } else {
            Ok(location)
        }
    }

    fn load_cached_uniforms(&mut self) {
        println!("{:?}", self.cache);
        for (k, v) in self.cache.iter() {
            match v {
                CacheType::Matrix4(m) => {
                    self.direct_set(*m, k)
                        .expect("Couldn't direct_set matrix");
                }
                CacheType::Matrix3(m) => {
                    self.direct_set(*m, k)
                        .expect("Couldn't direct_set matrix");
                }
                CacheType::Matrix2(m) => {
                    self.direct_set(*m, k)
                        .expect("Couldn't direct_set matrix");
                }
                CacheType::Float(f) => {
                    self.direct_set(*f, k).expect("Couldn't direct_set float");
                }
                CacheType::Int(i) => {
                    self.direct_set(*i, k).expect("Couldn't direct_set int");
                }
                CacheType::UInt(u) => {
                    self.direct_set(*u, k).expect("Couldn't direct_set uint");
                }
                CacheType::VecInt(v) => {
                    self.direct_set(v.clone(), k)
                        .expect("Couldn't direct_set vec int");
                }
                CacheType::VecUInt(v) => {
                    self.direct_set(v.clone(), k)
                        .expect("Couldn't direct_set vec uint");
                }
                CacheType::VecFloat(v) => {
                    self.direct_set(v.clone(), k)
                        .expect("Couldn't direct_set vec float");
                }
            }
        }
    }
}
pub enum MaybeColorTexture {
    Texture(DynamicImage),
    RGBA([f32; 4]),
    RGB([f32; 3]),
}

pub enum MaybeTexture {
    Texture(DynamicImage),
    Value(f32),
}

pub struct NarrowingMaterial {
    pub diffuse: Option<MaybeColorTexture>,
    pub emissive: Option<MaybeColorTexture>,
    pub specular: Option<MaybeTexture>,
    pub metallic: Option<MaybeTexture>,
    pub roughness: Option<MaybeTexture>,
    pub ambient_scaling: Option<MaybeTexture>,
    pub normal: Option<MaybeTexture>,
}
impl NarrowingMaterial {
    pub(crate) fn from_obj_mtl(mtl: obj::raw::material::Material) -> NarrowingMaterial {
        let mut ret = NarrowingMaterial {
            diffuse: None,
            emissive: None,
            specular: None,
            metallic: None,
            roughness: None,
            ambient_scaling: None,
            normal: None,
        };
        if mtl.diffuse.is_some() {
            let diff = from_color(mtl.diffuse);
            let new_diff = [diff[0], diff[1], diff[2], 1.0];
            ret.diffuse = Some(MaybeColorTexture::RGBA(new_diff));
        }
        if mtl.emissive.is_some() {
            let emis = from_color(mtl.emissive);
            let new_emis = [emis[0], emis[1], emis[2], 1.0];
            ret.emissive = Some(MaybeColorTexture::RGBA(new_emis));
        }
        if mtl.optical_density.is_some() {
            let specular = ((mtl.optical_density.unwrap() - 1.0)
                / (mtl.optical_density.unwrap() + 1.0))
                .powf(2.0)
                / 0.08;
            ret.specular = Some(MaybeTexture::Value(specular));
        }
        ret
        // todo: provide waay better support for this.
    }
    pub(crate) fn from_gltf_mtl(
        material: gltf::Material,
        images: &[gltf::image::Data],
        buffers: &[gltf::buffer::Data],
        base_path: &str,
    ) -> Result<NarrowingMaterial, Box<dyn Error>> {
        macro_rules! texture_or_factor {
            ($property:expr, $texture_source:expr, $factor_source:expr, $texture_enum:path, $factor_enum:path) => {
                if let Some(inner_texture) = $texture_source {
                    let source = inner_texture.texture().source().source();
                    match source {
                        gltf::image::Source::Uri { uri, .. } => {
                            $property = Some($texture_enum(
                                open(base_path.to_owned() + "/" + uri)
                                    .expect("Couldn't open image"),
                            ));
                        }
                        gltf::image::Source::View { view, .. } => {
                            let buf = view.buffer();
                            let start: usize = view.offset();
                            let end: usize = start + view.length();
                            let img_data =
                                &buffers.get(buf.index()).unwrap().to_owned()[start..end];
                            let dynimg = load_from_memory(img_data).expect("Couldn't load image");
                            $property = Some($texture_enum(dynimg));
                        }
                    }
                } else {
                    $property = Some($factor_enum($factor_source));
                }
            };
        }
        let mut ret = NarrowingMaterial {
            diffuse: None,
            emissive: None,
            specular: None,
            metallic: None,
            roughness: None,
            ambient_scaling: None,
            normal: None,
        };
        texture_or_factor!(
            ret.diffuse,
            material.pbr_metallic_roughness().base_color_texture(),
            material.pbr_metallic_roughness().base_color_factor(),
            MaybeColorTexture::Texture,
            MaybeColorTexture::RGBA
        );
        texture_or_factor!(
            ret.emissive,
            material.emissive_texture(),
            material.emissive_factor(),
            MaybeColorTexture::Texture,
            MaybeColorTexture::RGB
        );
        if let Some(spec) = material.specular() {
            texture_or_factor!(
                ret.specular,
                spec.specular_texture(),
                spec.specular_factor(),
                MaybeTexture::Texture,
                MaybeTexture::Value
            );
        }
        // TODO: Roughness/Metallic texture
        ret.metallic = Some(MaybeTexture::Value(
            material.pbr_metallic_roughness().metallic_factor(),
        ));
        ret.roughness = Some(MaybeTexture::Value(
            material.pbr_metallic_roughness().roughness_factor(),
        ));
        ret.ambient_scaling = Some(MaybeTexture::Value(1.0)); // todo: maybe change this? provide parameter?
        texture_or_factor!(
            ret.normal,
            material.normal_texture(),
            1.0,
            MaybeTexture::Texture,
            MaybeTexture::Value
        );
        Ok(ret)
    }
    pub(crate) fn with_path(self, base_path: &str) -> Result<Shader, Box<dyn Error>> {
        let vert_string = base_path.to_owned().clone() + ".vert";
        let vert_source = load_file(vert_string).to_str().unwrap().to_owned();
        let frag_string = base_path.to_owned().clone() + ".frag";
        let frag_source = load_file(frag_string).to_str().unwrap().to_owned();
        let debug_sources = vec![
            CString::new(vert_source.clone()).unwrap(),
            CString::new(frag_source.clone()).unwrap(),
            CString::new("").unwrap(),
        ];
        let mut ret = self.into_shader(vert_source, frag_source)?;
        ret.debug_sources.extend(debug_sources);
        ret.path = Some(base_path.to_string());
        Ok(ret)
    }
    pub(crate) fn into_shader(
        self,
        mut vert_source: String,
        mut frag_source: String,
    ) -> Result<Shader, Box<dyn Error>> {
        let mut ret = Shader {
            path: None,
            geo: 0,
            optionals: 0,
            textures: HashMap::new(),
            vector_values: HashMap::new(),
            values: Default::default(),
            debug_sources: vec![],
            program: None,
            cache: Default::default(),
        };
        match self.diffuse {
            None => {
                ret.vector_values.insert("diffuse".to_owned(), vec![0.5; 3]);
            }
            Some(enum_val) => match enum_val {
                MaybeColorTexture::Texture(v) => {
                    ret.textures
                        .insert("diffuse".to_owned(), Shader::create_image_texture(v));
                }
                MaybeColorTexture::RGBA(v) => {
                    ret.vector_values.insert("diffuse".to_owned(), v.to_vec());
                }
                MaybeColorTexture::RGB(v) => {
                    ret.vector_values
                        .insert("diffuse".to_owned(), vec![v[0], v[1], v[2], 1.0]);
                }
            },
        }
        match self.specular {
            None => {
                ret.values.insert("specular".to_owned(), 1.0);
            }
            Some(enum_val) => match enum_val {
                MaybeTexture::Texture(v) => {
                    ret.textures
                        .insert("specular".to_owned(), Shader::create_image_texture(v));
                }
                MaybeTexture::Value(v) => {
                    ret.values.insert("specular".to_owned(), v);
                }
            },
        }
        match self.emissive {
            None => {
                ret.vector_values
                    .insert("emissive".to_owned(), vec![0.0; 4]);
            }
            Some(enum_val) => match enum_val {
                MaybeColorTexture::Texture(v) => {
                    ret.textures
                        .insert("emissive".to_owned(), Shader::create_image_texture(v));
                }
                MaybeColorTexture::RGBA(v) => {
                    ret.vector_values.insert("emissive".to_owned(), v.to_vec());
                }
                MaybeColorTexture::RGB(v) => {
                    ret.vector_values
                        .insert("emissive".to_owned(), vec![v[0], v[1], v[2], 1.0]);
                }
            },
        }
        let mut locations =
            "layout (location = 0) in vec3 aPos;\nlayout (location = 1) in vec3 aNormal;\n"
                .to_owned();
        let mut passthroughs = "";
        let mut outs = "vec3 Normal;\nvec3 FragPos;\nfloat Time;".to_owned();
        let uniforms = "uniform mat4 model;";
        let std140s =
            "layout (std140) uniform Matrices {vec3 cameraPos;\nmat4 view;\nmat4 projection;\n};\nlayout (std140, binding=1) uniform World {vec4 ambient;};";
        // if mtl.diffuse_map.clone().is_some()
        //     || mtl.specular_map.clone().is_some()
        //     || mtl.emissive_map.is_some()
        if !ret.textures.is_empty() {
            locations += "layout (location = 2) in vec2 aTexCoord;";
            passthroughs = "vs_out.TexCoord = aTexCoord;";
            outs += "vec2 TexCoord;";
        }
        vert_source = vert_source.replace("//T: LOCATIONS", locations.as_str());
        vert_source = vert_source.replace("//T: PASSTHROUGHS", passthroughs);
        vert_source = vert_source.replace(
            "//T: OUT",
            format!("out VS_OUT {{\n{}}} vs_out;", outs).as_str(),
        );
        vert_source = vert_source.replace("//T: UNIFORMS", uniforms);
        vert_source = vert_source.replace("//T: STD140", std140s);

        frag_source = frag_source.replace(
            "//T: IN",
            format!("in VS_OUT {{\n{}}} fs_in;", outs).as_str(),
        );
        frag_source = frag_source.replace("//T: OUT", "out vec4 FragColor;");
        frag_source = frag_source.replace("//T: STD140", std140s);
        let mut textures = "".to_owned();
        let mut uniforms = "".to_owned();
        let mut logic = "".to_owned();
        for i in ["diffuse", "emissive"] {
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
        ret.values.insert("specular_exponent".to_owned(), 256.0);
        for i in ret.values.keys() {
            uniforms.push_str(format!("uniform float {};\n", i).as_str());
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
        debug!("frag_source: {}", frag_source);
        debug!("vert_source: {}", vert_source);
        ret.program = Some(ret.compile(
            CString::new(vert_source).expect("Failed to create CString"),
            CString::new(frag_source).expect("Failed to create CString"),
            CString::new("").expect("Failed to create CString"),
        )?);
        ret.use_();
        for (i, v) in ret.vector_values.clone() {
            let ov = v.clone();
            let vector = vec![ov[0], ov[1], ov[2], 1.0];
            let os = i.clone();
            // println!("{}", i);
            ret.set(vector, os.as_str())?;
        }
        for (i, v) in ret.values.clone() {
            let os = i.clone();
            ret.set(v, os.as_str()).expect("Failed to set values");
        }
        Shader::clear_shader();
        ret.check_optionals();
        Ok(ret)
    }
}

macro_rules! set_matrix_value {
    ($tt:ty, $gl_call:expr, $cache_type:path) => {
        impl SetValue<$tt> for Shader {
            /**
             * Sets a value in the shader and caches it.
             */
            fn set(&mut self, value: $tt, name: &str) -> Result<(), String> {
                if let Some($cache_type(cached)) = self.cache.get(name) {
                    if *cached == value {
                        return Ok(());
                    }
                }
                // Insert into the cache if it successfully sets the value.
                self.direct_set(value.clone(), name).map(|()| {
                    self.cache.insert(name.to_owned(), $cache_type(value));
                })
            }
            /**
             * Sets a value in the shader without caching it.
             */
            fn direct_set(&self, value: $tt, name: &str) -> Result<(), String> {
                if !(self.is_used().unwrap_or(true)) {
                    self.use_();
                }
                let loc = self.get_uniform_location(name)?;
                // Safety: trust it's safe
                unsafe { $gl_call(loc, value.clone())? };
                match find_gl_error() {
                    Ok(_) => Ok(()),
                    Err(e) => Err(e.to_string() + " " + name),
                }
            }
        }
    };
}

pub trait SetValue<T> {
    /// Sets a value based on the type of the value.
    fn set(&mut self, value: T, name: &str) -> Result<(), String>;
    fn direct_set(&self, value: T, name: &str) -> Result<(), String>;
}

set_matrix_value!(
    Matrix4<f32>,
    |loc: i32, value: Matrix4<f32>| -> Result<(), String> {
        gl::UniformMatrix4fv(loc, 1, FALSE, value.as_ptr());
        Ok(())
    },
    CacheType::Matrix4
);
set_matrix_value!(
    Matrix3<f32>,
    |loc: i32, mut value: Matrix3<f32>| -> Result<(), String> {
        gl::UniformMatrix3fv(loc, 1, false.into(), value.as_mut_ptr());
        Ok(())
    },
    CacheType::Matrix3
);
set_matrix_value!(
    Matrix2<f32>,
    |loc: i32, mut value: Matrix2<f32>| -> Result<(), String> {
        gl::UniformMatrix2fv(loc, 1, false.into(), value.as_mut_ptr());
        Ok(())
    },
    CacheType::Matrix2
);
set_matrix_value!(
    f32,
    |loc: i32, value: f32| -> Result<(), String> {
        gl::Uniform1f(loc, value);
        Ok(())
    },
    CacheType::Float
);
set_matrix_value!(
    u32,
    |loc: i32, value: u32| -> Result<(), String> {
        gl::Uniform1ui(loc, value);
        Ok(())
    },
    CacheType::UInt
);
set_matrix_value!(
    i32,
    |loc: i32, value: i32| -> Result<(), String> {
        gl::Uniform1i(loc, value);
        Ok(())
    },
    CacheType::Int
);
set_matrix_value!(
    Vec<i32>,
    |loc: i32, mut value: Vec<i32>| -> Result<(), String> {
        match value.len() {
            1 => gl::Uniform1iv(loc, 1, value.as_mut_ptr()),
            2 => gl::Uniform2iv(loc, 1, value.as_mut_ptr()),
            3 => gl::Uniform3iv(loc, 1, value.as_mut_ptr()),
            4 => gl::Uniform4iv(loc, 1, value.as_mut_ptr()),
            _ => return Err("Incorrectly sized vector ".to_owned()),
        }
        Ok(())
    },
    CacheType::VecInt
);

set_matrix_value!(
    Vec<u32>,
    |loc: i32, mut value: Vec<u32>| -> Result<(), String> {
        match value.len() {
            1 => gl::Uniform1uiv(loc, 1, value.as_mut_ptr()),
            2 => gl::Uniform2uiv(loc, 1, value.as_mut_ptr()),
            3 => gl::Uniform3uiv(loc, 1, value.as_mut_ptr()),
            4 => gl::Uniform4uiv(loc, 1, value.as_mut_ptr()),
            _ => return Err("Incorrectly sized vector ".to_owned()),
        }
        Ok(())
    },
    CacheType::VecUInt
);

set_matrix_value!(
    Vec<f32>,
    |loc: i32, mut value: Vec<f32>| -> Result<(), String> {
        match value.len() {
            1 => gl::Uniform1fv(loc, 1, value.as_mut_ptr()),
            2 => gl::Uniform2fv(loc, 1, value.as_mut_ptr()),
            3 => gl::Uniform3fv(loc, 1, value.as_mut_ptr()),
            4 => gl::Uniform4fv(loc, 1, value.as_mut_ptr()),
            _ => return Err("Incorrectly sized vector ".to_owned()),
        }
        Ok(())
    },
    CacheType::VecFloat
);
