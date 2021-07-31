use crate::{
    exec::interpreter::InstructionEnvironment,
    model::{CallStackFrameState, RuntimeResult},
};

pub fn pop(mut env: InstructionEnvironment) -> RuntimeResult<CallStackFrameState> {
    pop!(&mut env);

    Ok(env.state)
}

pub fn pop2(mut env: InstructionEnvironment) -> RuntimeResult<CallStackFrameState> {
    pop!(&mut env);
    env.state.stack.pop();

    Ok(env.state)
}

pub fn dup(mut env: InstructionEnvironment) -> RuntimeResult<CallStackFrameState> {
    let top = env.state.stack.last().expect("stack underflow").clone();
    env.state.stack.push(top);

    Ok(env.state)
}

pub fn dup2(mut env: InstructionEnvironment) -> RuntimeResult<CallStackFrameState> {
    let top = env.state.stack.last().expect("stack underflow").clone();
    if env.state.stack.len() > 1 {
        let under_top = env.state.stack[env.state.stack.len() - 2].clone();
        env.state.stack.push(under_top);
    }
    env.state.stack.push(top);

    Ok(env.state)
}

pub fn dupx1(mut env: InstructionEnvironment) -> RuntimeResult<CallStackFrameState> {
    let top = env.state.stack.last().expect("stack underflow").clone();
    env.state.stack.insert(env.state.stack.len() - 2, top);

    Ok(env.state)
}
