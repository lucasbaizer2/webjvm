use std::mem::size_of;

use crate::{model::JavaValue, util::log, Classpath, InvokeType, JniEnv};

const ADDRESS_SIZE: i32 = size_of::<usize>() as i32;

#[allow(non_snake_case)]
fn Java_sun_misc_Unsafe_registerNatives(_: &JniEnv) -> Option<JavaValue> {
    log("Registered Unsafe natives!");

    None
}

#[allow(non_snake_case)]
fn Java_sun_misc_Unsafe_arrayBaseOffset(_: &JniEnv) -> Option<JavaValue> {
    Some(JavaValue::Long(0))
}

#[allow(non_snake_case)]
fn Java_sun_misc_Unsafe_arrayIndexScale(env: &JniEnv) -> Option<JavaValue> {
    let array_class = &env.parameters[1].as_object().unwrap().unwrap();
    let scale = match env
        .invoke_instance_method(
            InvokeType::Virtual,
            *array_class,
            "java/lang/Class",
            "getComponentType",
            "()Ljava/lang/Class;",
            &[],
        )
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
            None => env.throw_exception(
                "java/lang/InvalidClassCastException",
                Some("expecting array type"),
            ),
        },
        _ => panic!(),
    };

    Some(JavaValue::Int(scale))
}

#[allow(non_snake_case)]
fn Java_sun_misc_Unsafe_addressSize(_: &JniEnv) -> Option<JavaValue> {
    Some(JavaValue::Int(ADDRESS_SIZE))
}

pub fn initialize(cp: &mut Classpath) {
    register_jni!(
        cp,
        Java_sun_misc_Unsafe_registerNatives,
        Java_sun_misc_Unsafe_arrayBaseOffset,
        Java_sun_misc_Unsafe_arrayIndexScale,
        Java_sun_misc_Unsafe_addressSize
    );
}
