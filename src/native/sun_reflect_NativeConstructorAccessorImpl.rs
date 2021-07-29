use crate::{
    model::{JavaValue, RuntimeResult},
    Classpath, InvokeType, JniEnv,
};

#[allow(non_snake_case)]
fn Java_sun_reflect_NativeConstructorAccessorImpl_newInstance0(env: &JniEnv) -> RuntimeResult<Option<JavaValue>> {
    let constructor = env.parameters[0].as_object().unwrap().unwrap();
    let args = env.parameters[1].as_array();

    let constructor_declaring_class_obj = env.get_field(constructor, "clazz").as_object().unwrap().unwrap();
    let constructor_declaring_class = env.get_internal_metadata(constructor_declaring_class_obj, "class_name").unwrap();
    let constructor_descriptor = "()V";

    let params = match args {
        Ok(args) => {
            let args_len = env.get_array_length(args);
            let mut params = Vec::with_capacity(args_len);
            for i in 0..args_len {
                params.push(env.get_array_element(args, i));
            }
            params
        }
        Err(_) => Vec::new(),
    };

    let new_instance = env.new_instance(&constructor_declaring_class);
    env.invoke_instance_method(
        InvokeType::Special,
        new_instance,
        &constructor_declaring_class,
        "<init>",
        &constructor_descriptor,
        &params,
    )?;

    Ok(Some(JavaValue::Object(Some(new_instance))))
}

pub fn initialize(cp: &mut Classpath) {
    register_jni!(cp, Java_sun_reflect_NativeConstructorAccessorImpl_newInstance0);
}
