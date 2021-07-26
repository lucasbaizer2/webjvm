use crate::{Classpath, JniEnv, model::{JavaValue, RuntimeResult}};

#[allow(non_snake_case)]
fn Java_sun_reflect_Reflection_getCallerClass(env: &JniEnv) -> RuntimeResult<Option<JavaValue>> {
    let call_depth = match env.parameters.len() {
        0 => 2,
        _ => env.parameters[0].as_int().unwrap(),
    };
    let frame = &env.stack_trace[env.stack_trace.len() - call_depth as usize - 1];
    let class_id = env.get_class_id(&frame.class_name);
    let class_obj = env.get_class_object(class_id);
    Ok(Some(JavaValue::Object(Some(class_obj))))
}

pub fn initialize(cp: &mut Classpath) {
    register_jni!(cp, Java_sun_reflect_Reflection_getCallerClass);
}
