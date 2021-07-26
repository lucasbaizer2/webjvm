use crate::{Classpath, JniEnv, model::{JavaValue, RuntimeResult}};

#[allow(non_snake_case)]
fn Java_sun_misc_VM_initialize(_: &JniEnv) -> RuntimeResult<Option<JavaValue>> {
    Ok(None)
}

pub fn initialize(cp: &mut Classpath) {
    register_jni!(cp, Java_sun_misc_VM_initialize);
}
