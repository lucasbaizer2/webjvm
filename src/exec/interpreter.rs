use crate::{exec::jvm::*, model::*, StackTraceElement};
use crate::{util::*, InvokeType, JniEnv};
use classfile_parser::{
    code_attribute::Instruction,
    constant_info::{ConstantInfo, MethodRefConstant},
};
use std::cell::RefCell;
use std::num::Wrapping;

pub struct InstructionExecutor {
    instruction_count: RefCell<u64>,
}

impl InstructionExecutor {
    pub fn new() -> InstructionExecutor {
        InstructionExecutor {
            instruction_count: RefCell::new(0),
        }
    }

    pub fn step_until_stack_depth(&self, jvm: &Jvm, depth: usize) -> RuntimeResult<()> {
        while {
            let csf = jvm.call_stack_frames.borrow();
            csf.len()
        } > depth
        {
            self.step(jvm)?;
        }

        Ok(())
    }

    fn create_stack_frame(
        &self,
        jvm: &Jvm,
        state: &mut CallStackFrameState,
        invoke_type: InvokeType,
        const_pool: &Vec<ConstantInfo>,
        mr: &MethodRefConstant,
    ) -> RuntimeResult<CallStackFrame> {
        let class_str = get_constant_string(const_pool, mr.class_index);
        jvm.ensure_class_loaded(class_str, true)?;

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
            let val = state.stack.pop().expect("stack underflow");
            args.push_exact(val);
        }

