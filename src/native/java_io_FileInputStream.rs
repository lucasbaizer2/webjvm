use crate::{exec::env::JniEnv, model::JavaValue, Classpath};

#[allow(non_snake_case)]
fn Java_java_io_FileInputStream_initIDs(_: &JniEnv) -> Option<JavaValue> {
    None
}

pub fn initialize(cp: &mut Classpath) {
    register_jni!(cp, Java_java_io_FileInputStream_initIDs);
}
