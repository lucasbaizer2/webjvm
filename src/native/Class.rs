use crate::{exec::JavaValue, util::log, Classpath, JniEnv};

#[allow(non_snake_case)]
fn Java_java_lang_Class_registerNatives(_: &JniEnv) -> Option<JavaValue> {
    log("Registered Class natives!");

    None
}

pub fn initialize(cp: &mut Classpath) {
    register_jni!(cp, Java_java_lang_Class_registerNatives);
}

