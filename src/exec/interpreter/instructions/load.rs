use crate::{exec::interpreter::InstructionEnvironment, model::RuntimeResult};

macro_rules! define_varload {
    ( $opcode:ident, $width:ident ) => {
        pub fn $opcode(env: &mut InstructionEnvironment) -> RuntimeResult<()> {
            let (index,) = take_values!(env, $width);
            env.state.stack.push(env.state.lvt[index as usize].clone());
            Ok(())
        }
    };
}

macro_rules! define_load {
    ( $opcode:ident, $index:literal ) => {
        pub fn $opcode(env: &mut InstructionEnvironment) -> RuntimeResult<()> {
            env.state.stack.push(env.state.lvt[$index].clone());
            Ok(())
        }
    };
}

define_varload!(iload, u8);
define_varload!(iloadwide, u16);
define_load!(iload0, 0);
define_load!(iload1, 1);
define_load!(iload2, 2);
define_load!(iload3, 3);

define_varload!(aload, u8);
define_varload!(aloadwide, u16);
define_load!(aload0, 0);
define_load!(aload1, 1);
define_load!(aload2, 2);
define_load!(aload3, 3);

define_varload!(fload, u8);
define_varload!(floadwide, u16);
define_load!(fload0, 0);
define_load!(fload1, 1);
define_load!(fload2, 2);
define_load!(fload3, 3);

define_varload!(dload, u8);
define_varload!(dloadwide, u16);
define_load!(dload0, 0);
define_load!(dload1, 1);
define_load!(dload2, 2);
define_load!(dload3, 3);

define_varload!(lload, u8);
define_varload!(lloadwide, u16);
define_load!(lload0, 0);
define_load!(lload1, 1);
define_load!(lload2, 2);
define_load!(lload3, 3);
