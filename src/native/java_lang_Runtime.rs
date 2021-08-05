use crate::{
    exec::env::JniEnv,
    model::{JavaValue, RuntimeResult},
    Classpath,
};

#[allow(non_snake_case)]
fn Java_java_lang_Runtime_availableProcessors(_: &JniEnv) -> RuntimeResult<Option<JavaValue>> {
    Ok(Some(JavaValue::Int(1)))
}

pub fn initialize(cp: &mut Classpath) {
    register_jni!(cp, Java_java_lang_Runtime_availableProcessors);
}
