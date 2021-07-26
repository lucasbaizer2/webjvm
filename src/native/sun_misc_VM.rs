use crate::{model::JavaValue, Classpath, JniEnv};

#[allow(non_snake_case)]
fn Java_sun_misc_VM_initialize(_: &JniEnv) -> Option<JavaValue> {
    None
}

pub fn initialize(cp: &mut Classpath) {
    register_jni!(cp, Java_sun_misc_VM_initialize);
}
