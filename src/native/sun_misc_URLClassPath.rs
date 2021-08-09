use crate::{
    exec::env::JniEnv,
    model::{JavaValue, RuntimeResult},
    Classpath,
};

#[allow(non_snake_case)]
fn Java_sun_misc_URLClassPath_getLookupCacheURLs(_: &JniEnv) -> RuntimeResult<Option<JavaValue>> {
    Ok(Some(JavaValue::Object(None)))
}

pub fn initialize(cp: &mut Classpath) {
    register_jni!(cp, Java_sun_misc_URLClassPath_getLookupCacheURLs);
}
