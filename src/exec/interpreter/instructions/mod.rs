use crate::model::CallStackFrame;

use super::{InstructionEnvironment, InstructionHandler};

#[inline]
fn take_u8(env: &mut InstructionEnvironment, top_frame: &mut CallStackFrame) -> u8 {
    let value = top_frame.instructions[env.state.instruction_offset];
    env.state.instruction_offset += 1;
    value
}

#[inline]
fn take_i8(env: &mut InstructionEnvironment, top_frame: &mut CallStackFrame) -> i8 {
    take_u8(env, top_frame) as i8
}

#[inline]
fn take_u16(env: &mut InstructionEnvironment, top_frame: &mut CallStackFrame) -> u16 {
    let value1 = top_frame.instructions[env.state.instruction_offset] as u16;
    let value2 = top_frame.instructions[env.state.instruction_offset + 1] as u16;
    env.state.instruction_offset += 2;
    (value1 << 8) | value2
}

#[inline]
fn take_i16(env: &mut InstructionEnvironment, top_frame: &mut CallStackFrame) -> i16 {
    take_u16(env, top_frame) as i16
}

#[inline]
fn take_u32(env: &mut InstructionEnvironment, top_frame: &mut CallStackFrame) -> u32 {
    let value1 = top_frame.instructions[env.state.instruction_offset] as u32;
    let value2 = top_frame.instructions[env.state.instruction_offset + 1] as u32;
    let value3 = top_frame.instructions[env.state.instruction_offset + 2] as u32;
    let value4 = top_frame.instructions[env.state.instruction_offset + 3] as u32;
    env.state.instruction_offset += 4;
    (value1 << 24) | (value2 << 16) | (value3 << 8) | value4
}

#[inline]
fn take_i32(env: &mut InstructionEnvironment, top_frame: &mut CallStackFrame) -> i32 {
    take_u32(env, top_frame) as i32
}

#[macro_export]
macro_rules! take_values {
    ( $env:expr, $( $value_type:ident ),* ) => {{
        use paste::paste;

        let mut csf = $env.jvm.call_stack_frames.borrow_mut();
        let top_frame = csf.last_mut().unwrap();

        paste! {
            (
                $(
                    crate::exec::interpreter::instructions::[<take_ $value_type>]($env, top_frame),
                )*
            )
        }
    }};
}

#[macro_export]
macro_rules! pop {
    ( $env:expr ) => {{
        match $env.state.stack.pop() {
            Some(val) => val,
            None => return Err($env.jvm.throw_exception("java/lang/Error", Some("stack underflow"))),
        }
    }};
}

#[macro_export]
macro_rules! pop_full {
    ( $env:expr ) => {{
        match $env.state.stack.pop_full() {
            Some(val) => val,
            None => return Err($env.jvm.throw_exception("java/lang/Error", Some("stack underflow"))),
        }
    }};
}

#[macro_export]
macro_rules! use_const_pool {
    ( $env:expr ) => {{
        let csf = $env.jvm.call_stack_frames.borrow();
        let top_frame = csf.last().unwrap();
        let container_class = &top_frame.container_class;
        &$env.jvm.classpath.get_classpath_entry(container_class).unwrap().const_pool
    }};
}

#[macro_export]
macro_rules! update_stack {
    ( $env:expr ) => {{
        let mut csf = $env.jvm.call_stack_frames.borrow_mut();
        let frame = csf.last_mut().expect("no stack frame present");
        if let Some(value) = &frame.state.return_stack_value {
            $env.state.stack.push(value.clone());
            frame.state.return_stack_value = None;
        }
    }};
}

#[macro_export]
macro_rules! branch_to {
    ( $env:expr, $offset:expr ) => {{
        $env.state.instruction_offset = ($env.instruction_address as isize + $offset as isize) as usize;
    }};
}

mod array;
mod constant;
mod control_flow;
mod field;
mod invoke;
mod load;
mod math;
mod misc;
mod stack;
mod store;
mod wide;

