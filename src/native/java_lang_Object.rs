use crate::{Classpath, JniEnv, model::{JavaValue, RuntimeResult}, util::log};

#[allow(non_snake_case)]
fn Java_java_lang_Object_registerNatives(_: &JniEnv) -> RuntimeResult<Option<JavaValue>> {
    log("Registered Object natives!");

    Ok(None)
}

#[allow(non_snake_case)]
fn Java_java_lang_Object_hashCode(env: &JniEnv) -> RuntimeResult<Option<JavaValue>> {
    let mut x = env.get_current_instance();
    x = ((x >> 16) ^ x) * 0x45d9f3b;
    x = ((x >> 16) ^ x) * 0x45d9f3b;
    x = (x >> 16) ^ x;
    Ok(Some(JavaValue::Int(x as i32)))
}

pub fn initialize(cp: &mut Classpath) {
    register_jni!(
        cp,
        Java_java_lang_Object_registerNatives,
        Java_java_lang_Object_hashCode
    );
}
