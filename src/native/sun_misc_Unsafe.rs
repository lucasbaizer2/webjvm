use std::mem::size_of;

use crate::{
    model::{InternalMetadata, JavaValue, RuntimeResult},
    Classpath, InvokeType, JniEnv,
};

const ADDRESS_SIZE: i32 = size_of::<usize>() as i32;

#[allow(non_snake_case)]
fn Java_sun_misc_Unsafe_registerNatives(_: &JniEnv) -> RuntimeResult<Option<JavaValue>> {
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
            env.get_class_id("java/lang/Class")?,
            "getComponentType",
            "()Ljava/lang/Class;",
            &[],
        )?
        .unwrap()
    {
        JavaValue::Object(obj) => match obj {
            Some(component_type) => {
                match env.get_internal_metadata(component_type, "class_name").unwrap().into_string().as_str() {
                    "byte" => 1,
                    "short" => 2,
                    "int" => 4,
                    "long" => 8,
                    "float" => 4,
                    "double" => 8,
                    "char" => 2,
                    "boolean" => 1,
                    _ => ADDRESS_SIZE,
                }
            }
            None => {
                return Err(env.throw_exception("java/lang/InvalidClassCastException", Some("expecting array type")))
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
    let field = env.parameters[1].as_object().unwrap().unwrap();
    let name = env.get_field(field, "name").as_object().unwrap().unwrap();
    Ok(Some(JavaValue::Long(name as i64)))
}

fn get_value_at_offset(env: &JniEnv, obj: usize, offset: i64) -> JavaValue {
    let field_name = env.get_string(offset as usize);

    let heap = env.jvm.heap.borrow();
    let internal_obj = heap.object_heap_map.get(&obj).unwrap();
    internal_obj.instance_fields.get(&field_name).unwrap().clone()
}

fn set_value_at_offset(env: &JniEnv, obj: usize, offset: i64, value: JavaValue) {
    let field_name = env.get_string(offset as usize);

    let mut heap = env.jvm.heap.borrow_mut();
    let internal_obj = heap.object_heap_map.get_mut(&obj).unwrap();
    internal_obj.instance_fields.insert(field_name, value);
}

fn cas(env: &JniEnv) -> RuntimeResult<Option<JavaValue>> {
    let obj = env.parameters[1].as_object().unwrap().unwrap();
    let offset = env.parameters[2].as_long().unwrap();
    let expect = env.parameters[4].clone();
    let update = env.parameters[5].clone();

    let current_field = get_value_at_offset(env, obj, offset);
    if current_field == expect {
        set_value_at_offset(env, obj, offset, update);
        Ok(Some(JavaValue::Boolean(true)))
    } else {
        Ok(Some(JavaValue::Boolean(false)))
    }
}

#[allow(non_snake_case)]
fn Java_sun_misc_Unsafe_compareAndSwapObject(env: &JniEnv) -> RuntimeResult<Option<JavaValue>> {
    cas(env)
}

#[allow(non_snake_case)]
fn Java_sun_misc_Unsafe_compareAndSwapInt(env: &JniEnv) -> RuntimeResult<Option<JavaValue>> {
    cas(env)
}

#[allow(non_snake_case)]
fn Java_sun_misc_Unsafe_getIntVolatile(env: &JniEnv) -> RuntimeResult<Option<JavaValue>> {
    let obj = env.parameters[1].as_object().unwrap().unwrap();
    let offset = env.parameters[2].as_long().unwrap();

    let current_field = get_value_at_offset(env, obj, offset);
    Ok(Some(current_field))
}

#[allow(non_snake_case)]
fn Java_sun_misc_Unsafe_allocateMemory(env: &JniEnv) -> RuntimeResult<Option<JavaValue>> {
    let instance = env.get_current_instance();
    let block_size = env.parameters[1].as_long().unwrap();
    let mut buf: Vec<u8> = Vec::with_capacity(block_size as usize);
    let ptr = buf.as_mut_ptr();

    std::mem::forget(buf);

    env.set_internal_metadata(instance, &(ptr as i64).to_string(), InternalMetadata::Numeric(block_size as usize));

    Ok(Some(JavaValue::Long(ptr as i64)))
}

#[allow(non_snake_case)]
fn Java_sun_misc_Unsafe_freeMemory(env: &JniEnv) -> RuntimeResult<Option<JavaValue>> {
    let instance = env.get_current_instance();
    let address = env.parameters[1].as_long().unwrap() as *mut u8;
    let block_size = env.remove_internal_metadata(instance, &(address as i64).to_string()).unwrap().into_usize();

    unsafe {
        let data = Vec::from_raw_parts(address, block_size, block_size);
        std::mem::drop(data);
    }

    Ok(None)
}

#[allow(non_snake_case)]
fn Java_sun_misc_Unsafe_getByte(env: &JniEnv) -> RuntimeResult<Option<JavaValue>> {
    let address = env.parameters[1].as_long().unwrap() as *mut i8;
    let value = unsafe { std::ptr::read(address) };

    Ok(Some(JavaValue::Byte(value)))
}

#[allow(non_snake_case)]
fn Java_sun_misc_Unsafe_putLong(env: &JniEnv) -> RuntimeResult<Option<JavaValue>> {
    let address = env.parameters[1].as_long().unwrap() as *mut i64;
    let value = env.parameters[3].as_long().unwrap();

    unsafe { std::ptr::write(address, value) };

    Ok(None)
}

pub fn initialize(cp: &mut Classpath) {
    register_jni!(
        cp,
        Java_sun_misc_Unsafe_registerNatives,
        Java_sun_misc_Unsafe_arrayBaseOffset,
        Java_sun_misc_Unsafe_arrayIndexScale,
        Java_sun_misc_Unsafe_addressSize,
        Java_sun_misc_Unsafe_objectFieldOffset,
        Java_sun_misc_Unsafe_compareAndSwapObject,
        Java_sun_misc_Unsafe_compareAndSwapInt,
        Java_sun_misc_Unsafe_getIntVolatile,
        Java_sun_misc_Unsafe_allocateMemory,
        Java_sun_misc_Unsafe_freeMemory,
        Java_sun_misc_Unsafe_putLong,
        Java_sun_misc_Unsafe_getByte
    );
}
