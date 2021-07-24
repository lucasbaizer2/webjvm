use crate::{exec::JavaValue, util::log, Classpath, JniEnv};

#[allow(non_snake_case)]
fn Java_java_lang_Object_registerNatives(_: &JniEnv) -> Option<JavaValue> {
    log("Registered Object natives!");

    None
}

#[allow(non_snake_case)]
fn Java_java_lang_Object_hashCode(env: &JniEnv) -> Option<JavaValue> {
    let mut x = env.container_instance.unwrap();
    x = ((x >> 16) ^ x) * 0x45d9f3b;
    x = ((x >> 16) ^ x) * 0x45d9f3b;
    x = (x >> 16) ^ x;
    Some(JavaValue::Int(x as i32))
}

pub fn initialize(cp: &mut Classpath) {
    register_jni!(cp, Java_java_lang_Object_registerNatives);
    register_jni!(cp, Java_java_lang_Object_hashCode);
}
