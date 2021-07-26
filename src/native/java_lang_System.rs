use crate::{model::JavaValue, util::log, Classpath, InvokeType, JniEnv};

#[allow(non_snake_case)]
fn Java_java_lang_System_registerNatives(_: &JniEnv) -> Option<JavaValue> {
    log("Registered System natives!");

    None
}

#[allow(non_snake_case)]
fn Java_java_lang_System_currentTimeMillis(_: &JniEnv) -> Option<JavaValue> {
    let now = js_sys::Date::now();
    Some(JavaValue::Long(now as i64))
}

#[allow(non_snake_case)]
fn Java_java_lang_System_nanoTime(_: &JniEnv) -> Option<JavaValue> {
    Some(JavaValue::Long(0))
}

/*#[allow(non_snake_case)]
fn Java_java_lang_System_arraycopy(_: &JniEnv) -> Option<JavaValue> {
    None
}*/

#[allow(non_snake_case)]
fn Java_java_lang_System_initProperties(env: &JniEnv) -> Option<JavaValue> {
    let default_properties = &[
        ("java.version", "1.8.0_211"),
        ("java.vendor", "webjvm"),
        ("java.vendor.url", "https://github.com/LucasBaizer/webjvm"),
        ("java.home", "/dev/null"),
        ("java.class.version", "52"),
        ("java.class.path", ""),
        ("os.name", "web"),
        ("os.arch", "wasm32"),
        ("os.version", "1.0"),
        ("file.separator", "/"),
        ("path.separator", ":"),
        ("line.separator", "\n"),
        ("user.name", "web"),
        ("user.home", "/dev/null"),
        ("user.dir", "/dev/null"),
        ("sun.nio.PageAlignDirectMemory", "false")
    ];

    let prop_map = env.parameters[0].as_object().unwrap().unwrap();
    for default_property in default_properties {
        let key_str = env.new_string(default_property.0);
        let value_str = env.new_string(default_property.1);
        env.invoke_instance_method(
            InvokeType::Virtual,
            prop_map,
            "java/util/Properties",
            "setProperty",
            "(Ljava/lang/String;Ljava/lang/String;)Ljava/lang/Object;",
            &[
                JavaValue::Object(Some(key_str)),
                JavaValue::Object(Some(value_str)),
            ],
        );
    }

    Some(JavaValue::Object(Some(prop_map)))
}

pub fn initialize(cp: &mut Classpath) {
    register_jni!(
        cp,
        Java_java_lang_System_registerNatives,
        Java_java_lang_System_currentTimeMillis,
        Java_java_lang_System_nanoTime,
        Java_java_lang_System_initProperties
    );
}