        let instance = match invoke_type {
            InvokeType::Static => None,
            _ => {
                let object_instance = state.stack.pop().expect("stack underflow");
                match object_instance {
                    JavaValue::Object(instance_id) => match instance_id {
                        None => return Err(jvm.throw_npe()),
                        _ => (),
                    },
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
                    let heap = jvm.heap.borrow();
                    let instance = &heap.object_heap_map.get(&instance_id.unwrap()).expect("bad object ref");
                    let class = &heap.loaded_classes[instance.class_id];
                    class.java_type.clone()
                }
                _ => String::from("java/lang/Object"),
            },
            _ => class_str.clone(),
        };
        let declaring_class = match jvm.classpath.get_classpath_entry(declaring_class_name.as_str()) {
            Some(file) => file,
            None => {
                return Err(jvm.throw_exception("java/lang/NoClassDefError", Some(&declaring_class_name)));
            }
        };
        let (method_class, method) =
            match jvm.classpath.get_method(invoke_type, declaring_class, method_str.0, method_str.1) {
                Some(method) => method,
                None => {
                    return Err(jvm.throw_exception(
                        "java/lang/NoSuchMethodError",
                        Some(&format!("{}.{}{}", declaring_class_name, method_str.0, method_str.1)),
                    ));
                }
            };
        let mut frame = jvm.create_stack_frame(method_class, method)?;
        for i in 0..args.len() {
            frame.state.lvt[i] = args.remove(0);
        }
        Ok(frame)
    }

    fn get_native_step_env<'a>(&self, jvm: &'a Jvm, frame: &CallStackFrame) -> JniEnv<'a> {
        let csf = jvm.call_stack_frames.borrow();
        let mut stack_trace = Vec::with_capacity(csf.len());
        for frame in csf.iter() {
            stack_trace.push(StackTraceElement {
                class_name: frame.container_class.clone(),
                method: frame.container_method.clone(),
            });
        }
        JniEnv {
            jvm,
            container_class: frame.container_class.clone(),
            parameters: frame.state.lvt.clone(),
            stack_trace,
        }
    }

    fn push_constant(
        &self,
        jvm: &Jvm,
        state: &mut CallStackFrameState,
        const_pool: &Vec<ConstantInfo>,
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
                    let obj = jvm.create_string_object(str.as_str(), true);
                    JavaValue::Object(Some(obj))
                }
                x => panic!("bad string constant definition: {:?}", x),
            },
            ConstantInfo::Class(cc) => {
                let class_name = get_constant_string(&const_pool, cc.name_index);
                jvm.ensure_class_loaded(class_name, true)?;

                let heap = jvm.heap.borrow();
                let class_id = *heap.loaded_classes_lookup.get(class_name).unwrap();
                let class_object_id = heap.loaded_classes[class_id].class_object_id;

                JavaValue::Object(Some(class_object_id))
            }
            x => panic!("bad constant: {:?}", x),
        };
        state.stack.push(value);

        Ok(())
    }

    pub fn is_instance_of(&self, jvm: &Jvm, val: &JavaValue, compare_type: &str) -> RuntimeResult<bool> {
        let res = match val {
            JavaValue::Object(instance) => match instance {
                Some(instance_id) => {
                    let current_class = {
                        let heap = jvm.heap.borrow();
                        let obj = heap.object_heap_map.get(&instance_id).expect("bad object ref");
                        heap.loaded_classes[obj.class_id].java_type.clone()
                    };
                    jvm.is_assignable_from(compare_type, &current_class)?
                }
                None => true,
            },
            JavaValue::Array(_) => {
                // TODO
                true
            }
            _ => panic!("invalid object"),
        };
        Ok(res)
    }

    pub fn step(&self, jvm: &Jvm) -> RuntimeResult<()> {
        match self.step_unchecked(jvm) {
            Ok(_) => Ok(()),
            Err(ex) => match ex {
                JavaThrowable::Handled(_) => Ok(()),
                JavaThrowable::Unhandled(_) => return Err(ex),
            },
        }
    }

    fn step_unchecked(&self, jvm: &Jvm) -> RuntimeResult<()> {
        let ic = { self.instruction_count.borrow().clone() };
        if ic == 50000 {
            // unsafe { util::PERMIT_LOGGING = true }
        }
        {
            self.instruction_count.replace(ic + 1);
        }

        let (mut state, insn, container_class, depth) = {
            let is_native_frame = {
                let csf = jvm.call_stack_frames.borrow();
                let frame = csf.last().expect("no stack frame present");
                frame.is_native_frame
            };

            if is_native_frame {
                let return_value = {
                    let (env, jni_name) = {
                        let csf = jvm.call_stack_frames.borrow();
                        let frame = csf.last().expect("no stack frame present");
                        let env = self.get_native_step_env(jvm, &frame);

                        let method_name = &frame.container_method[0..frame.container_method.find('(').unwrap()];
                        let jni_name = format!("Java_{}_{}", frame.container_class.replace("/", "_"), method_name);

                        (env, jni_name)
                    };

                    let method = match jvm.classpath.get_native_method(&jni_name) {
                        Some(method) => method,
                        None => return Err(jvm.throw_exception("java/lang/UnsatisfiedLinkError", Some(&jni_name))),
                    };
                    method.invoke(&env)?
                };

                let mut csf = jvm.call_stack_frames.borrow_mut();
                csf.pop().unwrap();
                csf.last_mut().expect("stack underflow").state.return_stack_value = return_value;

                return Ok(());
            } else {
                let csf = jvm.call_stack_frames.borrow();
                let depth = csf.len();
                let frame = csf.last().expect("no stack frame present");
                let state = frame.state.clone();
                let insn = frame
                    .instructions
                    .iter()
                    .find(|insn| insn.0 == state.instruction_offset)
                    .expect("invalid offset")
                    .clone();

                (state, insn, frame.container_class.clone(), depth)
            }
        };

        macro_rules! pop {
            () => {{
                state.stack.pop().expect("stack underflow")
            }};
        }

        macro_rules! pop_full {
            () => {{
                state.stack.pop_full().expect("stack underflow")
            }};
        }

        macro_rules! use_const_pool {
            () => {{
                &jvm.classpath.get_classpath_entry(container_class.as_str()).unwrap().const_pool
            }};
        }

        macro_rules! update_stack {
            () => {{
                let mut csf = jvm.call_stack_frames.borrow_mut();
                let frame = csf.last_mut().expect("no stack frame present");
                if let Some(value) = &frame.state.return_stack_value {
                    state.stack.push(value.clone());
                    frame.state.return_stack_value = None;
                }
            }};
        }

        macro_rules! branch_to {
            ( $offset:expr ) => {{
                state.instruction_offset = (state.instruction_offset as isize + $offset as isize) as usize;
            }};
        }

        macro_rules! eval_if {
            ( $offset:expr, $op:tt $val:expr ) => {{
                let val = pop!();
                let int = val.as_int().expect("expecting integral value");
                if int $op $val {
                    branch_to!($offset);
                }
            }}
        }

        macro_rules! eval_ificmp {
            ( $offset:expr, $op:tt ) => {{
                let rhs = pop!();
                let lhs = pop!();

                let lhs_int = lhs.as_int().expect("expecting integral value");
                let rhs_int = rhs.as_int().expect("expecting integral value");

                if lhs_int $op rhs_int {
                    branch_to!($offset);
                }
            }}
        }

        macro_rules! eval_imath {
            ( $op:tt ) => {{
                let rhs = Wrapping(pop!().as_int().expect("expecting integral value"));
                let lhs = Wrapping(pop!().as_int().expect("expecting integral value"));
                state.stack.push(JavaValue::Int((lhs $op rhs).0));
            }}
        }

        macro_rules! eval_lmath {
            ( $op:tt ) => {{
                let rhs = Wrapping(pop_full!().as_long().expect("expecting long value"));
                let lhs = Wrapping(pop_full!().as_long().expect("expecting long value"));
                state.stack.push(JavaValue::Long((lhs $op rhs).0));
            }}
        }

        macro_rules! eval_fmath {
            ( $op:tt ) => {{
                let rhs = pop!().as_float().expect("expecting float value");
                let lhs = pop!().as_float().expect("expecting float value");
                state.stack.push(JavaValue::Float(lhs $op rhs));
            }}
        }

        let expected_offset = state.instruction_offset;

        match &insn.1 {
            Instruction::Aaload | Instruction::Caload | Instruction::Iaload | Instruction::Baload => {
                let index = pop!().as_int().expect("invalid array index");
                let arrayref_id = pop!().as_array().expect("invalid array instance ID");

                let heap = jvm.heap.borrow();
                let arrayref = heap.array_heap_map.get(&arrayref_id).expect("invalid array instance ID");

                if index >= arrayref.values.len() as i32 || index < 0 {
                    return Err(jvm.throw_exception(
                        "java/lang/ArrayIndexOutOfBoundsException",
                        Some(index.to_string().as_str()),
                    ));
                } else {
                    let val = arrayref.values[index as usize].clone();
                    state.stack.push(val);
                }
            }
            Instruction::Aastore | Instruction::Castore | Instruction::Iastore | Instruction::Bastore => {
                let value = pop!();
                let index = pop!().as_int().expect("invalid array index");
                let arrayref_id = pop!().as_array().expect("invalid array instance ID");

                let mut heap = jvm.heap.borrow_mut();
                let arrayref = heap.array_heap_map.get_mut(&arrayref_id).expect("invalid array instance ID");
                if index >= arrayref.values.len() as i32 || index < 0 {
                    return Err(jvm.throw_exception(
                        "java/lang/ArrayIndexOutOfBoundsException",
                        Some(index.to_string().as_str()),
                    ));
                } else {
                    arrayref.values[index as usize] = value;
                }
            }
            Instruction::Aconstnull => {
                state.stack.push(JavaValue::Object(None));
            }
            Instruction::Aload(register) => state.stack.push(state.lvt[*register as usize].clone()),
            Instruction::Aload0 => state.stack.push(state.lvt[0].clone()),
            Instruction::Aload1 => state.stack.push(state.lvt[1].clone()),
            Instruction::Aload2 => state.stack.push(state.lvt[2].clone()),
            Instruction::Aload3 => state.stack.push(state.lvt[3].clone()),
            Instruction::Anewarray(type_ref_id) => {
                let const_pool = use_const_pool!();
                let type_str = get_constant_string(const_pool, *type_ref_id);
                let type_id = jvm.ensure_class_loaded(type_str, true)?;

                let length = state.stack.pop().expect("stack underflow").as_int().expect("expected integral value");
                let arr = jvm.create_empty_array(JavaArrayType::Object(type_id), length as usize);

                state.stack.push(JavaValue::Array(arr))
            }
            Instruction::Areturn
            | Instruction::Ireturn
            | Instruction::Freturn
            | Instruction::Dreturn
            | Instruction::Lreturn => {
                let return_value = pop_full!();

                let mut csf = jvm.call_stack_frames.borrow_mut();
                csf.pop().unwrap();

                csf.last_mut().expect("stack underflow").state.return_stack_value = Some(return_value);

                return Ok(());
            }
            Instruction::Arraylength => {
                let arrayref_id = match pop!() {
                    JavaValue::Array(id) => id,
                    _ => panic!("invalid array instance ID"),
                };
                let heap = jvm.heap.borrow();
                let arrayref = heap.array_heap_map.get(&arrayref_id).expect("invalid array instance ID");
                state.stack.push(JavaValue::Int(arrayref.values.len() as i32));
            }
            Instruction::Astore(register) => state.lvt[*register as usize] = pop!(),
            Instruction::Astore0 => state.lvt[0] = pop!(),
            Instruction::Astore1 => state.lvt[1] = pop!(),
            Instruction::Astore2 => state.lvt[2] = pop!(),
            Instruction::Astore3 => state.lvt[3] = pop!(),
            Instruction::Athrow => {
                let ex = match pop!().as_object().expect("expecting object ref") {
                    Some(obj) => obj,
                    None => return Err(jvm.throw_npe()),
                };
                return Err(jvm.throw_exception_ref(ex));
            }
            Instruction::Bipush(val) => state.stack.push(JavaValue::Int(*val as i32)),
            Instruction::Checkcast(compare_type_id) => {
                let test = state.stack.last().expect("stack underflow");

                if (test.is_object() && test.as_object().unwrap().is_some()) || test.is_array() {
                    let const_pool = use_const_pool!();
                    let compare_type = get_constant_string(&const_pool, *compare_type_id);

                    if !self.is_instance_of(jvm, &test, compare_type)? {
                        return Err(jvm.throw_exception("java/lang/ClassCastException", Some(compare_type)));
                    }
                }
            }
            Instruction::Dup => {
                let top = state.stack.last().expect("stack underflow").clone();
                state.stack.push(top);
            }
            Instruction::Dup2 => {
                let top = state.stack.last().expect("stack underflow").clone();
                if state.stack.len() > 1 {
                    let under_top = state.stack[state.stack.len() - 2].clone();
                    state.stack.push(under_top);
                }
                state.stack.push(top);
            }
            Instruction::Dupx1 => {
                let top = state.stack.last().expect("stack underflow").clone();
                state.stack.insert(state.stack.len() - 2, top);
            }
            Instruction::Fcmpg | Instruction::Fcmpl => {
                let rhs = state.stack.pop().expect("stack underflow").as_float().expect("expecting float");
                let lhs = state.stack.pop().expect("stack underflow").as_float().expect("expecting float");
                if lhs.is_nan() || rhs.is_nan() {
                    let nan_value = match &insn.1 {
                        Instruction::Fcmpg => 1,
                        Instruction::Fcmpl => -1,
                        _ => panic!(),
                    };
                    state.stack.push(JavaValue::Int(nan_value));
                } else {
                    if lhs > rhs {
                        state.stack.push(JavaValue::Int(1));
                    } else if lhs == rhs {
                        state.stack.push(JavaValue::Int(0));
                    } else {
                        state.stack.push(JavaValue::Int(-1));
                    }
                }
            }
            Instruction::F2i => {
                let float = state.stack.pop().expect("stack underflow").as_float().expect("expecting float value");
                state.stack.push(JavaValue::Int(float as i32));
            }
            Instruction::Fconst0 => state.stack.push(JavaValue::Float(0.0)),
            Instruction::Fconst1 => state.stack.push(JavaValue::Float(0.0)),
            Instruction::Fconst2 => state.stack.push(JavaValue::Float(0.0)),
            Instruction::Fadd => eval_fmath!(+),
            Instruction::Fdiv => eval_fmath!(/),
            Instruction::Fmul => eval_fmath!(*),
            Instruction::Frem => eval_fmath!(%),
            Instruction::Fsub => eval_fmath!(-),
            Instruction::Goto(offset) => {
                branch_to!(*offset);
            }
            Instruction::Getfield(field_ref_id) => {
                let const_pool = use_const_pool!();
                match &const_pool[*field_ref_id as usize - 1] {
                    ConstantInfo::FieldRef(fr) => {
                        let field_str = get_constant_name_and_type(const_pool, fr.name_and_type_index);

                        let instance_id = match pop!() {
                            JavaValue::Object(id) => match id {
                                Some(val) => val,
                                None => return Err(jvm.throw_npe()),
                            },
                            _ => panic!("invalid object reference"),
                        };

                        let heap = jvm.heap.borrow();
                        let instance = heap.object_heap_map.get(&instance_id).expect("invalid object reference");

                        let value = instance.get_field(jvm, field_str.0)?.clone();
                        state.stack.push(value);
                    }
                    x => panic!("bad field ref: {:?}", x),
                }
            }
            Instruction::Getstatic(field_ref_id) => {
                let const_pool = use_const_pool!();
                match &const_pool[*field_ref_id as usize - 1] {
                    ConstantInfo::FieldRef(fr) => {
                        let class_str = get_constant_string(const_pool, fr.class_index);
                        let field_str = get_constant_name_and_type(const_pool, fr.name_and_type_index);

                        let field_value = JavaClass::get_static_field(jvm, class_str, field_str.0)?;
                        state.stack.push(field_value.clone());
                    }
                    x => panic!("bad field ref: {:?}", x),
                }
            }
            Instruction::I2b => {
                let int = pop!().as_int().expect("expecting integral value");
                state.stack.push(JavaValue::Byte(int as i8));
            }
            Instruction::I2c => {
                let int = pop!().as_int().expect("expecting integral value");
                state.stack.push(JavaValue::Char(int as u16));
            }
            Instruction::I2f => {
                let int = pop!().as_int().expect("expecting integral value");
                state.stack.push(JavaValue::Float(int as f32));
            }
            Instruction::I2l => {
                let int = pop!().as_int().expect("expecting integral value");
                state.stack.push(JavaValue::Long(int as i64));
            }
            Instruction::Iconst0 => state.stack.push(JavaValue::Int(0)),
            Instruction::Iconst1 => state.stack.push(JavaValue::Int(1)),
            Instruction::Iconst2 => state.stack.push(JavaValue::Int(2)),
            Instruction::Iconst3 => state.stack.push(JavaValue::Int(3)),
            Instruction::Iconst4 => state.stack.push(JavaValue::Int(4)),
            Instruction::Iconst5 => state.stack.push(JavaValue::Int(5)),
            Instruction::Iconstm1 => state.stack.push(JavaValue::Int(-1)),
            Instruction::Ifeq(offset) => eval_if!(*offset, == 0),
            Instruction::IfAcmpeq(_) | Instruction::IfAcmpne(_) => {
                let rhs = pop!();
                let lhs = pop!();
                let equal = match lhs {
                    JavaValue::Object(obj1) => match rhs {
                        JavaValue::Object(obj2) => match (obj1, obj2) {
                            (Some(inner1), Some(inner2)) => inner1 == inner2,
                            (None, None) => true,
                            _ => false,
                        },
                        _ => false,
                    },
                    JavaValue::Array(obj1) => match rhs {
                        JavaValue::Array(obj2) => obj1 == obj2,
                        _ => false,
                    },
                    _ => false,
                };
                match &insn.1 {
                    Instruction::IfAcmpeq(offset) => {
                        if equal {
                            branch_to!(*offset);
                        }
                    }
                    Instruction::IfAcmpne(offset) => {
                        if !equal {
                            branch_to!(*offset);
                        }
                    }
                    _ => panic!(),
                };
            }
            Instruction::IfIcmpeq(offset) => eval_ificmp!(*offset, ==),
            Instruction::IfIcmpge(offset) => eval_ificmp!(*offset, >=),
            Instruction::IfIcmpgt(offset) => eval_ificmp!(*offset, >),
            Instruction::IfIcmple(offset) => eval_ificmp!(*offset, <=),
            Instruction::IfIcmplt(offset) => eval_ificmp!(*offset, <),
            Instruction::IfIcmpne(offset) => eval_ificmp!(*offset, !=),
            Instruction::Ifge(offset) => eval_if!(*offset, >= 0),
            Instruction::Ifgt(offset) => eval_if!(*offset, > 0),
            Instruction::Ifle(offset) => eval_if!(*offset, <= 0),
            Instruction::Iflt(offset) => eval_if!(*offset, < 0),
            Instruction::Ifne(offset) => eval_if!(*offset, != 0),
            Instruction::Ifnonnull(offset) => {
                let val = pop!();
                match val {
                    JavaValue::Object(ptr) => match ptr {
                        Some(_) => branch_to!(*offset),
                        None => (),
                    },
                    JavaValue::Array(_) => branch_to!(*offset), // internally the way we store arrays they can never be null
                    _ => panic!("ifnonnull expecting object"),
                };
            }
            Instruction::Ifnull(offset) => {
                let val = pop!();
                match val {
                    JavaValue::Object(ptr) => match ptr {
                        None => branch_to!(*offset),
                        Some(_) => (),
                    },
                    JavaValue::Array(_) => (), // internally the way we store arrays they can never be null
                    _ => panic!("ifnull expecting object"),
                };
            }
            Instruction::Iinc {
                index,
                value,
            } => {
                let current_val = state.lvt[*index as usize].as_int().expect("expecting integral value");
                state.lvt[*index as usize] = JavaValue::Int(current_val + *value as i32);
            }
            Instruction::Iload(register) | Instruction::Fload(register) | Instruction::Lload(register) => {
                state.stack.push(state.lvt[*register as usize].clone())
            }
            Instruction::Iload0 | Instruction::Fload0 | Instruction::Lload0 => state.stack.push(state.lvt[0].clone()),
            Instruction::Iload1 | Instruction::Fload1 | Instruction::Lload1 => state.stack.push(state.lvt[1].clone()),
            Instruction::Iload2 | Instruction::Fload2 | Instruction::Lload2 => state.stack.push(state.lvt[2].clone()),
            Instruction::Iload3 | Instruction::Fload3 | Instruction::Lload3 => state.stack.push(state.lvt[3].clone()),
            Instruction::Instanceof(compare_type_id) => {
                let const_pool = use_const_pool!();
                let compare_type = get_constant_string(&const_pool, *compare_type_id);
                let res = self.is_instance_of(jvm, &pop!(), compare_type)?;
                state.stack.push(JavaValue::Boolean(res));
            }
            Instruction::Invokespecial(method_ref_id) => {
                let const_pool = use_const_pool!();
                match &const_pool[*method_ref_id as usize - 1] {
                    ConstantInfo::MethodRef(mr) => {
                        let stack_frame =
                            self.create_stack_frame(jvm, &mut state, InvokeType::Special, const_pool, mr)?;

                        {
                            let mut csf = jvm.call_stack_frames.borrow_mut();
                            csf.push(stack_frame);
                        }
                        self.step_until_stack_depth(jvm, depth)?;
                        update_stack!();
                    }
                    x => panic!("bad method ref: {:?}", x),
                }
            }
            Instruction::Invokestatic(method_ref_id) => {
                let const_pool = use_const_pool!();
                match &const_pool[*method_ref_id as usize - 1] {
                    ConstantInfo::MethodRef(mr) => {
                        let stack_frame =
                            self.create_stack_frame(jvm, &mut state, InvokeType::Static, const_pool, mr)?;

                        {
                            let mut csf = jvm.call_stack_frames.borrow_mut();
                            csf.push(stack_frame);
                        }
                        self.step_until_stack_depth(jvm, depth)?;
                        update_stack!();
                    }
                    x => panic!("bad method ref: {:?}", x),
                }
            }
            Instruction::Invokevirtual(index)
            | Instruction::Invokeinterface {
                index,
                ..
            } => {
                let const_pool = use_const_pool!();
                let mr = match &const_pool[*index as usize - 1] {
                    ConstantInfo::MethodRef(mr) => mr.clone(),
                    ConstantInfo::InterfaceMethodRef(imr) => MethodRefConstant {
                        class_index: imr.class_index,
                        name_and_type_index: imr.name_and_type_index,
                    },
                    x => panic!("bad method ref: {:?}", x),
                };
                let stack_frame = self.create_stack_frame(jvm, &mut state, InvokeType::Virtual, const_pool, &mr)?;

                {
                    let mut csf = jvm.call_stack_frames.borrow_mut();
                    csf.push(stack_frame);
                }
                self.step_until_stack_depth(jvm, depth)?;
                update_stack!();
            }
            Instruction::Ishl => {
                let shift_amount =
                    state.stack.pop().expect("stack underflow").as_int().expect("expecting integral value") & 0b11111;
                let value_to_shift =
                    state.stack.pop().expect("stack underflow").as_int().expect("expecting long value");
                state.stack.push(JavaValue::Int(value_to_shift << shift_amount));
            }
            Instruction::Ishr => {
                let shift_amount =
                    state.stack.pop().expect("stack underflow").as_int().expect("expecting integral value") & 0b11111;
                let value_to_shift =
                    state.stack.pop().expect("stack underflow").as_int().expect("expecting long value");
                state.stack.push(JavaValue::Int(value_to_shift >> shift_amount));
            }
            Instruction::Istore(register) => state.lvt[*register as usize] = pop!(),
            Instruction::Istore0 => state.lvt[0] = pop!(),
            Instruction::Istore1 => state.lvt[1] = pop!(),
            Instruction::Istore2 => state.lvt[2] = pop!(),
            Instruction::Istore3 => state.lvt[3] = pop!(),
            Instruction::Iushr => {
                let shift_amount =
                    state.stack.pop().expect("stack underflow").as_int().expect("expecting integral value") & 0b11111;
                let value_to_shift =
                    state.stack.pop().expect("stack underflow").as_int().expect("expecting integral value");
                state.stack.push(JavaValue::Int((value_to_shift as u32 >> shift_amount) as i32));
            }
            Instruction::Iadd => eval_imath!(+),
            Instruction::Iand => eval_imath!(&),
            Instruction::Idiv => eval_imath!(/),
            Instruction::Imul => eval_imath!(*),
            Instruction::Ior => eval_imath!(|),
            Instruction::Irem => eval_imath!(%),
            Instruction::Isub => eval_imath!(-),
            Instruction::Ixor => eval_imath!(^),
            Instruction::L2i => {
                let long = pop_full!().as_long().expect("expecting long value");
                state.stack.push(JavaValue::Int(long as i32));
            }
            Instruction::Lconst0 => state.stack.push(JavaValue::Long(0)),
            Instruction::Lconst1 => state.stack.push(JavaValue::Long(1)),
            Instruction::Lcmp => {
                let rhs = state.stack.pop_full().expect("stack underflow").as_long();
                let lhs = state.stack.pop_full().expect("stack underflow").as_long();
                if lhs > rhs {
                    state.stack.push(JavaValue::Int(1));
                } else if lhs == rhs {
                    state.stack.push(JavaValue::Int(-1));
                } else {
                    state.stack.push(JavaValue::Int(0));
                }
            }
            Instruction::Lstore(register) => state.lvt[*register as usize] = pop_full!(),
            Instruction::Lstore0 => state.lvt[0] = pop_full!(),
            Instruction::Lstore1 => state.lvt[1] = pop_full!(),
            Instruction::Lstore2 => state.lvt[2] = pop_full!(),
            Instruction::Lstore3 => state.lvt[3] = pop_full!(),
            Instruction::Ladd => eval_lmath!(+),
            Instruction::Land => eval_lmath!(&),
            Instruction::Ldiv => eval_lmath!(/),
            Instruction::Lmul => eval_lmath!(*),
            Instruction::Lor => eval_lmath!(|),
            Instruction::Lrem => eval_lmath!(%),
            Instruction::Lsub => eval_lmath!(-),
            Instruction::Lxor => eval_lmath!(^),
            Instruction::Lshl => {
                let shift_amount =
                    state.stack.pop().expect("stack underflow").as_int().expect("expecting integral value") & 0b111111;
                let value_to_shift =
                    state.stack.pop_full().expect("stack underflow").as_long().expect("expecting long value");
                state.stack.push(JavaValue::Long(value_to_shift << shift_amount));
            }
            Instruction::Ldc(constant_id) => {
                let const_pool = use_const_pool!();
                self.push_constant(jvm, &mut state, const_pool, *constant_id as usize)?;
            }
            Instruction::LdcW(constant_id) | Instruction::Ldc2W(constant_id) => {
                let const_pool = use_const_pool!();
                self.push_constant(jvm, &mut state, const_pool, *constant_id as usize)?;
            }
            Instruction::Lookupswitch {
                default,
                pairs,
            } => {
                let key = pop!().as_int().expect("expecting integral value");
                let branched = 'b: {
                    for pair in pairs {
                        if key == pair.0 {
                            branch_to!(pair.1);
                            break 'b true;
                        }
                    }
                    false
                };
                if !branched {
                    branch_to!(*default);
                }
            }
            Instruction::Monitorenter => {
                // TODO
                pop!();
            }
            Instruction::Monitorexit => {
                // TODO
                pop!();
            }
            Instruction::New(type_ref_id) => {
                let const_pool = use_const_pool!();
                let type_str = get_constant_string(const_pool, *type_ref_id);

                let instance = jvm.new_instance(type_str)?;
                let instance_id = jvm.heap_store_instance(instance);

                state.stack.push(JavaValue::Object(Some(instance_id)))
            }
            Instruction::Nop => (),
            Instruction::Newarray(primitive_type) => {
                let array_type = match primitive_type {
                    4 => JavaArrayType::Boolean,
                    5 => JavaArrayType::Char,
                    6 => JavaArrayType::Float,
                    7 => JavaArrayType::Double,
                    8 => JavaArrayType::Byte,
                    9 => JavaArrayType::Short,
                    10 => JavaArrayType::Int,
                    11 => JavaArrayType::Long,
                    _ => panic!("invalid array type code"),
                };

                let length = state.stack.pop().expect("stack underflow").as_int().expect("expected integral value");
                let arr = jvm.create_empty_array(array_type, length as usize);

                state.stack.push(JavaValue::Array(arr))
            }
            Instruction::Pop => {
                pop!();
            }
            Instruction::Pop2 => {
                pop!();
                state.stack.pop(); // second pop does not need to succeed
            }
            Instruction::Putfield(field_ref_id) => {
                let const_pool = use_const_pool!();
                match &const_pool[*field_ref_id as usize - 1] {
                    ConstantInfo::FieldRef(fr) => {
                        let field_str = get_constant_name_and_type(const_pool, fr.name_and_type_index);

                        let value = pop_full!();
                        let instance_id = match pop!() {
                            JavaValue::Object(id) => match id {
                                Some(val) => val,
                                None => return Err(jvm.throw_npe()),
                            },
                            _ => panic!("invalid object reference"),
                        };

                        let mut heap = jvm.heap.borrow_mut();
                        let instance = heap.object_heap_map.get_mut(&instance_id).expect("invalid object reference");

                        instance.set_field(jvm, field_str.0, value)?;
                    }
                    x => panic!("bad field ref: {:?}", x),
                }
            }
            Instruction::Putstatic(field_ref_id) => {
                let const_pool = use_const_pool!();
                match &const_pool[*field_ref_id as usize - 1] {
                    ConstantInfo::FieldRef(fr) => {
                        let class_str = get_constant_string(const_pool, fr.class_index);
                        let field_str = get_constant_name_and_type(const_pool, fr.name_and_type_index);

                        JavaClass::set_static_field(jvm, class_str, field_str.0, pop_full!())?;
                    }
                    x => panic!("bad field ref: {:?}", x),
                }
            }
            Instruction::Return => {
                let mut csf = jvm.call_stack_frames.borrow_mut();
                csf.pop().unwrap();
                return Ok(());
            }
            Instruction::Sipush(val) => state.stack.push(JavaValue::Int(*val as i32)),
            x => return Err(jvm.throw_exception("webjvm/lang/UnhandledInstructionError", Some(&format!("{:?}", x)))),
        }

        // if the offset was not changed by an instruction
        if expected_offset == state.instruction_offset {
            let csf = jvm.call_stack_frames.borrow();
            let frame = csf.last().expect("no stack frame present");
            let next_insn_offset =
                frame.instructions.iter().find(|insn| insn.0 > state.instruction_offset).expect("invalid offset").0;

            state.instruction_offset = next_insn_offset;
        }
        let mut csf = jvm.call_stack_frames.borrow_mut();
        csf.last_mut().unwrap().state = state;

        Ok(())
    }
}
