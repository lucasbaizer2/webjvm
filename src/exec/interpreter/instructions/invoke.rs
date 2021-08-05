use crate::{
    exec::interpreter::InstructionEnvironment,
    model::{CallStackFrame, CallStackFrameState, JavaValue, JavaValueVec, MethodDescriptor, RuntimeResult},
    util::{get_constant_name_and_type, get_constant_string},
    InvokeType,
};
use classfile_parser::constant_info::{ConstantInfo, MethodRefConstant};

fn create_stack_frame(
    env: &mut InstructionEnvironment,
    invoke_type: InvokeType,
    const_pool: &[ConstantInfo],
    mr: &MethodRefConstant,
) -> RuntimeResult<CallStackFrame> {
    let class_str = get_constant_string(const_pool, mr.class_index);
    env.jvm.ensure_class_loaded(class_str, true)?;

    let method_str = get_constant_name_and_type(const_pool, mr.name_and_type_index);
    let parsed_descriptor = MethodDescriptor::new(method_str.1).expect("bad method descriptor");

    let args_len = parsed_descriptor
        .argument_types
        .iter()
        .map(|jt| match jt.as_str() {
            "D" | "J" => 2,
            _ => 1,
        })
        .sum();
    let mut args = JavaValueVec::with_capacity(match invoke_type {
        InvokeType::Static => args_len,
        _ => args_len + 1,
    });

    for _ in 0..args_len {
        let val = env.state.stack.pop().expect("stack underflow");
        args.push_exact(val);
    }

    let instance = match invoke_type {
        InvokeType::Static => None,
        _ => {
            let object_instance = env.state.stack.pop().expect("stack underflow");
            match object_instance {
                JavaValue::Object(instance_id) => {
                    if instance_id.is_none() {
                        return Err(env.jvm.throw_npe());
                    }
                }
                JavaValue::Array(_) => (),
                _ => panic!("bad object ref"),
            };
            args.push(object_instance.clone());
            Some(object_instance)
        }
    };

    args.reverse();

    let declaring_class_name = match invoke_type {
        InvokeType::Virtual => match instance.unwrap() {
            JavaValue::Object(instance_id) => {
                let heap = env.jvm.heap.borrow();
                let instance = &heap.object_heap_map.get(&instance_id.unwrap()).expect("bad object ref");
                let class = &heap.loaded_classes[instance.class_id];
                class.java_type.clone()
            }
            _ => String::from("java/lang/Object"),
        },
        _ => class_str.clone(),
    };
    let declaring_class = match env.jvm.classpath.get_classpath_entry(declaring_class_name.as_str()) {
        Some(file) => file,
        None => {
            return Err(env.jvm.throw_exception("java/lang/NoClassDefFoundError", Some(&declaring_class_name)));
        }
    };
    let (method_class, method) =
        match env.jvm.classpath.get_method(invoke_type, declaring_class, method_str.0, method_str.1) {
            Some(method) => method,
            None => {
                return Err(env.jvm.throw_exception(
                    "java/lang/NoSuchMethodError",
                    Some(&format!("{}.{}{}", declaring_class_name, method_str.0, method_str.1)),
                ));
            }
        };
    let mut frame = env.jvm.create_stack_frame(method_class, method)?;
    for i in 0..args.len() {
        frame.state.lvt[i] = args.remove(0);
    }
    Ok(frame)
}

pub fn invokevirtual(env: InstructionEnvironment) -> RuntimeResult<CallStackFrameState> {
    invoke_instance_method(env, false)
}

pub fn invokeinterface(env: InstructionEnvironment) -> RuntimeResult<CallStackFrameState> {
    invoke_instance_method(env, true)
}

pub fn invokespecial(mut env: InstructionEnvironment) -> RuntimeResult<CallStackFrameState> {
    let (method_ref_id,) = take_values!(&mut env, u16);
    let const_pool = use_const_pool!(&mut env);
    match &const_pool[method_ref_id as usize - 1] {
        ConstantInfo::MethodRef(mr) => {
            let stack_frame = create_stack_frame(&mut env, InvokeType::Special, const_pool, mr)?;

            {
                let mut csf = env.jvm.call_stack_frames.borrow_mut();
                csf.push(stack_frame);
            }
            env.executor.step_until_stack_depth(env.jvm, env.depth)?;
            update_stack!(&mut env);
        }
        x => panic!("bad method ref: {:?}", x),
    }

    Ok(env.state)
}

pub fn invokestatic(mut env: InstructionEnvironment) -> RuntimeResult<CallStackFrameState> {
    let (method_ref_id,) = take_values!(&mut env, u16);
    let const_pool = use_const_pool!(&mut env);
    match &const_pool[method_ref_id as usize - 1] {
        ConstantInfo::MethodRef(mr) => {
            let stack_frame = create_stack_frame(&mut env, InvokeType::Static, const_pool, mr)?;

            {
                let mut csf = env.jvm.call_stack_frames.borrow_mut();
                csf.push(stack_frame);
            }
            env.executor.step_until_stack_depth(env.jvm, env.depth)?;
            update_stack!(&mut env);
        }
        x => panic!("bad method ref: {:?}", x),
    }

    Ok(env.state)
}

#[inline]
fn invoke_instance_method(
    mut env: InstructionEnvironment,
    is_interface_method: bool,
) -> RuntimeResult<CallStackFrameState> {
    let (index,) = take_values!(&mut env, u16);
    if is_interface_method {
        take_values!(&mut env, u16);
    }

    let const_pool = use_const_pool!(&mut env);
    let mr = match &const_pool[index as usize - 1] {
        ConstantInfo::MethodRef(mr) => mr.clone(),
        ConstantInfo::InterfaceMethodRef(imr) => MethodRefConstant {
            class_index: imr.class_index,
            name_and_type_index: imr.name_and_type_index,
        },
        x => panic!("bad method ref: {:?}", x),
    };
    let stack_frame = create_stack_frame(&mut env, InvokeType::Virtual, const_pool, &mr)?;
    {
        let mut csf = env.jvm.call_stack_frames.borrow_mut();
        csf.push(stack_frame);
    }
    env.executor.step_until_stack_depth(env.jvm, env.depth)?;
    update_stack!(&mut env);

    Ok(env.state)
}
