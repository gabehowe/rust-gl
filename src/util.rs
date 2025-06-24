use std::ffi::{CStr, CString};
use std::fmt;
use std::fs::File;
use std::io::Read;

pub extern "system" fn debug_log(
    _: gl::types::GLenum,
    _: gl::types::GLenum,
    _: gl::types::GLuint,
    _: gl::types::GLenum,
    _: gl::types::GLsizei,
    msg: *const gl::types::GLchar,
    _: *mut std::os::raw::c_void,
) {
    let msg = unsafe { CStr::from_ptr(msg).to_string_lossy() };
    if !msg.contains("Buffer detailed info:") {
        println!("GL Debug: {}", msg);
    }
}

pub fn load_file(path: String) -> CString {
    let mut file =
        File::open(path.as_str()).unwrap_or_else(|_| panic!("Failed to open file {}!", path));
    let mut contents = String::new();
    file.read_to_string(&mut contents)
        .expect("TODO: panic message");
    let new_contents = CString::new(contents.as_bytes()).unwrap();
    new_contents
}
/// These need to be disabled for performance, apparently.
pub fn find_gl_error() -> Result<(), GLFunctionError> {
    let error = unsafe { gl::GetError() };
    if error != gl::NO_ERROR {
        Err(GLFunctionError::new(error.to_string()))
    } else {
        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct GLFunctionError {
    pub message: String,
}
impl GLFunctionError {
    pub fn new(message: String) -> GLFunctionError {
        GLFunctionError { message }
    }
}
impl Default for GLFunctionError {
    fn default() -> Self {
        GLFunctionError::new("".to_string())
    }
}
impl std::error::Error for GLFunctionError {}
impl fmt::Display for GLFunctionError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "A GL Function failed with {}", self.message)
    }
}
