use classfile_parser::constant_info::ConstantInfo;

use crate::{
    exec::interpreter::InstructionEnvironment,
    model::{JavaClass, JavaValue, RuntimeResult},
    util::{get_constant_name_and_type, get_constant_string},
};

pub fn getfield(env: &mut InstructionEnvironment) -> RuntimeResult<()> {
    let (field_ref_id,) = take_values!(env, u16);
    let const_pool = use_const_pool!(env);
    match &const_pool[field_ref_id as usize - 1] {
        ConstantInfo::FieldRef(fr) => {
            let field_str = get_constant_name_and_type(const_pool, fr.name_and_type_index);

            let instance_id = match pop!(env) {
                JavaValue::Object(id) => match id {
                    Some(val) => val,
                    None => return Err(env.jvm.throw_npe()),
                },
                _ => panic!("invalid object reference"),
            };

            let heap = env.jvm.heap.borrow();
            let instance = heap.object_heap_map.get(&instance_id).expect("invalid object reference");

            let value = instance.get_field(env.jvm, field_str.0)?.clone();
            env.state.stack.push(value);
        }
        x => panic!("bad field ref: {:?}", x),
    }

    Ok(())
}

pub fn putfield(env: &mut InstructionEnvironment) -> RuntimeResult<()> {
    let (field_ref_id,) = take_values!(env, u16);
    let const_pool = use_const_pool!(env);
    match &const_pool[field_ref_id as usize - 1] {
        ConstantInfo::FieldRef(fr) => {
            let field_str = get_constant_name_and_type(const_pool, fr.name_and_type_index);

            let value = pop_full!(env);
            let instance_id = match pop!(env) {
                JavaValue::Object(id) => match id {
                    Some(val) => val,
                    None => return Err(env.jvm.throw_npe()),
                },
                _ => panic!("invalid object reference"),
            };

            let mut heap = env.jvm.heap.borrow_mut();
            let instance = heap.object_heap_map.get_mut(&instance_id).expect("invalid object reference");

            instance.set_field(env.jvm, field_str.0, value)?;
        }
        x => panic!("bad field ref: {:?}", x),
    }

    Ok(())
}

pub fn getstatic(env: &mut InstructionEnvironment) -> RuntimeResult<()> {
    let (field_ref_id,) = take_values!(env, u16);
    let const_pool = use_const_pool!(env);
    match &const_pool[field_ref_id as usize - 1] {
        ConstantInfo::FieldRef(fr) => {
            let class_str = get_constant_string(const_pool, fr.class_index);
            let field_str = get_constant_name_and_type(const_pool, fr.name_and_type_index);

            let field_value = JavaClass::get_static_field(env.jvm, class_str, field_str.0)?;
            env.state.stack.push(field_value);
        }
        x => panic!("bad field ref: {:?}", x),
    }

    Ok(())
}

pub fn putstatic(env: &mut InstructionEnvironment) -> RuntimeResult<()> {
    let (field_ref_id,) = take_values!(env, u16);
    let const_pool = use_const_pool!(env);
    match &const_pool[field_ref_id as usize - 1] {
        ConstantInfo::FieldRef(fr) => {
            let class_str = get_constant_string(const_pool, fr.class_index);
            let field_str = get_constant_name_and_type(const_pool, fr.name_and_type_index);

            JavaClass::set_static_field(env.jvm, class_str, field_str.0, pop_full!(env))?;
        }
        x => panic!("bad field ref: {:?}", x),
    }

    Ok(())
}
