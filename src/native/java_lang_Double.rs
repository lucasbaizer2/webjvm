use crate::{
    model::{JavaValue, RuntimeResult},
    Classpath, JniEnv,
};

#[allow(non_snake_case)]
fn Java_java_lang_Double_doubleToRawLongBits(env: &JniEnv) -> RuntimeResult<Option<JavaValue>> {
    let double = env.parameters[0].as_double().unwrap();
    Ok(Some(JavaValue::Long(double.to_bits() as i64)))
}

#[allow(non_snake_case)]
fn Java_java_lang_Double_longBitsToDouble(env: &JniEnv) -> RuntimeResult<Option<JavaValue>> {
    let long = env.parameters[0].as_long().unwrap();
    Ok(Some(JavaValue::Double(f64::from_bits(long as u64))))
}

pub fn initialize(cp: &mut Classpath) {
    register_jni!(cp, Java_java_lang_Double_doubleToRawLongBits, Java_java_lang_Double_longBitsToDouble);
}
