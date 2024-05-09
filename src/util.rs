use std::ffi::{CStr, CString};
use std::fs::File;
use std::io::Read;

pub extern "system" fn debug_log(
    _: gl::types::GLenum,
    kind: gl::types::GLenum,
    _: gl::types::GLuint,
    _: gl::types::GLenum,
    _: gl::types::GLsizei,
    msg: *const gl::types::GLchar,
    _: *mut std::os::raw::c_void,
) {
    unsafe { println!("{}", CStr::from_ptr(msg).to_string_lossy()) }
}

pub fn load_file(path: String) -> CString {
    let mut file = File::open(path.as_str()).unwrap_or_else(|_| panic!("TODO: panic message"));
    let mut contents = String::new();
    file.read_to_string(&mut contents)
        .expect("TODO: panic message");
    let new_contents = CString::new(contents.as_bytes()).unwrap();
    return new_contents
}
