use crate::{
    exec::interpreter::InstructionEnvironment,
    model::{JavaValue, RuntimeResult},
};

macro_rules! define_varstore {
    ( $opcode:ident, $width:ident ) => {
        pub fn $opcode(env: &mut InstructionEnvironment) -> RuntimeResult<()> {
            let (index,) = take_values!(env, $width);
            env.state.lvt[index as usize] = pop_full!(env);
            Ok(())
        }
    };
}

macro_rules! define_store {
    ( $opcode:ident, $index:literal ) => {
        pub fn $opcode(env: &mut InstructionEnvironment) -> RuntimeResult<()> {
            let value = pop_full!(env);
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
            Ok(())
        }
    };
}

define_varstore!(istore, u8);
define_varstore!(istorewide, u16);
define_store!(istore0, 0);
define_store!(istore1, 1);
define_store!(istore2, 2);
define_store!(istore3, 3);

define_varstore!(astore, u8);
define_varstore!(astorewide, u16);
define_store!(astore0, 0);
define_store!(astore1, 1);
define_store!(astore2, 2);
define_store!(astore3, 3);

define_varstore!(fstore, u8);
define_varstore!(fstorewide, u16);
define_store!(fstore0, 0);
define_store!(fstore1, 1);
define_store!(fstore2, 2);
define_store!(fstore3, 3);

define_varstore!(dstore, u8);
define_varstore!(dstorewide, u16);
define_store!(dstore0, 0);
define_store!(dstore1, 1);
define_store!(dstore2, 2);
define_store!(dstore3, 3);

define_varstore!(lstore, u8);
define_varstore!(lstorewide, u16);
define_store!(lstore0, 0);
define_store!(lstore1, 1);
define_store!(lstore2, 2);
define_store!(lstore3, 3);
