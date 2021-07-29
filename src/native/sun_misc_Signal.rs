use crate::{
    model::{JavaValue, RuntimeResult},
    Classpath, JniEnv,
};

#[allow(non_snake_case)]
fn Java_sun_misc_Signal_findSignal(_: &JniEnv) -> RuntimeResult<Option<JavaValue>> {
    Ok(Some(JavaValue::Int(0)))
}

#[allow(non_snake_case)]
fn Java_sun_misc_Signal_handle0(_: &JniEnv) -> RuntimeResult<Option<JavaValue>> {
    Ok(Some(JavaValue::Long(0)))
}

pub fn initialize(cp: &mut Classpath) {
    register_jni!(cp, Java_sun_misc_Signal_findSignal, Java_sun_misc_Signal_handle0);
}
