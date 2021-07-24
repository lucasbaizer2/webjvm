use crate::{exec::JavaValue, util::log, Classpath, JniEnv};

#[allow(non_snake_case)]
fn Java_java_lang_System_registerNatives(_: &JniEnv) -> Option<JavaValue> {
    log("Registered System natives!");

    None
}

#[allow(non_snake_case)]
fn Java_java_lang_System_currentTimeMillis(_: &JniEnv) -> Option<JavaValue> {
    Some(JavaValue::Long(5))
}

#[allow(non_snake_case)]
fn Java_java_lang_System_nanoTime(_: &JniEnv) -> Option<JavaValue> {
    Some(JavaValue::Long(5))
}

#[allow(non_snake_case)]
fn Java_java_lang_System_arraycopy(_: &JniEnv) -> Option<JavaValue> {
    None
}

pub fn initialize(cp: &mut Classpath) {
    register_jni!(cp, Java_java_lang_System_registerNatives);
    register_jni!(cp, Java_java_lang_System_currentTimeMillis);
    register_jni!(cp, Java_java_lang_System_nanoTime);
    register_jni!(cp, Java_java_lang_System_arraycopy);
}
