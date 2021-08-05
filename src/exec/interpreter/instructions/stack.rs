use crate::{
    exec::interpreter::InstructionEnvironment,
    model::{CallStackFrameState, RuntimeResult},
};

pub fn pop(mut env: InstructionEnvironment) -> RuntimeResult<CallStackFrameState> {
    pop!(&mut env);

    Ok(env.state)
}

pub fn pop2(mut env: InstructionEnvironment) -> RuntimeResult<CallStackFrameState> {
    pop_full!(&mut env);

    Ok(env.state)
}

pub fn dup(mut env: InstructionEnvironment) -> RuntimeResult<CallStackFrameState> {
    let top = env.state.stack.last().expect("stack underflow").clone();
    env.state.stack.push(top);

    Ok(env.state)
}

pub fn dup2(mut env: InstructionEnvironment) -> RuntimeResult<CallStackFrameState> {
    let top = env.state.stack.last_full().expect("stack underflow").clone();
    env.state.stack.push(top.clone());

    Ok(env.state)
}

pub fn dupx1(mut env: InstructionEnvironment) -> RuntimeResult<CallStackFrameState> {
    let top = env.state.stack.last().expect("stack underflow").clone();
    env.state.stack.insert(env.state.stack.len() - 2, top);

    Ok(env.state)
}

pub fn swap(mut env: InstructionEnvironment) -> RuntimeResult<CallStackFrameState> {
    let top = pop!(&mut env);
    let under_top = pop!(&mut env);

    env.state.stack.push(top);
    env.state.stack.push(under_top);

    Ok(env.state)
}
