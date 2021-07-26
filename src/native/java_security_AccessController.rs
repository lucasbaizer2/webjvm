use crate::{model::JavaValue, Classpath, InvokeType, JniEnv};

#[allow(non_snake_case)]
fn Java_java_security_AccessController_doPrivileged(env: &JniEnv) -> Option<JavaValue> {
    let action = env.parameters[0]
        .as_object()
        .unwrap()
        .unwrap_or_else(|| env.throw_exception("java/lang/NullPointerException", None));
    Some(
        env.invoke_instance_method(
            InvokeType::Virtual,
            action,
            "java/security/PrivilegedAction",
            "run",
            "()Ljava/lang/Object;",
            &[],
        )
        .unwrap(),
    )
}

#[allow(non_snake_case)]
fn Java_java_security_AccessController_getStackAccessControlContext(
    _: &JniEnv,
) -> Option<JavaValue> {
    Some(JavaValue::Object(None))
}

pub fn initialize(cp: &mut Classpath) {
    register_jni!(
        cp,
        Java_java_security_AccessController_doPrivileged,
        Java_java_security_AccessController_getStackAccessControlContext
    );
}
