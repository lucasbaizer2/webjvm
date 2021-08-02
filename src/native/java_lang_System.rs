use crate::{
    model::{JavaValue, RuntimeResult},
    Classpath, InvokeType, JniEnv,
};

#[allow(non_snake_case)]
fn Java_java_lang_System_registerNatives(_: &JniEnv) -> RuntimeResult<Option<JavaValue>> {
    Ok(None)
}

#[allow(non_snake_case)]
fn Java_java_lang_System_currentTimeMillis(_: &JniEnv) -> RuntimeResult<Option<JavaValue>> {
    #[cfg(target_arch = "wasm32")]
    return Ok(Some(JavaValue::Long(js_sys::Date::now() as i64)));

    #[cfg(not(target_arch = "wasm32"))]
    {
        use std::time::{SystemTime, UNIX_EPOCH};
        return Ok(Some(JavaValue::Long(SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_millis() as i64)));
    }
}

#[allow(non_snake_case)]
fn Java_java_lang_System_nanoTime(_: &JniEnv) -> RuntimeResult<Option<JavaValue>> {
    Ok(Some(JavaValue::Long(0)))
}

#[allow(non_snake_case)]
fn Java_java_lang_System_arraycopy(env: &JniEnv) -> RuntimeResult<Option<JavaValue>> {
    let src = env.parameters[0].as_array().unwrap();
    let srcPos = env.parameters[1].as_int().unwrap();
    let dest = env.parameters[2].as_array().unwrap();
    let destPos = env.parameters[3].as_int().unwrap();
    let length = env.parameters[4].as_int().unwrap();

    for i in 0..length {
        let value = env.get_array_element(src, (srcPos + i) as usize);
        env.set_array_element(dest, (destPos + i) as usize, value);
    }

    Ok(None)
}

#[allow(non_snake_case)]
fn Java_java_lang_System_initProperties(env: &JniEnv) -> RuntimeResult<Option<JavaValue>> {
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
        ("sun.nio.PageAlignDirectMemory", "false"),
        ("sun.stdout.encoding", "UTF-8"),
        ("sun.stderr.encoding", "UTF-8"),
    ];

    let props_class = env.get_class_id("java/util/Properties");
    let prop_map = env.parameters[0].as_object().unwrap().unwrap();
    for default_property in default_properties {
        let key_str = env.new_string(default_property.0);
        let value_str = env.new_string(default_property.1);
        env.invoke_instance_method(
            InvokeType::Virtual,
            prop_map,
            props_class,
            "setProperty",
            "(Ljava/lang/String;Ljava/lang/String;)Ljava/lang/Object;",
            &[JavaValue::Object(Some(key_str)), JavaValue::Object(Some(value_str))],
        )?;
    }

    Ok(Some(JavaValue::Object(Some(prop_map))))
}

#[allow(non_snake_case)]
fn Java_java_lang_System_setIn0(env: &JniEnv) -> RuntimeResult<Option<JavaValue>> {
    let stdin = env.parameters[0].clone();
    env.set_static_field("java/lang/System", "in", stdin);

    Ok(None)
}

#[allow(non_snake_case)]
fn Java_java_lang_System_setOut0(env: &JniEnv) -> RuntimeResult<Option<JavaValue>> {
    let stdout = env.parameters[0].clone();
    env.set_static_field("java/lang/System", "out", stdout);

    Ok(None)
}

#[allow(non_snake_case)]
fn Java_java_lang_System_setErr0(env: &JniEnv) -> RuntimeResult<Option<JavaValue>> {
    let stderr = env.parameters[0].clone();
    env.set_static_field("java/lang/System", "err", stderr);

    Ok(None)
}

#[allow(non_snake_case)]
fn Java_java_lang_System_mapLibraryName(env: &JniEnv) -> RuntimeResult<Option<JavaValue>> {
    let library_name = env.get_string(env.parameters[0].as_object().unwrap().unwrap());
    let linux_style = format!("lib{}.so", library_name);
    let java_str = env.new_string(&linux_style);

    Ok(Some(JavaValue::Object(Some(java_str))))
}

pub fn initialize(cp: &mut Classpath) {
    register_jni!(
        cp,
        Java_java_lang_System_registerNatives,
        Java_java_lang_System_currentTimeMillis,
        Java_java_lang_System_nanoTime,
        Java_java_lang_System_initProperties,
        Java_java_lang_System_arraycopy,
        Java_java_lang_System_setIn0,
        Java_java_lang_System_setOut0,
        Java_java_lang_System_setErr0,
        Java_java_lang_System_mapLibraryName
    );
}
