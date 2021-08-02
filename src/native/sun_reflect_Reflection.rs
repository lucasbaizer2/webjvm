use crate::{
    model::{JavaValue, RuntimeResult},
    Classpath, JniEnv,
};

#[allow(non_snake_case)]
fn Java_sun_reflect_Reflection_getCallerClass(env: &JniEnv) -> RuntimeResult<Option<JavaValue>> {
    let call_depth = match env.parameters.len() {
        0 => 2,
        _ => env.parameters[0].as_int().unwrap(),
    };
    let frame = &env.stack_trace[env.stack_trace.len() - call_depth as usize - 1];
    let class_id = env.get_class_id(&frame.class_name)?;
    let class_obj = env.get_class_object(class_id);
    Ok(Some(JavaValue::Object(Some(class_obj))))
}

#[allow(non_snake_case)]
fn Java_sun_reflect_Reflection_getClassAccessFlags(env: &JniEnv) -> RuntimeResult<Option<JavaValue>> {
    let class_obj = env.parameters[0].as_object().unwrap().unwrap();
    let class_id = env.get_internal_metadata(class_obj, "class_id").unwrap().into_usize();
    let class_file = env.get_class_file(class_id);
    Ok(Some(JavaValue::Int(class_file.access_flags.bits() as i32 & 0x1FFF)))
}

pub fn initialize(cp: &mut Classpath) {
    register_jni!(cp, Java_sun_reflect_Reflection_getCallerClass, Java_sun_reflect_Reflection_getClassAccessFlags);
}
