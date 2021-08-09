use crate::{
    model::{JavaArray, JavaArrayType, JavaValue, RuntimeResult},
    Classpath, JniEnv,
};

#[allow(non_snake_case)]
fn Java_java_lang_Object_registerNatives(_: &JniEnv) -> RuntimeResult<Option<JavaValue>> {
    Ok(None)
}

#[allow(non_snake_case)]
fn Java_java_lang_Object_hashCode(env: &JniEnv) -> RuntimeResult<Option<JavaValue>> {
    env.invoke_static_method(
        env.get_class_id("java/lang/System")?,
        "identityHashCode",
        "(Ljava/lang/Object;)I",
        &[JavaValue::Object(Some(env.get_current_instance()?))],
    )
}

#[allow(non_snake_case)]
fn Java_java_lang_Object_getClass(env: &JniEnv) -> RuntimeResult<Option<JavaValue>> {
    if env.parameters[0].is_array() {
        let array_type = {
            let heap = env.jvm.heap.borrow();
            heap.array_heap_map[&env.parameters[0].as_array().unwrap()].array_type.clone()
        };
        let type_name = match array_type {
            JavaArrayType::Byte => String::from("[B"),
            JavaArrayType::Short => String::from("[S"),
            JavaArrayType::Int => String::from("[I"),
            JavaArrayType::Long => String::from("[J"),
            JavaArrayType::Float => String::from("[F"),
            JavaArrayType::Double => String::from("[D"),
            JavaArrayType::Char => String::from("[C"),
            JavaArrayType::Boolean => String::from("[Z"),
            JavaArrayType::Object(obj_type) => {
                let heap = env.jvm.heap.borrow();
                format!("[L{};", heap.loaded_classes[obj_type].java_type)
            }
            JavaArrayType::Array(_) => unimplemented!("getClass() on multidimensional arrays not yet implemented"),
        };
        let type_obj = env.get_class_object(env.get_class_id(&type_name)?);
        return Ok(Some(JavaValue::Object(Some(type_obj))));
    }
    let type_name = env.get_object_type_name(env.get_current_instance()?);
    let type_obj = env.get_class_object(env.get_class_id(&type_name)?);

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
