use std::io::Write;

use crate::{
    exec::env::JniEnv,
    model::{JavaValue, RuntimeResult},
    Classpath,
};

#[allow(non_snake_case)]
fn Java_java_io_FileOutputStream_initIDs(_: &JniEnv) -> RuntimeResult<Option<JavaValue>> {
    Ok(None)
}

#[allow(non_snake_case)]
fn Java_java_io_FileOutputStream_writeBytes(env: &JniEnv) -> RuntimeResult<Option<JavaValue>> {
    let byte_buffer = env.parameters[1].as_array().unwrap();
    let offset = env.parameters[2].as_int().unwrap();
    let length = env.parameters[3].as_int().unwrap();
    let _ = env.parameters[4].as_boolean().unwrap();

    let mut local_buffer = Vec::with_capacity(length as usize);
    for i in offset..offset + length {
        local_buffer.push(env.get_array_element(byte_buffer, i as usize).as_byte().unwrap() as u8);
    }

    let fos = env.get_current_instance();
    let fd_obj = env.get_field(fos, "fd").as_object().unwrap().unwrap();
    let fd = env.get_field(fd_obj, "fd").as_int().unwrap();
    if fd == 1 || fd == 2 {
        #[cfg(target_arch = "wasm32")]
        {
            let str = String::from_utf8(local_buffer).unwrap();
            if fd == 1 {
                web_sys::console::log_1(&str.into());
            } else if fd == 2 {
                web_sys::console::error_1(&str.into());
            }
        }
        #[cfg(not(target_arch = "wasm32"))]
        {
            if fd == 1 {
                let mut stdout = std::io::stdout();
                stdout.write_all(&local_buffer).unwrap();
            } else if fd == 2 {
                let mut stderr = std::io::stderr();
                stderr.write_all(&local_buffer).unwrap();
            }
        }
    } else {
        return Err(env.throw_exception("java/lang/UnsupportedOperationException", Some("writing to a file")));
    }

    Ok(None)
}

pub fn initialize(cp: &mut Classpath) {
    register_jni!(cp, Java_java_io_FileOutputStream_initIDs, Java_java_io_FileOutputStream_writeBytes);
}
