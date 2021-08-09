use crate::{
    exec::env::JniEnv,
    model::{JavaValue, RuntimeResult},
    Classpath,
};

#[allow(non_snake_case)]
fn Java_java_io_UnixFileSystem_initIDs(_: &JniEnv) -> RuntimeResult<Option<JavaValue>> {
    Ok(None)
}

#[allow(non_snake_case)]
fn Java_java_io_UnixFileSystem_getBooleanAttributes0(_: &JniEnv) -> RuntimeResult<Option<JavaValue>> {
    Ok(Some(JavaValue::Int(0)))
}

#[allow(non_snake_case)]
fn Java_java_io_UnixFileSystem_canonicalize0(env: &JniEnv) -> RuntimeResult<Option<JavaValue>> {
    // TODO: actually canonicalize paths
    Ok(Some(env.parameters[1].clone()))
}

pub fn initialize(cp: &mut Classpath) {
    register_jni!(
        cp,
        Java_java_io_UnixFileSystem_initIDs,
        Java_java_io_UnixFileSystem_getBooleanAttributes0,
        Java_java_io_UnixFileSystem_canonicalize0
    );
}