pub fn initialize(handlers: &mut Vec<InstructionHandler>) {
    handlers[0x00] = misc::nop;

    handlers[0x01] = constant::aconstnull;

    handlers[0x02] = constant::iconstm1;
    handlers[0x03] = constant::iconst0;
    handlers[0x04] = constant::iconst1;
    handlers[0x05] = constant::iconst2;
    handlers[0x06] = constant::iconst3;
    handlers[0x07] = constant::iconst4;
    handlers[0x08] = constant::iconst5;

    handlers[0x09] = constant::lconst0;
    handlers[0x0a] = constant::lconst1;

    handlers[0x0b] = constant::fconst0;
    handlers[0x0c] = constant::fconst1;
    handlers[0x0d] = constant::fconst2;

    handlers[0x0e] = constant::dconst0;
    handlers[0x0f] = constant::dconst1;

    handlers[0x10] = constant::bipush;
    handlers[0x11] = constant::sipush;
    handlers[0x12] = constant::ldc;
    handlers[0x13] = constant::ldcw;
    handlers[0x14] = constant::ldcw;

    handlers[0x15] = load::iload;
    handlers[0x16] = load::lload;
    handlers[0x17] = load::fload;
    handlers[0x18] = load::dload;
    handlers[0x19] = load::aload;

    handlers[0x1a] = load::iload0;
    handlers[0x1b] = load::iload1;
    handlers[0x1c] = load::iload2;
    handlers[0x1d] = load::iload3;

    handlers[0x1e] = load::lload0;
    handlers[0x1f] = load::lload1;
    handlers[0x20] = load::lload2;
    handlers[0x21] = load::lload3;

    handlers[0x22] = load::fload0;
    handlers[0x23] = load::fload1;
    handlers[0x24] = load::fload2;
    handlers[0x25] = load::fload3;

    handlers[0x26] = load::dload0;
    handlers[0x27] = load::dload1;
    handlers[0x28] = load::dload2;
    handlers[0x29] = load::dload3;

    handlers[0x2a] = load::aload0;
    handlers[0x2b] = load::aload1;
    handlers[0x2c] = load::aload2;
    handlers[0x2d] = load::aload3;

    handlers[0x2e] = array::arrayload;
    handlers[0x2f] = array::arrayload;
    handlers[0x30] = array::arrayload;
    handlers[0x31] = array::arrayload;
    handlers[0x32] = array::arrayload;
    handlers[0x33] = array::arrayload;
    handlers[0x34] = array::arrayload;
    handlers[0x35] = array::arrayload;

    handlers[0x36] = store::istore;
    handlers[0x37] = store::lstore;
    handlers[0x38] = store::fstore;
    handlers[0x39] = store::dstore;
    handlers[0x3a] = store::astore;

    handlers[0x3b] = store::istore0;
    handlers[0x3c] = store::istore1;
    handlers[0x3d] = store::istore2;
    handlers[0x3e] = store::istore3;

    handlers[0x3f] = store::lstore0;
    handlers[0x40] = store::lstore1;
    handlers[0x41] = store::lstore2;
    handlers[0x42] = store::lstore3;

    handlers[0x43] = store::fstore0;
    handlers[0x44] = store::fstore1;
    handlers[0x45] = store::fstore2;
    handlers[0x46] = store::fstore3;

    handlers[0x47] = store::dstore0;
    handlers[0x48] = store::dstore1;
    handlers[0x49] = store::dstore2;
    handlers[0x4a] = store::dstore3;

    handlers[0x4b] = store::astore0;
    handlers[0x4c] = store::astore1;
    handlers[0x4d] = store::astore2;
    handlers[0x4e] = store::astore3;

    handlers[0x4f] = array::arraystore;
    handlers[0x50] = array::arraystore;
    handlers[0x51] = array::arraystore;
    handlers[0x52] = array::arraystore;
    handlers[0x53] = array::arraystore;
    handlers[0x54] = array::arraystore;
    handlers[0x55] = array::arraystore;
    handlers[0x56] = array::arraystore;

    handlers[0x57] = stack::pop;
    handlers[0x58] = stack::pop2;
    handlers[0x59] = stack::dup;
    handlers[0x5a] = stack::dupx1;
    handlers[0x5c] = stack::dup2;
    handlers[0x5f] = stack::swap;

    handlers[0x60] = math::iadd;
    handlers[0x61] = math::ladd;
    handlers[0x62] = math::fadd;
    handlers[0x63] = math::dadd;
    handlers[0x64] = math::isub;
    handlers[0x65] = math::lsub;
    handlers[0x66] = math::fsub;
    handlers[0x67] = math::dsub;
    handlers[0x68] = math::imul;
    handlers[0x69] = math::lmul;
    handlers[0x6a] = math::fmul;
    handlers[0x6b] = math::dmul;
    handlers[0x6c] = math::idiv;
    handlers[0x6d] = math::ldiv;
    handlers[0x6e] = math::fdiv;
    handlers[0x6f] = math::ddiv;
    handlers[0x70] = math::irem;
    handlers[0x71] = math::lrem;
    handlers[0x72] = math::frem;
    handlers[0x73] = math::drem;
    // handlers[0x74] = math::ineg;
    // handlers[0x75] = math::lneg;
    // handlers[0x76] = math::fneg;
    // handlers[0x77] = math::dneg;

    handlers[0x78] = math::ishl;
    handlers[0x79] = math::lshl;
    handlers[0x7a] = math::ishr;
    handlers[0x7b] = math::lshr;
    handlers[0x7c] = math::iushr;
    handlers[0x7d] = math::lushr;
    handlers[0x7e] = math::iand;
    handlers[0x7f] = math::land;
    handlers[0x80] = math::ior;
    handlers[0x81] = math::lor;
    handlers[0x82] = math::ixor;
    handlers[0x83] = math::lxor;

    handlers[0x84] = math::iinc;
    handlers[0x85] = math::i2l;
    handlers[0x86] = math::i2f;
    handlers[0x87] = math::i2d;
    handlers[0x88] = math::l2i;
    handlers[0x89] = math::l2f;
    handlers[0x8a] = math::l2d;
    handlers[0x8b] = math::f2i;
    handlers[0x8c] = math::f2l;
    handlers[0x8d] = math::f2d;
    handlers[0x8e] = math::d2i;
    handlers[0x8f] = math::d2l;
    handlers[0x90] = math::d2f;
    handlers[0x91] = math::i2b;
    handlers[0x92] = math::i2c;
    handlers[0x93] = math::i2s;

    handlers[0x94] = math::lcmp;
    handlers[0x95] = math::fcmpl;
    handlers[0x96] = math::fcmpg;
    handlers[0x97] = math::dcmpl;
    handlers[0x98] = math::dcmpg;

    handlers[0x99] = control_flow::ifeq;
    handlers[0x9a] = control_flow::ifne;
    handlers[0x9b] = control_flow::iflt;
    handlers[0x9c] = control_flow::ifge;
    handlers[0x9d] = control_flow::ifgt;
    handlers[0x9e] = control_flow::ifle;

    handlers[0x9f] = control_flow::ificmpeq;
    handlers[0xa0] = control_flow::ificmpne;
    handlers[0xa1] = control_flow::ificmplt;
    handlers[0xa2] = control_flow::ificmpge;
    handlers[0xa3] = control_flow::ificmpgt;
    handlers[0xa4] = control_flow::ificmple;
    handlers[0xa5] = control_flow::ifacmpeq;
    handlers[0xa6] = control_flow::ifacmpne;
    handlers[0xa7] = control_flow::goto;
    handlers[0xab] = control_flow::lookupswitch;

    handlers[0xac] = control_flow::returnvalue;
    handlers[0xad] = control_flow::returnvalue;
    handlers[0xae] = control_flow::returnvalue;
    handlers[0xaf] = control_flow::returnvalue;
    handlers[0xb0] = control_flow::returnvalue;
    handlers[0xb1] = control_flow::returnvoid;

    handlers[0xb2] = field::getstatic;
    handlers[0xb3] = field::putstatic;
    handlers[0xb4] = field::getfield;
    handlers[0xb5] = field::putfield;

    handlers[0xb6] = invoke::invokevirtual;
    handlers[0xb7] = invoke::invokespecial;
    handlers[0xb8] = invoke::invokestatic;
    handlers[0xb9] = invoke::invokeinterface;

    handlers[0xbb] = misc::new;
    handlers[0xbc] = array::newarray;
    handlers[0xbd] = array::anewarray;
    handlers[0xbe] = array::arraylength;
    handlers[0xbf] = misc::athrow;

    handlers[0xc0] = control_flow::checkcast;
    handlers[0xc1] = control_flow::instanceof;
    handlers[0xc2] = stack::pop; // TODO: monitorenter
    handlers[0xc3] = stack::pop; // TODO: monitorexit
    handlers[0xc4] = wide::wide;
    handlers[0xc6] = control_flow::ifnull;
    handlers[0xc7] = control_flow::ifnonnull;
}
