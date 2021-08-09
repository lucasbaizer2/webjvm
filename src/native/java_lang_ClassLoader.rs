use crate::{
    model::{JavaValue, RuntimeResult},
    Classpath, JniEnv,
};

#[allow(non_snake_case)]
fn Java_java_lang_ClassLoader_registerNatives(_: &JniEnv) -> RuntimeResult<Option<JavaValue>> {
    Ok(None)
}

#[allow(non_snake_case)]
fn Java_java_lang_ClassLoader_findBuiltinLib(env: &JniEnv) -> RuntimeResult<Option<JavaValue>> {
    Ok(Some(env.parameters[0].clone()))
}

#[allow(non_snake_case)]
fn Java_java_lang_ClassLoader_findLoadedClass0(env: &JniEnv) -> RuntimeResult<Option<JavaValue>> {
    let class_name = env.get_string(env.parameters[1].as_object().unwrap().unwrap());
    let heap = env.jvm.heap.borrow();
    if let Some(looked_up_id) = heap.loaded_classes_lookup.get(&class_name) {
        Ok(Some(JavaValue::Object(Some(env.get_class_object(*looked_up_id)))))
    } else {
        Ok(Some(JavaValue::Object(None)))
    }
}

#[allow(non_snake_case)]
fn Java_java_lang_ClassLoader_findBootstrapClass(env: &JniEnv) -> RuntimeResult<Option<JavaValue>> {
    Java_java_lang_ClassLoader_findLoadedClass0(env)
}

#[allow(non_snake_case)]
fn Java_java_lang_ClassLoader_00024NativeLibrary_load(env: &JniEnv) -> RuntimeResult<Option<JavaValue>> {
    let instance = env.get_current_instance()?;
    env.set_field(instance, "loaded", JavaValue::Boolean(true));

    Ok(None)
}

pub fn initialize(cp: &mut Classpath) {
    register_jni!(
        cp,
        Java_java_lang_ClassLoader_registerNatives,
        Java_java_lang_ClassLoader_findBuiltinLib,
        Java_java_lang_ClassLoader_findLoadedClass0,
        Java_java_lang_ClassLoader_findBootstrapClass,
        Java_java_lang_ClassLoader_00024NativeLibrary_load
    );
}
