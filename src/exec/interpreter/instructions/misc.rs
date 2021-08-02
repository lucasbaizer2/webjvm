use crate::{
    exec::interpreter::InstructionEnvironment,
    model::{CallStackFrameState, JavaValue, RuntimeResult},
    util::get_constant_string,
};

pub fn nop(env: InstructionEnvironment) -> RuntimeResult<CallStackFrameState> {
    Ok(env.state)
}

pub fn new(mut env: InstructionEnvironment) -> RuntimeResult<CallStackFrameState> {
    let (type_ref_id,) = take_values!(&mut env, u16);
    let const_pool = use_const_pool!(&mut env);
    let type_str = get_constant_string(const_pool, type_ref_id);

    let type_loaded_id = env.jvm.ensure_class_loaded(type_str, true)?;
    let instance = env.jvm.new_instance(type_loaded_id)?;
    let instance_id = env.jvm.heap_store_instance(instance);

    env.state.stack.push(JavaValue::Object(Some(instance_id)));

    Ok(env.state)
}

pub fn athrow(mut env: InstructionEnvironment) -> RuntimeResult<CallStackFrameState> {
    let ex = match pop!(&mut env).as_object().expect("expecting object ref") {
        Some(obj) => obj,
        None => return Err(env.jvm.throw_npe()),
    };
    Err(env.jvm.throw_exception_ref(ex))
}
