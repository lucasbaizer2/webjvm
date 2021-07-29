use crate::{
    exec::env::JniEnv,
    model::{JavaValue, RuntimeResult},
    Classpath,
};

#[allow(non_snake_case)]
fn Java_java_util_concurrent_atomic_AtomicLong_VMSupportsCS8(_: &JniEnv) -> RuntimeResult<Option<JavaValue>> {
    Ok(Some(JavaValue::Boolean(false)))
}

pub fn initialize(cp: &mut Classpath) {
    register_jni!(cp, Java_java_util_concurrent_atomic_AtomicLong_VMSupportsCS8);
}
