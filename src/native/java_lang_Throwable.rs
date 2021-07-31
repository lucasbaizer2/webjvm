use crate::{
    exec::env::JniEnv,
    model::{JavaArrayType, JavaValue, RuntimeResult},
    Classpath, InvokeType,
};

#[allow(non_snake_case)]
fn Java_java_lang_Throwable_fillInStackTrace(env: &JniEnv) -> RuntimeResult<Option<JavaValue>> {
    let csf_len = env.jvm.get_stack_depth();
    let object_type = env.get_class_id("java/lang/StackTraceElement");
    let stacktrace = env.new_array(JavaArrayType::Object(object_type), csf_len - 1);
    for i in 1..csf_len {
        let (class_name, method_name, line_number) = {
            let csf = env.jvm.call_stack_frames.borrow();
            let frame = &csf[csf_len - i - 1];
            let class_name = env.new_string(&frame.container_class);
            let method_name = env.new_string(&frame.container_method);
            let line_number = match frame.is_native_frame {
                true => -2,
                false => -1,
            };

            (class_name, method_name, line_number)
        };

        let ste_class_id = env.get_class_id("java/lang/StackTraceElement");
        let ste = env.new_instance(ste_class_id);
        env.invoke_instance_method(
            InvokeType::Special,
            ste,
            ste_class_id,
            "<init>",
            "(Ljava/lang/String;Ljava/lang/String;Ljava/lang/String;I)V",
            &[
                JavaValue::Object(Some(class_name)),
                JavaValue::Object(Some(method_name)),
                JavaValue::Object(None),
                JavaValue::Int(line_number),
            ],
        )?;
        env.set_array_element(stacktrace, i - 1, JavaValue::Object(Some(ste)));
    }
    env.set_field(env.get_current_instance(), "stackTrace", JavaValue::Array(stacktrace));

    Ok(Some(JavaValue::Object(Some(env.get_current_instance()))))
}

pub fn initialize(cp: &mut Classpath) {
    register_jni!(cp, Java_java_lang_Throwable_fillInStackTrace);
}
