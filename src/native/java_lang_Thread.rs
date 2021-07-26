use crate::{model::JavaValue, Classpath, JniEnv};

#[allow(non_snake_case)]
fn Java_java_lang_Thread_registerNatives(_: &JniEnv) -> Option<JavaValue> {
    None
}

#[allow(non_snake_case)]
fn Java_java_lang_Thread_currentThread(env: &JniEnv) -> Option<JavaValue> {
    Some(JavaValue::Object(Some(env.get_main_thread())))
}

#[allow(non_snake_case)]
fn Java_java_lang_Thread_setPriority0(_: &JniEnv) -> Option<JavaValue> {
    None
}

#[allow(non_snake_case)]
fn Java_java_lang_Thread_isAlive(env: &JniEnv) -> Option<JavaValue> {
    if let Some(is_alive) = env.get_internal_metadata(env.get_current_instance(), "is_alive") {
        Some(JavaValue::Boolean(is_alive == "true"))
    } else {
        Some(JavaValue::Boolean(false))
    }
}

#[allow(non_snake_case)]
fn Java_java_lang_Thread_start0(env: &JniEnv) -> Option<JavaValue> {
    env.set_internal_metadata(env.get_current_instance(), "is_alive", "true");
    None
}

pub fn initialize(cp: &mut Classpath) {
    register_jni!(
        cp,
        Java_java_lang_Thread_registerNatives,
        Java_java_lang_Thread_currentThread,
        Java_java_lang_Thread_setPriority0,
        Java_java_lang_Thread_isAlive,
        Java_java_lang_Thread_start0
    );
}
