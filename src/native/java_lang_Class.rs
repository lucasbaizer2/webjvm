use classfile_parser::ClassAccessFlags;

use crate::{
    model::{JavaArrayType, JavaValue, MethodDescriptor, RuntimeResult},
    util::get_constant_string,
    Classpath, JniEnv,
};

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
    let class_name = env.get_internal_metadata(env.get_current_instance(), "class_name").unwrap().into_string();
    let non_internalized = class_name.replace("/", ".");
    let result = env.new_string(&non_internalized);
    Ok(Some(JavaValue::Object(Some(result))))
}

#[allow(non_snake_case)]
fn Java_java_lang_Class_isArray(env: &JniEnv) -> RuntimeResult<Option<JavaValue>> {
    let class_name = env.get_internal_metadata(env.get_current_instance(), "class_name").unwrap().into_string();
    Ok(Some(JavaValue::Boolean(class_name.starts_with('['))))
}

#[allow(non_snake_case)]
fn Java_java_lang_Class_getComponentType(env: &JniEnv) -> RuntimeResult<Option<JavaValue>> {
    let class_name = env.get_internal_metadata(env.get_current_instance(), "class_name").unwrap().into_string();
    if !class_name.starts_with('[') {
        return Ok(Some(JavaValue::Object(None)));
    }

    let mut component_name = &class_name[1..class_name.len()];
    if component_name.starts_with('L') && component_name.ends_with(';') {
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
    let class_object = env.get_class_object(class_id);
    Ok(Some(JavaValue::Object(Some(class_object))))
}

fn get_declared_methods(env: &JniEnv, constructors: bool) -> usize {
    let class_id = env.get_internal_metadata(env.get_current_instance(), "class_id").unwrap().into_usize();
    let class_file = env.get_class_file(class_id);
    let methods = &class_file.methods;

    let mut starting_offset = 0usize;
    let mut superclass = env.get_superclass(class_id);
    while superclass.is_some() {
        let sc_id = superclass.unwrap();
        starting_offset += env.get_class_file(sc_id).methods_count as usize;

        superclass = env.get_superclass(sc_id);
    }

    let method_class_type = match constructors {
        true => "java/lang/reflect/Constructor",
        false => "java/lang/reflect/Method",
    };
    let method_type_id = env.load_class(method_class_type, false);
    let result_array = env.new_array(
        JavaArrayType::Object(method_type_id),
        methods
            .iter()
            .filter(|method| match constructors {
                true => get_constant_string(&class_file.const_pool, method.name_index) == "<init>",
                false => get_constant_string(&class_file.const_pool, method.name_index) != "<clinit>",
            })
            .count(),
    );
    for (i, method) in methods.iter().enumerate() {
        let method_name = get_constant_string(&class_file.const_pool, method.name_index);
        if constructors && method_name != "<init>" {
            continue;
        }
        if !constructors && (method_name == "<init>" || method_name == "<clinit>") {
            continue;
        }
        let reflected_method = env.new_instance(method_type_id);
        let method_name_interned = env.new_interned_string(method_name);
        env.set_field(reflected_method, "clazz", JavaValue::Object(Some(env.get_current_instance())));
        env.set_field(reflected_method, "slot", JavaValue::Int(starting_offset as i32 + i as i32));
        if !constructors {
            env.set_field(reflected_method, "name", JavaValue::Object(Some(method_name_interned)));
        }
        env.set_field(reflected_method, "modifiers", JavaValue::Int(method.access_flags.bits() as i32));

        let signature = get_constant_string(&class_file.const_pool, method.descriptor_index);
        let descriptor = MethodDescriptor::new(signature).unwrap();

        let parameter_types = env.new_array(
            JavaArrayType::Object(env.load_class("java/lang/Class", false)),
            descriptor.argument_types.len(),
        );
        for i in 0..descriptor.argument_types.len() {
            let param_type_id = env.load_class(&descriptor.argument_types[i], false);
            let param_type_class = env.get_class_object(param_type_id);
            env.set_array_element(parameter_types, i, JavaValue::Object(Some(param_type_class)));
        }
        env.set_field(reflected_method, "parameterTypes", JavaValue::Array(parameter_types));

        if !constructors {
            let return_type_id = env.load_class(&descriptor.return_type, false);
            let return_type_class = env.get_class_object(return_type_id);
            env.set_field(reflected_method, "returnType", JavaValue::Object(Some(return_type_class)));
        }

        env.set_array_element(result_array, i, JavaValue::Object(Some(reflected_method)));
    }

    result_array
}

#[allow(non_snake_case)]
fn Java_java_lang_Class_getDeclaredConstructors0(env: &JniEnv) -> RuntimeResult<Option<JavaValue>> {
    let constructors = get_declared_methods(env, true);
    Ok(Some(JavaValue::Array(constructors)))
}

#[allow(non_snake_case)]
fn Java_java_lang_Class_getDeclaredFields0(env: &JniEnv) -> RuntimeResult<Option<JavaValue>> {
    let class_id = env.get_internal_metadata(env.get_current_instance(), "class_id").unwrap().into_usize();
    let class_file = env.get_class_file(class_id);
    let fields = &class_file.fields;

    let mut starting_offset = 0usize;
    let mut superclass = env.get_superclass(class_id);
    while superclass.is_some() {
        let sc_id = superclass.unwrap();
        starting_offset += env.get_class_file(sc_id).fields_count as usize;

        superclass = env.get_superclass(sc_id);
    }

    let field_type_id = env.load_class("java/lang/reflect/Field", false);
    let result_array = env.new_array(JavaArrayType::Object(field_type_id), fields.len());
    for (i, field) in fields.iter().enumerate() {
        let reflected_field = env.new_instance(field_type_id);
        let field_name = env.new_interned_string(get_constant_string(&class_file.const_pool, field.name_index));
        env.set_field(reflected_field, "clazz", JavaValue::Object(Some(env.get_current_instance())));
        env.set_field(reflected_field, "slot", JavaValue::Int(starting_offset as i32 + i as i32));
        env.set_field(reflected_field, "name", JavaValue::Object(Some(field_name)));

        let mut field_type_name = get_constant_string(&class_file.const_pool, field.descriptor_index).as_str();
        if field_type_name.starts_with('L') && field_type_name.ends_with(';') {
            field_type_name = &field_type_name[1..field_type_name.len() - 1];
        }

        let field_type_id = env.load_class(field_type_name, false);
        let field_type_class = env.get_class_object(field_type_id);
        env.set_field(reflected_field, "type", JavaValue::Object(Some(field_type_class)));
        env.set_field(reflected_field, "modifiers", JavaValue::Int(field.access_flags.bits() as i32));

        env.set_array_element(result_array, i, JavaValue::Object(Some(reflected_field)));
    }

    Ok(Some(JavaValue::Array(result_array)))
}

#[allow(non_snake_case)]
fn Java_java_lang_Class_isPrimitive(env: &JniEnv) -> RuntimeResult<Option<JavaValue>> {
    let class_name = env.get_internal_metadata(env.get_current_instance(), "class_name").unwrap().into_string();

    let is_primitive =
        matches!(class_name.chars().next().unwrap(), 'B' | 'S' | 'I' | 'J' | 'F' | 'D' | 'Z' | 'C' | 'V');
    Ok(Some(JavaValue::Boolean(is_primitive)))
}

#[allow(non_snake_case)]
fn Java_java_lang_Class_isAssignableFrom(env: &JniEnv) -> RuntimeResult<Option<JavaValue>> {
    let this_class = env.get_current_instance();
    let compare_class = env.parameters[1].as_object().unwrap().unwrap();

    let this_class_name = env.get_internal_metadata(this_class, "class_name").unwrap().into_string();
    let compare_class_id = env.get_internal_metadata(compare_class, "class_id").unwrap().into_usize();
    let is_assignable_from = env.jvm.is_assignable_from(&this_class_name, compare_class_id)?;

    Ok(Some(JavaValue::Boolean(is_assignable_from)))
}

#[allow(non_snake_case)]
fn Java_java_lang_Class_isInterface(env: &JniEnv) -> RuntimeResult<Option<JavaValue>> {
    let class_id = env.get_internal_metadata(env.get_current_instance(), "class_id").unwrap().into_usize();
    let class_file = env.get_class_file(class_id);
    Ok(Some(JavaValue::Boolean(class_file.access_flags.contains(ClassAccessFlags::INTERFACE))))
}

#[allow(non_snake_case)]
fn Java_java_lang_Class_getModifiers(env: &JniEnv) -> RuntimeResult<Option<JavaValue>> {
    let class_id = env.get_internal_metadata(env.get_current_instance(), "class_id").unwrap().into_usize();
    let class_file = env.get_class_file(class_id);
    Ok(Some(JavaValue::Int(class_file.access_flags.bits() as i32)))
}

#[allow(non_snake_case)]
fn Java_java_lang_Class_getSuperclass(env: &JniEnv) -> RuntimeResult<Option<JavaValue>> {
    let class_id = env.get_internal_metadata(env.get_current_instance(), "class_id").unwrap().into_usize();

    let heap = env.jvm.heap.borrow();
    let id = match heap.loaded_classes[class_id].superclass_id {
        Some(id) => id,
        None => return Ok(Some(JavaValue::Object(None))),
    };
    let obj_id = heap.loaded_classes[id].class_object_id;

    Ok(Some(JavaValue::Object(Some(obj_id))))
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
        Java_java_lang_Class_getDeclaredConstructors0,
        Java_java_lang_Class_getDeclaredFields0,
        Java_java_lang_Class_isPrimitive,
        Java_java_lang_Class_isAssignableFrom,
        Java_java_lang_Class_isInterface,
        Java_java_lang_Class_getModifiers,
        Java_java_lang_Class_getSuperclass
    );
}
