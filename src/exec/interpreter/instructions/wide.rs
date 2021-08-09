use crate::{
    exec::interpreter::{
        instructions::{load, math, store},
        InstructionEnvironment, InstructionHandler,
    },
    model::RuntimeResult,
};
use std::collections::HashMap;

lazy_static! {
    static ref WIDE_HANDLERS: HashMap<u8, InstructionHandler> = {
        let mut map: HashMap<u8, InstructionHandler> = HashMap::with_capacity(11);

        map.insert(0x15, load::iloadwide);
        map.insert(0x16, load::lloadwide);
        map.insert(0x17, load::floadwide);
        map.insert(0x18, load::dloadwide);
        map.insert(0x19, load::aloadwide);

        map.insert(0x36, store::istorewide);
        map.insert(0x37, store::lstorewide);
        map.insert(0x38, store::fstorewide);
        map.insert(0x39, store::dstorewide);
        map.insert(0x3a, store::astorewide);

        map.insert(0x84, math::iincwide);

        map
    };
}

pub fn wide(env: &mut InstructionEnvironment) -> RuntimeResult<()> {
    let (opcode,) = take_values!(env, u8);
    WIDE_HANDLERS[&opcode](env)
}
