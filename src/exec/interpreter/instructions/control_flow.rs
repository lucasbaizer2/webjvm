use crate::{
    exec::interpreter::InstructionEnvironment,
    model::{CallStackFrameState, JavaValue, MethodDescriptor, RuntimeResult},
    util::get_constant_string,
};

macro_rules! define_if {
    ( $insn:ident, $op:tt ) => {
        pub fn $insn(mut env: InstructionEnvironment) -> RuntimeResult<CallStackFrameState> {
            let (offset,) = take_values!(&mut env, i16);
            let val = pop!(&mut env);
            let int = val.as_int().expect("expecting integral value");
            if int $op 0 {
                branch_to!(&mut env, offset);
            }

            Ok(env.state)
        }
    }
}

macro_rules! define_ificmp {
    ( $insn:ident, $op:tt ) => {
        pub fn $insn(mut env: InstructionEnvironment) -> RuntimeResult<CallStackFrameState> {
            let (offset,) = take_values!(&mut env, i16);
            let rhs = pop!(&mut env);
            let lhs = pop!(&mut env);

            let lhs_int = lhs.as_int().expect("expecting integral value");
            let rhs_int = rhs.as_int().expect("expecting integral value");

            if lhs_int $op rhs_int {
                branch_to!(&mut env, offset);
            }

            Ok(env.state)
        }
    }
}

define_ificmp!(ificmpeq, ==);
define_ificmp!(ificmpge, >=);
define_ificmp!(ificmpgt, >);
define_ificmp!(ificmple, <=);
define_ificmp!(ificmplt, <);
define_ificmp!(ificmpne, !=);

define_if!(ifeq, ==);
define_if!(ifge, >=);
define_if!(ifgt, >);
define_if!(ifle, <=);
define_if!(iflt, <);
define_if!(ifne, !=);

pub fn checkcast(mut env: InstructionEnvironment) -> RuntimeResult<CallStackFrameState> {
    let (compare_type_id,) = take_values!(&mut env, u16);
    let test = env.state.stack.last().expect("stack underflow");

    if (test.is_object() && test.as_object().unwrap().is_some()) || test.is_array() {
        let const_pool = use_const_pool!(&env);
        let compare_type = get_constant_string(&const_pool, compare_type_id);

        if !env.jvm.is_instance_of(&test, compare_type)? {
            return Err(env.jvm.throw_exception("java/lang/ClassCastException", Some(compare_type)));
        }
    }

    Ok(env.state)
}

pub fn instanceof(mut env: InstructionEnvironment) -> RuntimeResult<CallStackFrameState> {
    let (compare_type_id,) = take_values!(&mut env, u16);
    let const_pool = use_const_pool!(&env);
    let compare_type = get_constant_string(&const_pool, compare_type_id);
    let res = env.jvm.is_instance_of(&pop!(&mut env), compare_type)?;
    env.state.stack.push(JavaValue::Boolean(res));

    Ok(env.state)
}

pub fn goto(mut env: InstructionEnvironment) -> RuntimeResult<CallStackFrameState> {
    let (offset,) = take_values!(&mut env, i16);
    branch_to!(&mut env, offset);

    Ok(env.state)
}

pub fn ifnonnull(mut env: InstructionEnvironment) -> RuntimeResult<CallStackFrameState> {
    let (offset,) = take_values!(&mut env, i16);
    let val = pop!(&mut env);
    match val {
        JavaValue::Object(ptr) => match ptr {
            Some(_) => branch_to!(&mut env, offset),
            None => (),
        },
        JavaValue::Array(_) => branch_to!(&mut env, offset), // internally the way we store arrays they can never be null
        _ => panic!("ifnonnull expecting object"),
    };

    Ok(env.state)
}

pub fn ifnull(mut env: InstructionEnvironment) -> RuntimeResult<CallStackFrameState> {
    let (offset,) = take_values!(&mut env, i16);
    let val = pop!(&mut env);
    match val {
        JavaValue::Object(ptr) => match ptr {
            None => branch_to!(&mut env, offset),
            Some(_) => (),
        },
        JavaValue::Array(_) => (), // internally the way we store arrays they can never be null
        _ => panic!("ifnull expecting object"),
    };

    Ok(env.state)
}

pub fn returnvoid(env: InstructionEnvironment) -> RuntimeResult<CallStackFrameState> {
    let mut csf = env.jvm.call_stack_frames.borrow_mut();
    csf.pop().unwrap();
    Ok(env.state)
}

pub fn returnvalue(mut env: InstructionEnvironment) -> RuntimeResult<CallStackFrameState> {
    let return_value = pop_full!(&mut env);

    let container_method = {
        let csf = env.jvm.call_stack_frames.borrow();
        let top_frame = csf.last().unwrap();
        top_frame.container_method.clone()
    };
    match return_value {
        JavaValue::Object(_) => {
            let descriptor = MethodDescriptor::new(&container_method).unwrap();
            if !env.jvm.is_instance_of(&return_value, &descriptor.return_type[1..descriptor.return_type.len() - 1])? {
                return Err(env.jvm.throw_exception("java/lang/ClassCastException", None));
            }
        }
        _ => (),
    }

    let mut csf = env.jvm.call_stack_frames.borrow_mut();
    csf.pop().unwrap();
    csf.last_mut().expect("stack underflow").state.return_stack_value = Some(return_value);

    Ok(env.state)
}

pub fn ifacmpeq(env: InstructionEnvironment) -> RuntimeResult<CallStackFrameState> {
    compare_references(env, true)
}

pub fn ifacmpne(env: InstructionEnvironment) -> RuntimeResult<CallStackFrameState> {
    compare_references(env, false)
}

#[inline]
pub fn compare_references(mut env: InstructionEnvironment, jump_if_equal: bool) -> RuntimeResult<CallStackFrameState> {
    let (offset,) = take_values!(&mut env, u16);
    let rhs = pop!(&mut env);
    let lhs = pop!(&mut env);
    let equal = match lhs {
        JavaValue::Object(obj1) => match rhs {
            JavaValue::Object(obj2) => match (obj1, obj2) {
                (Some(inner1), Some(inner2)) => inner1 == inner2,
                (None, None) => true,
                _ => false,
            },
            _ => false,
        },
        JavaValue::Array(obj1) => match rhs {
            JavaValue::Array(obj2) => obj1 == obj2,
            _ => false,
        },
        _ => false,
    };
    if jump_if_equal && equal {
        branch_to!(&mut env, offset);
    } else if !jump_if_equal && !equal {
        branch_to!(&mut env, offset);
    }

    Ok(env.state)
}

pub fn lookupswitch(mut env: InstructionEnvironment) -> RuntimeResult<CallStackFrameState> {
    let key = pop!(&mut env).as_int().expect("expecting integral value");

    env.state.instruction_offset += (4 - env.state.instruction_offset % 4) % 4;
    let (default, npairs) = take_values!(&mut env, u32, u32);
    for _ in 0..npairs {
        let (value, offset) = take_values!(&mut env, i32, i32);
        if key == value {
            branch_to!(&mut env, offset);
            return Ok(env.state);
        }
    }

    branch_to!(&mut env, default);

    Ok(env.state)
}
