use crate::{model::JavaValue, Classpath, JniEnv};

#[allow(non_snake_case)]
fn Java_java_lang_Double_doubleToRawLongBits(env: &JniEnv) -> Option<JavaValue> {
    let double = env.parameters[0].as_double().unwrap();
    Some(JavaValue::Long(double.to_bits() as i64))
}

#[allow(non_snake_case)]
fn Java_java_lang_Double_longBitsToDouble(env: &JniEnv) -> Option<JavaValue> {
    let long = env.parameters[0].as_long().unwrap();
    Some(JavaValue::Double(f64::from_bits(long as u64)))
}

pub fn initialize(cp: &mut Classpath) {
    register_jni!(
        cp,
        Java_java_lang_Double_doubleToRawLongBits,
        Java_java_lang_Double_longBitsToDouble
    );
}
