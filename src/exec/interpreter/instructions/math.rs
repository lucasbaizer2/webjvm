use crate::{
    exec::interpreter::InstructionEnvironment,
    model::{JavaValue, RuntimeResult},
};
use std::{cmp::Ordering, num::Wrapping};

macro_rules! define_imath {
    ( $insn:ident, $op:tt ) => {
        pub fn $insn(env: &mut InstructionEnvironment) -> RuntimeResult<()> {
            let rhs = Wrapping(pop!(env).as_int().expect("expecting integral value"));
            let lhs = Wrapping(pop!(env).as_int().expect("expecting integral value"));
            env.state.stack.push(JavaValue::Int((lhs $op rhs).0));

            Ok(())
        }
    }
}

macro_rules! define_ishift {
    ( $insn:ident, $op:tt, $int_type:ty ) => {
        pub fn $insn(env: &mut InstructionEnvironment) -> RuntimeResult<()> {
            let rhs = pop!(env).as_int().expect("expecting integral value") & 0b11111;
            let lhs = pop!(env).as_int().expect("expecting integral value");
            env.state.stack.push(JavaValue::Int(((lhs as $int_type) $op (rhs as $int_type)) as i32));

            Ok(())
        }
    }
}

macro_rules! define_lshift {
    ( $insn:ident, $op:tt, $int_type:ty ) => {
        pub fn $insn(env: &mut InstructionEnvironment) -> RuntimeResult<()> {
            let rhs = pop!(env).as_int().expect("expecting integral value") & 0b111111;
            let lhs = pop_full!(env).as_long().expect("expecting long value");
            env.state.stack.push(JavaValue::Long(((lhs as $int_type) $op (rhs as $int_type)) as i64));

            Ok(())
        }
    }
}

macro_rules! define_lmath {
    ( $insn:ident, $op:tt ) => {
        pub fn $insn(env: &mut InstructionEnvironment) -> RuntimeResult<()> {
            let rhs = Wrapping(pop_full!(env).as_long().expect("expecting long value"));
            let lhs = Wrapping(pop_full!(env).as_long().expect("expecting long value"));
            env.state.stack.push(JavaValue::Long((lhs $op rhs).0));

            Ok(())
        }
    }
}

macro_rules! define_fmath {
    ( $insn:ident, $op:tt ) => {
        pub fn $insn(env: &mut InstructionEnvironment) -> RuntimeResult<()> {
            let rhs = pop!(env).as_float().expect("expecting float value");
            let lhs = pop!(env).as_float().expect("expecting float value");
            env.state.stack.push(JavaValue::Float(lhs $op rhs));

            Ok(())
        }
    }
}

macro_rules! define_dmath {
    ( $insn:ident, $op:tt ) => {
        pub fn $insn(env: &mut InstructionEnvironment) -> RuntimeResult<()> {
            let rhs = pop_full!(env).as_double().expect("expecting double value");
            let lhs = pop_full!(env).as_double().expect("expecting double value");
            env.state.stack.push(JavaValue::Double(lhs $op rhs));

            Ok(())
        }
    }
}

macro_rules! define_cast {
    ( $insn:ident, $from:ident, $jt:ident, $cast:ty ) => {
        pub fn $insn(env: &mut InstructionEnvironment) -> RuntimeResult<()> {
            use paste::paste;

            paste! {
                let int = match pop_full!(env).[<as_ $from>]() {
                    Ok(e) => e,
                    Err(_) => panic!(),
                };
                env.state.stack.push(JavaValue::$jt(int as $cast));

                Ok(())
            }
        }
    };
}

define_imath!(iadd, +);
define_imath!(iand, &);
define_imath!(idiv, /);
define_imath!(imul, *);
define_imath!(ior, |);
define_imath!(irem, %);
define_imath!(isub, -);
define_imath!(ixor, ^);

define_lmath!(ladd, +);
define_lmath!(land, &);
define_lmath!(ldiv, /);
define_lmath!(lmul, *);
define_lmath!(lor, |);
define_lmath!(lrem, %);
define_lmath!(lsub, -);
define_lmath!(lxor, ^);

define_fmath!(fadd, +);
define_fmath!(fsub, -);
define_fmath!(fmul, *);
define_fmath!(fdiv, /);
define_fmath!(frem, %);

