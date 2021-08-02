use crate::{
    model::{JavaValue, RuntimeResult},
    Classpath, InvokeType, JniEnv,
};

#[allow(non_snake_case)]
fn Java_java_security_AccessController_doPrivileged(env: &JniEnv) -> RuntimeResult<Option<JavaValue>> {
    let action = match env.parameters[0].as_object().unwrap() {
        Some(id) => id,
        None => return Err(env.throw_exception("java/lang/NullPointerException", None)),
    };
    Ok(Some(
        env.invoke_instance_method(
            InvokeType::Virtual,
            action,
            env.get_class_id("java/security/PrivilegedAction")?,
            "run",
            "()Ljava/lang/Object;",
            &[],
        )?
        .unwrap(),
    ))
}

#[allow(non_snake_case)]
fn Java_java_security_AccessController_getStackAccessControlContext(_: &JniEnv) -> RuntimeResult<Option<JavaValue>> {
    Ok(Some(JavaValue::Object(None)))
}

pub fn initialize(cp: &mut Classpath) {
    register_jni!(
        cp,
        Java_java_security_AccessController_doPrivileged,
        Java_java_security_AccessController_getStackAccessControlContext
    );
}
