use crate::{Classpath, JniEnv, model::{JavaArrayType, JavaValue, RuntimeResult}, util::{get_constant_string, log_error}};

#[allow(non_snake_case)]
fn Java_java_lang_Class_registerNatives(_: &JniEnv) -> RuntimeResult<Option<JavaValue>> {
    Ok(None)
}

#[allow(non_snake_case)]
fn Java_java_lang_Class_desiredAssertionStatus0(_: &JniEnv) -> RuntimeResult<Option<JavaValue>> {
    Ok(Some(JavaValue::Boolean(false)))
}

#[allow(non_snake_case)]
fn Java_java_lang_Class_getName0(env: &JniEnv) -> RuntimeResult<Option<JavaValue>> {
    let class_name = env
        .get_internal_metadata(env.get_current_instance(), "class_name")
        .unwrap();
    let non_internalized = class_name.replace("/", ".");
    let result = env.new_string(&non_internalized);
    Ok(Some(JavaValue::Object(Some(result))))
}

#[allow(non_snake_case)]
fn Java_java_lang_Class_isArray(env: &JniEnv) -> RuntimeResult<Option<JavaValue>> {
    let class_name = env
        .get_internal_metadata(env.get_current_instance(), "class_name")
        .unwrap();
    Ok(Some(JavaValue::Boolean(class_name.starts_with("["))))
}

#[allow(non_snake_case)]
fn Java_java_lang_Class_getComponentType(env: &JniEnv) -> RuntimeResult<Option<JavaValue>> {
    let class_name = env
        .get_internal_metadata(env.get_current_instance(), "class_name")
        .unwrap();
    if !class_name.starts_with("[") {
        return Ok(Some(JavaValue::Object(None)));
    }

    let mut component_name = &class_name[1..class_name.len()];
    if component_name.starts_with("L") && component_name.ends_with(";") {
        component_name = &component_name[1..component_name.len() - 1];
    }
    let component_class_id = env.get_class_id(component_name);
    let class_instance = env.get_class_object(component_class_id);

    Ok(Some(JavaValue::Object(Some(class_instance))))
}

#[allow(non_snake_case)]
fn Java_java_lang_Class_getPrimitiveClass(env: &JniEnv) -> RuntimeResult<Option<JavaValue>> {
    let class_name_id = env.get_current_instance();
    let class_name = env.get_string(class_name_id);
    let signature_name = match class_name.as_str() {
        "byte" => "B",
        "short" => "S",
        "int" => "I",
        "long" => "J",
        "float" => "F",
        "double" => "D",
        "char" => "C",
        "boolean" => "Z",
        x => return Err(env.throw_exception("java/lang/IllegalArgumentException", Some(x))),
    };

    let primitive_class_id = env.get_class_id(signature_name);
    let primitive_class = env.get_class_object(primitive_class_id);

    Ok(Some(JavaValue::Object(Some(primitive_class))))
}

#[allow(non_snake_case)]
fn Java_java_lang_Class_forName0(env: &JniEnv) -> RuntimeResult<Option<JavaValue>> {
    let name = env.get_string(match env.parameters[0].as_object().unwrap() {
        Some(id) => id,
        None => return Err(env.throw_exception("java/lang/NullPointerException", None)),
    });
    let initialize = env.parameters[1].as_boolean().unwrap();
    let class_id = env.load_class(&name.replace(".", "/"), initialize);
    Ok(Some(JavaValue::Object(Some(class_id))))
}

#[allow(non_snake_case)]
fn Java_java_lang_Class_getDeclaredFields0(env: &JniEnv) -> RuntimeResult<Option<JavaValue>> {
    let class_name = env
        .get_internal_metadata(env.get_current_instance(), "class_name")
        .unwrap();
    let class_file = env.get_class_file(&class_name);
    let fields = &class_file.fields;

    let field_type_id = env.get_class_id("java/lang/reflect/Field");
    let result_array = env.new_array(JavaArrayType::Object(field_type_id), fields.len());
    for i in 0..fields.len() {
        let field = &fields[i];
        log_error(&format!("REFLECT: {}: {:?}", class_name, field));
        let reflected_field = env.new_instance("java/lang/reflect/Field");
        env.set_field(
            reflected_field,
            "clazz",
            JavaValue::Object(Some(env.get_current_instance())),
        );
        env.set_field(reflected_field, "slot", JavaValue::Int(i as i32));
        env.set_field(
            reflected_field,
            "name",
            JavaValue::Object(Some(env.new_interned_string(get_constant_string(
                &class_file.const_pool,
                field.name_index,
            )))),
        );

        let mut field_type_name =
            get_constant_string(&class_file.const_pool, field.descriptor_index).as_str();
        if field_type_name.starts_with("L") && field_type_name.ends_with(";") {
            field_type_name = &field_type_name[1..field_type_name.len() - 1];
        }

        let field_type_id = env.get_class_id(field_type_name);
        let field_type_class = env.get_class_object(field_type_id);
        env.set_field(
            reflected_field,
            "type",
            JavaValue::Object(Some(field_type_class)),
        );

        env.set_field(
            reflected_field,
            "modifiers",
            JavaValue::Int(field.access_flags.bits() as i32),
        );

        env.set_array_element(result_array, i, JavaValue::Object(Some(reflected_field)));
    }

    Ok(Some(JavaValue::Array(result_array)))
}

pub fn initialize(cp: &mut Classpath) {
    register_jni!(
        cp,
        Java_java_lang_Class_registerNatives,
        Java_java_lang_Class_desiredAssertionStatus0,
        Java_java_lang_Class_getName0,
        Java_java_lang_Class_isArray,
        Java_java_lang_Class_getComponentType,
        Java_java_lang_Class_getPrimitiveClass,
        Java_java_lang_Class_forName0,
        Java_java_lang_Class_getDeclaredFields0
    );
}
