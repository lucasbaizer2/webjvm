use crate::{
    exec::env::JniEnv,
    model::{JavaArrayType, JavaValue, RuntimeResult},
    Classpath,
};

#[allow(non_snake_case)]
fn Java_java_lang_reflect_Array_newArray(env: &JniEnv) -> RuntimeResult<Option<JavaValue>> {
    let component_type_name =
        env.get_internal_metadata(env.get_current_instance()?, "class_name").unwrap().into_string();
    let length = env.parameters[1].as_int().unwrap();

    if length < 0 {
        return Err(env.throw_exception("java/lang/NegativeArraySizeException", None));
    }

    let array_id = env.new_array(
        match component_type_name.as_str() {
            "byte" => JavaArrayType::Byte,
            "short" => JavaArrayType::Short,
            "int" => JavaArrayType::Int,
            "long" => JavaArrayType::Long,
            "float" => JavaArrayType::Float,
            "double" => JavaArrayType::Double,
            "char" => JavaArrayType::Char,
            "boolean" => JavaArrayType::Boolean,
            other => {
                if other.starts_with('[') {
                    unimplemented!("multidimensional arrays not yet implemented");
                } else {
                    JavaArrayType::Object(
                        env.get_internal_metadata(env.get_current_instance()?, "class_id").unwrap().into_usize(),
                    )
                }
            }
        },
        length as usize,
    );

    Ok(Some(JavaValue::Array(array_id)))
}

pub fn initialize(cp: &mut Classpath) {
    register_jni!(cp, Java_java_lang_reflect_Array_newArray);
}
