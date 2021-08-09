use crate::{
    model::{JavaValue, RuntimeResult},
    Classpath, JniEnv,
};

#[allow(non_snake_case)]
fn Java_java_lang_String_intern(env: &JniEnv) -> RuntimeResult<Option<JavaValue>> {
    let current_string = env.get_string(env.get_current_instance()?);
    let intern_id = match {
        let heap = env.jvm.heap.borrow();
        heap.interned_string_map.get(&current_string).cloned()
    } {
        Some(intern_id) => intern_id,
        None => env.jvm.create_string_object(&current_string, true),
    };
    Ok(Some(JavaValue::Object(Some(intern_id))))
}

pub fn initialize(cp: &mut Classpath) {
    register_jni!(cp, Java_java_lang_String_intern);
}
