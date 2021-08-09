use crate::{
    exec::interpreter::InstructionEnvironment,
    model::{JavaValue, MethodDescriptor, RuntimeResult},
    util::get_constant_string,
};

macro_rules! define_if {
    ( $insn:ident, $op:tt ) => {
        pub fn $insn(env: &mut InstructionEnvironment) -> RuntimeResult<()> {
            let (offset,) = take_values!(env, i16);
            let val = pop!(env);
            let int = val.as_int().expect("expecting integral value");
            if int $op 0 {
                branch_to!(env, offset);
            }

            Ok(())
        }
    }
}

macro_rules! define_ificmp {
    ( $insn:ident, $op:tt ) => {
        pub fn $insn(env: &mut InstructionEnvironment) -> RuntimeResult<()> {
            let (offset,) = take_values!(env, i16);
            let rhs = pop!(env);
            let lhs = pop!(env);

            let lhs_int = lhs.as_int().expect("expecting integral value");
            let rhs_int = rhs.as_int().expect("expecting integral value");

            if lhs_int $op rhs_int {
                branch_to!(env, offset);
            }

            Ok(())
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

pub fn checkcast(env: &mut InstructionEnvironment) -> RuntimeResult<()> {
    let (compare_type_id,) = take_values!(env, u16);
    let test = env.state.stack.last().expect("stack underflow");

    if (test.is_object() && test.as_object().unwrap().is_some()) || test.is_array() {
        let const_pool = use_const_pool!(&env);
        let compare_type = get_constant_string(const_pool, compare_type_id);

        if !env.jvm.is_instance_of(test, compare_type, true)? {
            return Err(env.jvm.throw_exception("java/lang/ClassCastException", Some(compare_type)));
        }
    }

    Ok(())
}

pub fn instanceof(env: &mut InstructionEnvironment) -> RuntimeResult<()> {
    let (compare_type_id,) = take_values!(env, u16);
    let const_pool = use_const_pool!(&env);
    let compare_type = get_constant_string(const_pool, compare_type_id);
    let res = env.jvm.is_instance_of(&pop!(env), compare_type, false)?;
    env.state.stack.push(JavaValue::Boolean(res));

    Ok(())
}

pub fn goto(env: &mut InstructionEnvironment) -> RuntimeResult<()> {
    let (offset,) = take_values!(env, i16);
    branch_to!(env, offset);

    Ok(())
}

pub fn ifnonnull(env: &mut InstructionEnvironment) -> RuntimeResult<()> {
    let (offset,) = take_values!(env, i16);
    let val = pop!(env);
    match val {
        JavaValue::Object(ptr) => {
            if ptr.is_some() {
                branch_to!(env, offset);
            }
        }
        JavaValue::Array(_) => branch_to!(env, offset), // internally the way we store arrays they can never be null
        _ => panic!("ifnonnull expecting object"),
    };

    Ok(())
}

pub fn ifnull(env: &mut InstructionEnvironment) -> RuntimeResult<()> {
    let (offset,) = take_values!(env, i16);
    let val = pop!(env);
    match val {
        JavaValue::Object(ptr) => {
            if ptr.is_none() {
                branch_to!(env, offset);
            }
        }
        JavaValue::Array(_) => (), // internally the way we store arrays they can never be null
        _ => panic!("ifnull expecting object"),
    };

    Ok(())
}

pub fn returnvoid(env: &mut InstructionEnvironment) -> RuntimeResult<()> {
    let mut csf = env.jvm.call_stack_frames.borrow_mut();
    csf.pop().unwrap();
    Ok(())
}

pub fn returnvalue(env: &mut InstructionEnvironment) -> RuntimeResult<()> {
    let return_value = pop_full!(env);

    let container_method = {
        let csf = env.jvm.call_stack_frames.borrow();
        let top_frame = csf.last().unwrap();
        top_frame.container_method.clone()
    };
    if let JavaValue::Object(_) = return_value {
        let descriptor = MethodDescriptor::new(&container_method).unwrap();
        if !env.jvm.is_instance_of(&return_value, &descriptor.return_type[1..descriptor.return_type.len() - 1], true)? {
            return Err(env.jvm.throw_exception("java/lang/ClassCastException", None));
        }
    }
    let mut csf = env.jvm.call_stack_frames.borrow_mut();
    csf.pop().unwrap();
    csf.last_mut().expect("stack underflow").state.return_stack_value = Some(return_value);

    Ok(())
}

pub fn ifacmpeq(env: &mut InstructionEnvironment) -> RuntimeResult<()> {
    compare_references(env, true)
}

pub fn ifacmpne(env: &mut InstructionEnvironment) -> RuntimeResult<()> {
    compare_references(env, false)
}

#[inline]
pub fn compare_references(env: &mut InstructionEnvironment, jump_if_equal: bool) -> RuntimeResult<()> {
    let (offset,) = take_values!(env, u16);
    let rhs = pop!(env);
    let lhs = pop!(env);
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
    if (jump_if_equal && equal) || (!jump_if_equal && !equal) {
        branch_to!(env, offset);
    }

    Ok(())
}

pub fn lookupswitch(env: &mut InstructionEnvironment) -> RuntimeResult<()> {
    let key = pop!(env).as_int().expect("expecting integral value");

    env.state.instruction_offset += (4 - env.state.instruction_offset % 4) % 4;
    let (default, npairs) = take_values!(env, u32, u32);
    for _ in 0..npairs {
        let (value, offset) = take_values!(env, i32, i32);
        if key == value {
            branch_to!(env, offset);
            return Ok(());
        }
    }

    branch_to!(env, default);

    Ok(())
}
