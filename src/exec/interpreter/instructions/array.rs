use crate::{
    exec::interpreter::InstructionEnvironment,
    model::{JavaArrayType, JavaValue, RuntimeResult},
    util::get_constant_string,
};

pub fn newarray(env: &mut InstructionEnvironment) -> RuntimeResult<()> {
    let (primitive_type,) = take_values!(env, u8);
    let array_type = match primitive_type {
        4 => JavaArrayType::Boolean,
        5 => JavaArrayType::Char,
        6 => JavaArrayType::Float,
        7 => JavaArrayType::Double,
        8 => JavaArrayType::Byte,
        9 => JavaArrayType::Short,
        10 => JavaArrayType::Int,
        11 => JavaArrayType::Long,
        _ => panic!("invalid array type code"),
    };

    let length = pop!(env).as_int().expect("expected integral value");
    let arr = env.jvm.create_empty_array(array_type, length as usize);

    env.state.stack.push(JavaValue::Array(arr));

    Ok(())
}

pub fn anewarray(env: &mut InstructionEnvironment) -> RuntimeResult<()> {
    let (type_ref_id,) = take_values!(env, u16);
    let const_pool = use_const_pool!(env);
    let type_str = get_constant_string(const_pool, type_ref_id);
    let type_id = env.jvm.ensure_class_loaded(type_str, true)?;

    let length = env.state.stack.pop().expect("stack underflow").as_int().expect("expected integral value");
    let arr = env.jvm.create_empty_array(JavaArrayType::Object(type_id), length as usize);

    env.state.stack.push(JavaValue::Array(arr));

    Ok(())
}

pub fn arraylength(env: &mut InstructionEnvironment) -> RuntimeResult<()> {
    let arrayref_id = match pop!(env) {
        JavaValue::Array(id) => id,
        JavaValue::Object(_) => return Err(env.jvm.throw_npe()),
        _ => return Err(env.jvm.throw_exception("java/lang/Error", Some("invalid array instance ID"))),
    };
    let heap = env.jvm.heap.borrow();
    let arrayref = heap.array_heap_map.get(&arrayref_id).expect("arraylength: invalid array instance ID");
    env.state.stack.push(JavaValue::Int(arrayref.values.len() as i32));

    Ok(())
}

pub fn arraystore(env: &mut InstructionEnvironment) -> RuntimeResult<()> {
    let value = pop_full!(env);
    let index = pop!(env).as_int().expect("invalid array index");
    let arrayref_id = match pop!(env) {
        JavaValue::Array(id) => id,
        JavaValue::Object(None) => return Err(env.jvm.throw_npe()),
        _ => return Err(env.jvm.throw_exception("java/lang/Error", Some("arraystore: invalid array instance ID"))),
    };

    let mut heap = env.jvm.heap.borrow_mut();
    let arrayref = heap.array_heap_map.get_mut(&arrayref_id).expect("invalid array instance ID");
    if index >= arrayref.values.len() as i32 || index < 0 {
        return Err(env
            .jvm
            .throw_exception("java/lang/ArrayIndexOutOfBoundsException", Some(index.to_string().as_str())));
    } else {
        arrayref.values[index as usize] = value;
    }

    Ok(())
}

pub fn arrayload(env: &mut InstructionEnvironment) -> RuntimeResult<()> {
    let index = pop!(env).as_int().expect("invalid array index");
    let arrayref_id = match pop!(env) {
        JavaValue::Array(id) => id,
        JavaValue::Object(None) => return Err(env.jvm.throw_npe()),
        _ => return Err(env.jvm.throw_exception("java/lang/Error", Some("arrayload: invalid array instance ID"))),
    };

    let heap = env.jvm.heap.borrow();
    let arrayref = heap.array_heap_map.get(&arrayref_id).expect("invalid array instance ID");

    if index >= arrayref.values.len() as i32 || index < 0 {
        return Err(env
            .jvm
            .throw_exception("java/lang/ArrayIndexOutOfBoundsException", Some(index.to_string().as_str())));
    } else {
        let val = arrayref.values[index as usize].clone();
        env.state.stack.push(val);
    }

    Ok(())
}
