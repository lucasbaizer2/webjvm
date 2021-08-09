use crate::{exec::interpreter::InstructionEnvironment, model::RuntimeResult};

pub fn pop(env: &mut InstructionEnvironment) -> RuntimeResult<()> {
    pop!(env);

    Ok(())
}

pub fn pop2(env: &mut InstructionEnvironment) -> RuntimeResult<()> {
    pop_full!(env);

    Ok(())
}

pub fn dup(env: &mut InstructionEnvironment) -> RuntimeResult<()> {
    let top = env.state.stack.last().expect("stack underflow").clone();
    env.state.stack.push(top);

    Ok(())
}

pub fn dup2(env: &mut InstructionEnvironment) -> RuntimeResult<()> {
    let top = env.state.stack.last_full().expect("stack underflow").clone();
    if !top.is_wide() {
        let under_top = env.state.stack[env.state.stack.len() - 2].clone();
        env.state.stack.push(under_top);
    }
    env.state.stack.push(top);

    Ok(())
}

pub fn dupx1(env: &mut InstructionEnvironment) -> RuntimeResult<()> {
    let top = env.state.stack.last().expect("stack underflow").clone();
    env.state.stack.insert(env.state.stack.len() - 2, top);

    Ok(())
}

pub fn swap(env: &mut InstructionEnvironment) -> RuntimeResult<()> {
    let top = pop!(env);
    let under_top = pop!(env);

    env.state.stack.push(top);
    env.state.stack.push(under_top);

    Ok(())
}
