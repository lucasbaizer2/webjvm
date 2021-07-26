use crate::{model::JavaValue, Classpath, JniEnv};

#[allow(non_snake_case)]
fn Java_java_lang_Float_floatToRawIntBits(env: &JniEnv) -> Option<JavaValue> {
    let float = env.parameters[0].as_float().unwrap();
    Some(JavaValue::Int(float.to_bits() as i32))
}

pub fn initialize(cp: &mut Classpath) {
    register_jni!(cp, Java_java_lang_Float_floatToRawIntBits);
}
