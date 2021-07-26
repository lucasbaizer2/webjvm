use crate::{Classpath, exec::env::JniEnv, model::{JavaValue, RuntimeResult}};

#[allow(non_snake_case)]
fn Java_java_io_FileDescriptor_initIDs(_: &JniEnv) -> RuntimeResult<Option<JavaValue>> {
    Ok(None)
}

pub fn initialize(cp: &mut Classpath) {
    register_jni!(cp, Java_java_io_FileDescriptor_initIDs);
}
