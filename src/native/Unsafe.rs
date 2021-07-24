use crate::{exec::JavaValue, util::log, Classpath, JniEnv};

#[allow(non_snake_case)]
fn Java_sun_misc_Unsafe_registerNatives(_: &JniEnv) -> Option<JavaValue> {
    log("Registered Unsafe natives!");

    None
}

pub fn initialize(cp: &mut Classpath) {
    register_jni!(cp, Java_sun_misc_Unsafe_registerNatives);
}
