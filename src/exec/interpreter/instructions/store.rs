use crate::{
    exec::interpreter::InstructionEnvironment,
    model::{CallStackFrameState, JavaValue, RuntimeResult},
};

macro_rules! define_varstore {
    ( $opcode:ident ) => {
        pub fn $opcode(mut env: InstructionEnvironment) -> RuntimeResult<CallStackFrameState> {
            let (index,) = take_values!(&mut env, u8);
            env.state.lvt[index as usize] = pop_full!(&mut env);
            Ok(env.state)
        }
    };
}

macro_rules! define_store {
    ( $opcode:ident, $index:literal ) => {
        pub fn $opcode(mut env: InstructionEnvironment) -> RuntimeResult<CallStackFrameState> {
            let value = pop_full!(&mut env);
            match &value {
                JavaValue::Double(_) | JavaValue::Long(_) => {
                    env.state.lvt[$index + 1] = JavaValue::Internal {
                        is_unset: false,
                        is_higher_bits: true,
                    };
                }
                _ => (),
            }
            env.state.lvt[$index] = value;
            Ok(env.state)
        }
    };
}

define_varstore!(istore);
define_store!(istore0, 0);
define_store!(istore1, 1);
define_store!(istore2, 2);
define_store!(istore3, 3);

define_varstore!(astore);
define_store!(astore0, 0);
define_store!(astore1, 1);
define_store!(astore2, 2);
define_store!(astore3, 3);

define_varstore!(fstore);
define_store!(fstore0, 0);
define_store!(fstore1, 1);
define_store!(fstore2, 2);
define_store!(fstore3, 3);

define_varstore!(dstore);
define_store!(dstore0, 0);
define_store!(dstore1, 1);
define_store!(dstore2, 2);
define_store!(dstore3, 3);

define_varstore!(lstore);
define_store!(lstore0, 0);
define_store!(lstore1, 1);
define_store!(lstore2, 2);
define_store!(lstore3, 3);
