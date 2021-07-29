use crate::{
    model::{JavaArray, JavaValue, RuntimeResult},
    Classpath, JniEnv,
};

#[allow(non_snake_case)]
fn Java_java_lang_Object_registerNatives(_: &JniEnv) -> RuntimeResult<Option<JavaValue>> {
    Ok(None)
}

#[allow(non_snake_case)]
fn Java_java_lang_Object_hashCode(env: &JniEnv) -> RuntimeResult<Option<JavaValue>> {
    let mut x = match &env.parameters[0] {
        JavaValue::Array(id) => *id,
        JavaValue::Object(obj) => *obj.as_ref().unwrap(),
        _ => panic!(),
    };
    x = ((x >> 16) ^ x) * 0x45d9f3b;
    x = ((x >> 16) ^ x) * 0x45d9f3b;
    x = (x >> 16) ^ x;
    Ok(Some(JavaValue::Int(x as i32)))
}

#[allow(non_snake_case)]
fn Java_java_lang_Object_getClass(env: &JniEnv) -> RuntimeResult<Option<JavaValue>> {
    let type_name = env.get_object_type_name(env.get_current_instance());
    let type_obj = env.get_class_object(env.get_class_id(&type_name));

    Ok(Some(JavaValue::Object(Some(type_obj))))
}

#[allow(non_snake_case)]
fn Java_java_lang_Object_clone(env: &JniEnv) -> RuntimeResult<Option<JavaValue>> {
    match &env.parameters[0] {
        JavaValue::Array(id) => {
            let new_array = {
                let heap = env.jvm.heap.borrow();
                let old_array = heap.array_heap_map.get(id).unwrap();
                JavaArray {
                    array_type: old_array.array_type.clone(),
                    values: old_array.values.clone(),
                }
            };
            let array_id = env.jvm.heap_store_array(new_array);
            Ok(Some(JavaValue::Array(array_id)))
        }
        JavaValue::Object(_) => {
            // TODO
            panic!("Java_java_lang_Object_clone");
        }
        _ => panic!(),
    }
}

#[allow(non_snake_case)]
fn Java_java_lang_Object_notifyAll(_: &JniEnv) -> RuntimeResult<Option<JavaValue>> {
    Ok(None)
}

pub fn initialize(cp: &mut Classpath) {
    register_jni!(
        cp,
        Java_java_lang_Object_registerNatives,
        Java_java_lang_Object_hashCode,
        Java_java_lang_Object_getClass,
        Java_java_lang_Object_clone,
        Java_java_lang_Object_notifyAll
    );
}