define_dmath!(dadd, +);
define_dmath!(dsub, -);
define_dmath!(dmul, *);
define_dmath!(ddiv, /);
define_dmath!(drem, %);

define_ishift!(ishl, <<, i32);
define_ishift!(ishr, >>, i32);
define_ishift!(iushr, >>, u32);

define_lshift!(lshl, <<, i64);
define_lshift!(lshr, >>, i64);
define_lshift!(lushr, >>, u64);

define_cast!(i2b, int, Byte, i8);
define_cast!(i2c, int, Char, u16);
define_cast!(i2d, int, Double, f64);
define_cast!(i2f, int, Float, f32);
define_cast!(i2l, int, Long, i64);
define_cast!(i2s, int, Short, i16);

define_cast!(f2i, float, Int, i32);
define_cast!(f2l, float, Long, i64);
define_cast!(f2d, float, Double, f64);

define_cast!(d2i, double, Int, i32);
define_cast!(d2l, double, Long, i64);
define_cast!(d2f, double, Float, f32);

define_cast!(l2i, long, Int, i32);
define_cast!(l2f, long, Float, f32);
define_cast!(l2d, long, Double, f64);

pub fn iinc(env: &mut InstructionEnvironment) -> RuntimeResult<()> {
    let (index, value) = take_values!(env, u8, i8);
    let current_val = env.state.lvt[index as usize].as_int().expect("expecting integral value");
    env.state.lvt[index as usize] = JavaValue::Int(current_val + value as i32);

    Ok(())
}

pub fn iincwide(env: &mut InstructionEnvironment) -> RuntimeResult<()> {
    let (index, value) = take_values!(env, u16, i16);
    let current_val = env.state.lvt[index as usize].as_int().expect("expecting integral value");
    env.state.lvt[index as usize] = JavaValue::Int(current_val + value as i32);

    Ok(())
}

pub fn dcmpg(env: &mut InstructionEnvironment) -> RuntimeResult<()> {
    compare_doubles(env, true)
}

pub fn dcmpl(env: &mut InstructionEnvironment) -> RuntimeResult<()> {
    compare_doubles(env, false)
}

#[allow(clippy::float_cmp)]
pub fn compare_doubles(env: &mut InstructionEnvironment, greater: bool) -> RuntimeResult<()> {
    let rhs = pop_full!(env).as_double().expect("expecting double");
    let lhs = pop_full!(env).as_double().expect("expecting double");
    if lhs.is_nan() || rhs.is_nan() {
        let nan_value = match greater {
            true => 1,
            false => -1,
        };
        env.state.stack.push(JavaValue::Int(nan_value));
    } else if lhs > rhs {
        env.state.stack.push(JavaValue::Int(1));
    } else if lhs == rhs {
        env.state.stack.push(JavaValue::Int(0));
    } else {
        env.state.stack.push(JavaValue::Int(-1));
    }

    Ok(())
}

pub fn fcmpg(env: &mut InstructionEnvironment) -> RuntimeResult<()> {
    compare_floats(env, true)
}

pub fn fcmpl(env: &mut InstructionEnvironment) -> RuntimeResult<()> {
    compare_floats(env, false)
}

#[allow(clippy::float_cmp)]
pub fn compare_floats(env: &mut InstructionEnvironment, greater: bool) -> RuntimeResult<()> {
    let rhs = pop!(env).as_float().expect("expecting float");
    let lhs = pop!(env).as_float().expect("expecting float");
    if lhs.is_nan() || rhs.is_nan() {
        let nan_value = match greater {
            true => 1,
            false => -1,
        };
        env.state.stack.push(JavaValue::Int(nan_value));
    } else if lhs > rhs {
        env.state.stack.push(JavaValue::Int(1));
    } else if lhs == rhs {
        env.state.stack.push(JavaValue::Int(0));
    } else {
        env.state.stack.push(JavaValue::Int(-1));
    }

    Ok(())
}

pub fn lcmp(env: &mut InstructionEnvironment) -> RuntimeResult<()> {
    let rhs = pop_full!(env).as_long().unwrap();
    let lhs = pop_full!(env).as_long().unwrap();
    let val = match lhs.cmp(&rhs) {
        Ordering::Greater => 1,
        Ordering::Less => -1,
        Ordering::Equal => 0,
    };
    env.state.stack.push(JavaValue::Int(val));

    Ok(())
}
