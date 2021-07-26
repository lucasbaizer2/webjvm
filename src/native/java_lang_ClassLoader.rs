use crate::{Classpath, JniEnv, model::{JavaValue, RuntimeResult}};

#[allow(non_snake_case)]
fn Java_java_lang_ClassLoader_registerNatives(_: &JniEnv) -> RuntimeResult<Option<JavaValue>> {
    Ok(None)
}

pub fn initialize(cp: &mut Classpath) {
    register_jni!(cp, Java_java_lang_ClassLoader_registerNatives);
}
