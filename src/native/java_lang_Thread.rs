use crate::{
    model::{InternalMetadata, JavaValue, RuntimeResult},
    Classpath, JniEnv,
};

#[allow(non_snake_case)]
fn Java_java_lang_Thread_registerNatives(_: &JniEnv) -> RuntimeResult<Option<JavaValue>> {
    Ok(None)
}

#[allow(non_snake_case)]
fn Java_java_lang_Thread_currentThread(env: &JniEnv) -> RuntimeResult<Option<JavaValue>> {
    Ok(Some(JavaValue::Object(Some(env.get_main_thread()))))
}

#[allow(non_snake_case)]
fn Java_java_lang_Thread_setPriority0(_: &JniEnv) -> RuntimeResult<Option<JavaValue>> {
    Ok(None)
}

#[allow(non_snake_case)]
fn Java_java_lang_Thread_isAlive(env: &JniEnv) -> RuntimeResult<Option<JavaValue>> {
    if let Some(is_alive) = env.get_internal_metadata(env.get_current_instance()?, "is_alive") {
        Ok(Some(JavaValue::Boolean(is_alive.into_usize() == 1)))
    } else {
        Ok(Some(JavaValue::Boolean(false)))
    }
}

#[allow(non_snake_case)]
fn Java_java_lang_Thread_start0(env: &JniEnv) -> RuntimeResult<Option<JavaValue>> {
    env.set_internal_metadata(env.get_current_instance()?, "is_alive", InternalMetadata::Numeric(1));
    Ok(None)
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
