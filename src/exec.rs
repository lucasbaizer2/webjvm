use crate::{util::*, Classpath};
use classfile_parser::{
    attribute_info::code_attribute_parser,
    code_attribute::{code_parser, Instruction},
    constant_info::ConstantInfo,
    method_info::MethodInfo,
    ClassFile,
};
use std::{collections::HashMap, rc::Rc, usize};

#[derive(Debug, Clone)]
pub enum JavaValue {
    Byte(i8),
    Short(i16),
    Int(i32),
    Long(i64),
    Float(f32),
    Double(f64),
    Char(u16),
    Boolean(bool),
    Object(Option<usize>),
    Array(Option<Rc<Vec<JavaValue>>>),
}

#[derive(Debug)]
pub struct JavaObject {
    pub java_type: String,
    pub instance_fields: HashMap<String, JavaValue>,
}

pub struct CallStackFrame {
    pub container_class: String,
    pub container_method: String,
    pub instructions: Vec<(usize, Instruction)>,
    pub state: CallStackFrameState,
}

#[derive(Clone)]
pub struct CallStackFrameState {
    pub line_number: u32,
    pub instruction_offset: usize,
    pub stack: Vec<JavaValue>,
    pub lvt: Vec<JavaValue>,
}

pub struct JvmExecutor {
    pub classpath: Classpath,
    pub call_stack_frames: Vec<CallStackFrame>,
    pub object_heap_map: HashMap<usize, Rc<JavaObject>>,
    pub object_id_offset: usize,
}

impl JvmExecutor {
    pub fn new(webjvm: Classpath) -> JvmExecutor {
        JvmExecutor {
            classpath: webjvm,
            call_stack_frames: Vec::new(),
            object_heap_map: HashMap::new(),
            object_id_offset: 0,
        }
    }

    pub fn create_stack_frame(cls: &ClassFile, method: &MethodInfo) -> CallStackFrame {
        let (_, code_attribute) = code_attribute_parser(&method.attributes[0].info).unwrap();
        let (_, instructions) =
            code_parser(&code_attribute.code).expect("error parsing method instructions");
        log(&format!("{:?}", instructions));

        CallStackFrame {
            container_class: get_constant_string(&cls.const_pool, cls.this_class).clone(),
            container_method: get_constant_string(&cls.const_pool, method.name_index).clone()
                + get_constant_string(&cls.const_pool, method.descriptor_index),
            instructions,
            state: CallStackFrameState {
                line_number: 0,
                instruction_offset: 0,
                lvt: Vec::with_capacity(code_attribute.max_locals as usize),
                stack: Vec::with_capacity(code_attribute.max_stack as usize),
            },
        }
    }

    pub fn push_call_stack_frame(&mut self, frame: CallStackFrame) {
        self.call_stack_frames.push(frame);
    }

    pub fn is_stack_empty(&self) -> bool {
        self.call_stack_frames.len() == 0
    }

    fn get_string_object(&mut self, inner: &str) -> usize {
        let chars: Vec<JavaValue> = inner
            .encode_utf16()
            .into_iter()
            .map(|c| JavaValue::Char(c))
            .collect();

        let mut instance_fields = HashMap::new();
        instance_fields.insert(
            String::from("value"),
            JavaValue::Array(Some(Rc::new(chars))),
        );

        let instance = JavaObject {
            java_type: String::from("java/lang/String"),
            instance_fields,
        };

        let idx = self.object_id_offset;
        self.object_heap_map.insert(idx, Rc::new(instance));
        self.object_id_offset += 1;

        idx
    }

    pub fn step(&mut self) {
        let frame = self
            .call_stack_frames
            .last()
            .expect("no stack frame present");
        let mut state = frame.state.clone();
        let insn = &frame.instructions[state.instruction_offset];

        log(&format!("{:?}", insn));

        match &insn.1 {
            Instruction::Getstatic(field_ref) => {}
            Instruction::Invokevirtual(method_ref_id) => {
                let const_pool = &self
                    .classpath
                    .get_classpath_entry(frame.container_class.as_str())
                    .unwrap()
                    .const_pool;
                match &const_pool[*method_ref_id as usize - 1] {
                    ConstantInfo::MethodRef(mr) => {
                        let class_str = get_constant_string(const_pool, mr.class_index);
                        let method_str = match &const_pool[mr.name_and_type_index as usize - 1] {
                            ConstantInfo::NameAndType(nat) => (
                                get_constant_string(const_pool, nat.name_index),
                                get_constant_string(const_pool, nat.descriptor_index),
                            ),
                            x => panic!("bad name and type: {:?}", x),
                        };

                        let declaring_class = &self
                            .classpath
                            .get_classpath_entry(class_str)
                            .expect("class not found");
                        let method = self.classpath.get_virtual_method(
                            declaring_class,
                            method_str.0,
                            method_str.1,
                        );

                        log(&format!("{:?}", method));
                    }
                    x => panic!("bad method ref: {:?}", x),
                }
            }
            Instruction::Ldc(constant_id) => {
                let const_pool = &self
                    .classpath
                    .get_classpath_entry(frame.container_class.as_str())
                    .unwrap()
                    .const_pool;
                let value = match &const_pool[*constant_id as usize - 1] {
                    ConstantInfo::Integer(ic) => JavaValue::Int(ic.value),
                    ConstantInfo::String(sc) => match &const_pool[sc.string_index as usize - 1] {
                        ConstantInfo::Utf8(inner) => {
                            let str = inner.utf8_string.clone();
                            let obj = self.get_string_object(str.as_str());
                            JavaValue::Object(Some(obj))
                        }
                        x => panic!("bad string constant definition: {:?}", x),
                    },
                    x => panic!("bad constant: {:?}", x),
                };
                state.stack.push(value);
            }
            Instruction::Return => {
                log(&format!("unhandled instruction: {:?}", state.stack));
                self.call_stack_frames.pop().unwrap();
                return;
            }
            x => {
                log(&format!("unhandled instruction: {:?}", x));
                panic!();
            }
        }

        state.instruction_offset += 1;
        self.call_stack_frames.last_mut().unwrap().state = state;
    }
}
