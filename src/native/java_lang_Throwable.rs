use crate::{
    exec::env::JniEnv,
    model::{JavaArrayType, JavaValue, RuntimeResult},
    Classpath, InvokeType,
};

#[allow(non_snake_case)]
fn Java_java_lang_Throwable_getStackTraceElement(env: &JniEnv) -> RuntimeResult<Option<JavaValue>> {
    let arr = env.get_field(env.get_current_instance()?, "backtrace").as_array().unwrap();
    let index = env.parameters[1].as_int().unwrap();
    let val = env.get_array_element(arr, index as usize);
    Ok(Some(val))
}

#[allow(non_snake_case)]
fn Java_java_lang_Throwable_getStackTraceDepth(env: &JniEnv) -> RuntimeResult<Option<JavaValue>> {
    let arr = env.get_field(env.get_current_instance()?, "backtrace").as_array().unwrap();
    Ok(Some(JavaValue::Int(env.get_array_length(arr) as i32)))
}

#[allow(non_snake_case)]
fn Java_java_lang_Throwable_fillInStackTrace(env: &JniEnv) -> RuntimeResult<Option<JavaValue>> {
    let csf_len = env.jvm.get_stack_depth();
    let object_type = env.get_class_id("java/lang/StackTraceElement")?;
    let stacktrace = env.new_array(JavaArrayType::Object(object_type), csf_len - 6);
    for i in 1..csf_len - 5 {
        let (class_name, method_name, line_number) = {
            let csf = env.jvm.call_stack_frames.borrow();
            let frame = &csf[csf_len - i - 5];
            let class_name = env.new_string(&frame.container_class.replace("/", "."));
            let method_name = env.new_string(&frame.container_method[0..frame.container_method.find('(').unwrap()]);
            let line_number = match frame.is_native_frame {
                true => -2,
                false => -1,
            };

            (class_name, method_name, line_number)
        };

        let ste = env.new_instance(object_type)?;
        env.invoke_instance_method(
            InvokeType::Special,
            ste,
            object_type,
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
    env.set_field(env.get_current_instance()?, "backtrace", JavaValue::Array(stacktrace));

    Ok(Some(JavaValue::Object(Some(env.get_current_instance()?))))
}

pub fn initialize(cp: &mut Classpath) {
    register_jni!(
        cp,
        Java_java_lang_Throwable_getStackTraceDepth,
        Java_java_lang_Throwable_getStackTraceElement,
        Java_java_lang_Throwable_fillInStackTrace
    );
}
