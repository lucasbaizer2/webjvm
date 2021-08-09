use classfile_parser::constant_info::ConstantInfo;

use crate::{
    exec::interpreter::InstructionEnvironment,
    model::{JavaValue, RuntimeResult},
    util::get_constant_string,
};

macro_rules! define_const {
    ( $opcode:ident, $const_type:ident, $value:expr ) => {
        pub fn $opcode(env: &mut InstructionEnvironment) -> RuntimeResult<()> {
            env.state.stack.push(JavaValue::$const_type($value));
            Ok(())
        }
    };
}

define_const!(aconstnull, Object, None);

define_const!(iconst0, Int, 0);
define_const!(iconst1, Int, 1);
define_const!(iconst2, Int, 2);
define_const!(iconst3, Int, 3);
define_const!(iconst4, Int, 4);
define_const!(iconst5, Int, 5);
define_const!(iconstm1, Int, -1);

define_const!(fconst0, Float, 0f32);
define_const!(fconst1, Float, 1f32);
define_const!(fconst2, Float, 2f32);

define_const!(lconst0, Long, 0);
define_const!(lconst1, Long, 1);

define_const!(dconst0, Double, 0f64);
define_const!(dconst1, Double, 1f64);

fn push_constant(
    env: &mut InstructionEnvironment,
    const_pool: &[ConstantInfo],
    constant_id: usize,
) -> RuntimeResult<()> {
    let value = match &const_pool[constant_id - 1] {
        ConstantInfo::Integer(ic) => JavaValue::Int(ic.value),
        ConstantInfo::Long(lc) => JavaValue::Long(lc.value),
        ConstantInfo::Float(fc) => JavaValue::Float(fc.value),
        ConstantInfo::Double(dc) => JavaValue::Double(dc.value),
        ConstantInfo::String(sc) => match &const_pool[sc.string_index as usize - 1] {
            ConstantInfo::Utf8(inner) => {
                let str = inner.utf8_string.clone();
                let obj = env.jvm.create_string_object(str.as_str(), true);
                JavaValue::Object(Some(obj))
            }
            x => panic!("bad string constant definition: {:?}", x),
        },
        ConstantInfo::Class(cc) => {
            let class_name = get_constant_string(const_pool, cc.name_index);
            let class_id = env.jvm.ensure_class_loaded(class_name, true)?;

            let heap = env.jvm.heap.borrow();
            let class_object_id = heap.loaded_classes[class_id].class_object_id;

            JavaValue::Object(Some(class_object_id))
        }
        x => panic!("bad constant: {:?}", x),
    };
    env.state.stack.push(value);

    Ok(())
}

pub fn ldc(env: &mut InstructionEnvironment) -> RuntimeResult<()> {
    let (constant_id,) = take_values!(env, u8);
    let const_pool = use_const_pool!(env);
    push_constant(env, const_pool, constant_id as usize)?;

    Ok(())
}

pub fn ldcw(env: &mut InstructionEnvironment) -> RuntimeResult<()> {
    let (constant_id,) = take_values!(env, u16);
    let const_pool = use_const_pool!(env);
    push_constant(env, const_pool, constant_id as usize)?;

    Ok(())
}

pub fn bipush(env: &mut InstructionEnvironment) -> RuntimeResult<()> {
    let (val,) = take_values!(env, i8);
    env.state.stack.push(JavaValue::Int(val as i32));

    Ok(())
}

pub fn sipush(env: &mut InstructionEnvironment) -> RuntimeResult<()> {
    let (val,) = take_values!(env, i16);
    env.state.stack.push(JavaValue::Int(val as i32));

    Ok(())
}
