use crate::{
    model::{JavaValue, RuntimeResult},
    Classpath, JniEnv,
};

#[allow(non_snake_case)]
fn Java_java_lang_Float_floatToRawIntBits(env: &JniEnv) -> RuntimeResult<Option<JavaValue>> {
    let float = env.parameters[0].as_float().unwrap();
    Ok(Some(JavaValue::Int(float.to_bits() as i32)))
}

pub fn initialize(cp: &mut Classpath) {
    register_jni!(cp, Java_java_lang_Float_floatToRawIntBits);
}
