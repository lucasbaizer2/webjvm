use crate::{model::JavaValue, Classpath, JniEnv};

#[allow(non_snake_case)]
fn Java_java_lang_ClassLoader_registerNatives(_: &JniEnv) -> Option<JavaValue> {
    None
}

pub fn initialize(cp: &mut Classpath) {
    register_jni!(cp, Java_java_lang_ClassLoader_registerNatives);
}
