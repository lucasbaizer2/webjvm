use crate::{
    exec::interpreter::InstructionEnvironment,
    model::{CallStackFrameState, JavaArrayType, JavaValue, RuntimeResult},
    util::get_constant_string,
};

pub fn newarray(mut env: InstructionEnvironment) -> RuntimeResult<CallStackFrameState> {
    let (primitive_type,) = take_values!(&mut env, u8);
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

    let length = pop!(&mut env).as_int().expect("expected integral value");
    let arr = env.jvm.create_empty_array(array_type, length as usize);

    env.state.stack.push(JavaValue::Array(arr));

    Ok(env.state)
}

pub fn anewarray(mut env: InstructionEnvironment) -> RuntimeResult<CallStackFrameState> {
    let (type_ref_id,) = take_values!(&mut env, u16);
    let const_pool = use_const_pool!(&mut env);
    let type_str = get_constant_string(const_pool, type_ref_id);
    let type_id = env.jvm.ensure_class_loaded(type_str, true)?;

    let length = env.state.stack.pop().expect("stack underflow").as_int().expect("expected integral value");
    let arr = env.jvm.create_empty_array(JavaArrayType::Object(type_id), length as usize);

    env.state.stack.push(JavaValue::Array(arr));

    Ok(env.state)
}

pub fn arraylength(mut env: InstructionEnvironment) -> RuntimeResult<CallStackFrameState> {
    let arrayref_id = match pop!(&mut env) {
        JavaValue::Array(id) => id,
        _ => panic!("invalid array instance ID"),
    };
    let heap = env.jvm.heap.borrow();
    let arrayref = heap.array_heap_map.get(&arrayref_id).expect("invalid array instance ID");
    env.state.stack.push(JavaValue::Int(arrayref.values.len() as i32));

    Ok(env.state)
}

pub fn arraystore(mut env: InstructionEnvironment) -> RuntimeResult<CallStackFrameState> {
    let value = pop!(&mut env);
    let index = pop!(&mut env).as_int().expect("invalid array index");
    let arrayref_id = pop!(&mut env).as_array().expect("invalid array instance ID");

    let mut heap = env.jvm.heap.borrow_mut();
    let arrayref = heap.array_heap_map.get_mut(&arrayref_id).expect("invalid array instance ID");
    if index >= arrayref.values.len() as i32 || index < 0 {
        return Err(env
            .jvm
            .throw_exception("java/lang/ArrayIndexOutOfBoundsException", Some(index.to_string().as_str())));
    } else {
        arrayref.values[index as usize] = value;
    }

    Ok(env.state)
}

pub fn arrayload(mut env: InstructionEnvironment) -> RuntimeResult<CallStackFrameState> {
    let index = pop!(&mut env).as_int().expect("invalid array index");
    let arrayref_id = pop!(&mut env).as_array().expect("invalid array instance ID");

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

    Ok(env.state)
}
