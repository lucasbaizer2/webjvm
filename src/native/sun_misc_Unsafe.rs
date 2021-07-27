use std::mem::size_of;

use crate::{Classpath, InvokeType, JniEnv, model::{JavaValue, RuntimeResult}, util::{log, log_error}};

const ADDRESS_SIZE: i32 = size_of::<usize>() as i32;

#[allow(non_snake_case)]
fn Java_sun_misc_Unsafe_registerNatives(_: &JniEnv) -> RuntimeResult<Option<JavaValue>> {
    log("Registered Unsafe natives!");

    Ok(None)
}

#[allow(non_snake_case)]
fn Java_sun_misc_Unsafe_arrayBaseOffset(_: &JniEnv) -> RuntimeResult<Option<JavaValue>> {
    Ok(Some(JavaValue::Long(0)))
}

#[allow(non_snake_case)]
fn Java_sun_misc_Unsafe_arrayIndexScale(env: &JniEnv) -> RuntimeResult<Option<JavaValue>> {
    let array_class = &env.parameters[1].as_object().unwrap().unwrap();
    let scale = match env
        .invoke_instance_method(
            InvokeType::Virtual,
            *array_class,
            "java/lang/Class",
            "getComponentType",
            "()Ljava/lang/Class;",
            &[],
        )?
        .unwrap()
    {
        JavaValue::Object(obj) => match obj {
            Some(component_type) => match env
                .get_internal_metadata(component_type, "class_name")
                .unwrap()
                .as_str()
            {
                "byte" => 1,
                "short" => 2,
                "int" => 4,
                "long" => 8,
                "float" => 4,
                "double" => 8,
                "char" => 2,
                "boolean" => 1,
                _ => ADDRESS_SIZE,
            },
            None => {
                return Err(env.throw_exception(
                    "java/lang/InvalidClassCastException",
                    Some("expecting array type"),
                ))
            }
        },
        _ => panic!(),
    };

    Ok(Some(JavaValue::Int(scale)))
}

#[allow(non_snake_case)]
fn Java_sun_misc_Unsafe_addressSize(_: &JniEnv) -> RuntimeResult<Option<JavaValue>> {
    Ok(Some(JavaValue::Int(ADDRESS_SIZE)))
}

#[allow(non_snake_case)]
fn Java_sun_misc_Unsafe_objectFieldOffset(env: &JniEnv) -> RuntimeResult<Option<JavaValue>> {
    let field_obj = env.parameters[1].as_object().unwrap().unwrap();
    let name_id = env
        .get_field(field_obj, "name")
        .as_object()
        .unwrap()
        .unwrap();
    let name = env.get_string(name_id);

    let heap = env.jvm.heap.borrow();
    let internal_obj = heap.object_heap_map.get(&field_obj).unwrap();

    let mut keys: Vec<&String> = internal_obj.instance_fields.keys().collect();
    keys.sort();

    log_error(&format!("{:?}", keys));

    let pos = keys.iter().position(|x| x == &&name).unwrap();
    Ok(Some(JavaValue::Long(pos as i64)))
}

#[allow(non_snake_case)]
fn Java_sun_misc_Unsafe_compareAndSwapObject(env: &JniEnv) -> RuntimeResult<Option<JavaValue>> {
    let obj = env.parameters[1].as_object().unwrap().unwrap();
    let offset = env.parameters[2].as_long().unwrap();
    let expect = env.parameters[4].as_object().unwrap();
    let update = env.parameters[5].as_object().unwrap();

    let mut heap = env.jvm.heap.borrow_mut();
    let internal_obj = heap.object_heap_map.get_mut(&obj).unwrap();

    let mut keys: Vec<&String> = internal_obj.instance_fields.keys().collect();
    keys.sort();

    let key = keys[offset as usize].clone();
    let current_value = internal_obj.instance_fields.get(&key).unwrap();
    if current_value.as_object().unwrap() == expect {
        internal_obj
            .instance_fields
            .insert(key, JavaValue::Object(update));
        Ok(Some(JavaValue::Boolean(true)))
    } else {
        Ok(Some(JavaValue::Boolean(false)))
    }

    // let object_type = env.get_object_type_name(obj);
}

pub fn initialize(cp: &mut Classpath) {
    register_jni!(
        cp,
        Java_sun_misc_Unsafe_registerNatives,
        Java_sun_misc_Unsafe_arrayBaseOffset,
        Java_sun_misc_Unsafe_arrayIndexScale,
        Java_sun_misc_Unsafe_addressSize,
        Java_sun_misc_Unsafe_objectFieldOffset,
        Java_sun_misc_Unsafe_compareAndSwapObject
    );
}
