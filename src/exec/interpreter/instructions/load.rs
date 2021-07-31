use crate::{
    exec::interpreter::InstructionEnvironment,
    model::{CallStackFrameState, RuntimeResult},
};

macro_rules! define_varload {
    ( $opcode:ident ) => {
        pub fn $opcode(mut env: InstructionEnvironment) -> RuntimeResult<CallStackFrameState> {
            let (index,) = take_values!(&mut env, u8);
            env.state.stack.push(env.state.lvt[index as usize].clone());
            Ok(env.state)
        }
    };
}

macro_rules! define_load {
    ( $opcode:ident, $index:literal ) => {
        pub fn $opcode(mut env: InstructionEnvironment) -> RuntimeResult<CallStackFrameState> {
            env.state.stack.push(env.state.lvt[$index].clone());
            Ok(env.state)
        }
    };
}

define_varload!(iload);
define_load!(iload0, 0);
define_load!(iload1, 1);
define_load!(iload2, 2);
define_load!(iload3, 3);

define_varload!(aload);
define_load!(aload0, 0);
define_load!(aload1, 1);
define_load!(aload2, 2);
define_load!(aload3, 3);

define_varload!(fload);
define_load!(fload0, 0);
define_load!(fload1, 1);
define_load!(fload2, 2);
define_load!(fload3, 3);

define_varload!(dload);
define_load!(dload0, 0);
define_load!(dload1, 1);
define_load!(dload2, 2);
define_load!(dload3, 3);

define_varload!(lload);
define_load!(lload0, 0);
define_load!(lload1, 1);
define_load!(lload2, 2);
define_load!(lload3, 3);
