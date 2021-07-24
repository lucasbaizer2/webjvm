use crate::{java::MethodDescriptor, util::*, Classpath, InvokeType, JniEnv};
use classfile_parser::{
    attribute_info::{code_attribute_parser, method_parameters_attribute_parser, CodeAttribute},
    code_attribute::{code_parser, Instruction},
    constant_info::{ConstantInfo, MethodRefConstant},
    field_info::{FieldAccessFlags, FieldInfo},
    method_info::{MethodAccessFlags, MethodInfo},
    ClassFile,
};
use std::{cell::RefCell, collections::HashMap, usize};

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
    Array(usize),
    InternalUnset,
}

#[derive(Debug)]
pub enum JavaArrayType {
    Byte,
    Short,
    Int,
    Long,
    Float,
    Double,
    Char,
    Boolean,
    Object(usize),
    Array(Box<JavaArrayType>),
}

#[derive(Debug)]
pub struct JavaArray {
    pub array_type: JavaArrayType,
    pub values: Vec<JavaValue>,
}

#[derive(Debug)]
pub struct JavaObject {
    pub class_id: usize,
    pub instance_fields: HashMap<String, JavaValue>,
}

impl JavaObject {
    pub fn set_field(&mut self, name: &str, val: JavaValue) {
        if !self.instance_fields.contains_key(name) {
            panic!("NoSuchFieldError: {}", name);
        }

        self.instance_fields.insert(String::from(name), val);
    }

    pub fn get_field(&self, name: &str) -> &JavaValue {
        self.instance_fields.get(name).expect("NoSuchFieldError")
    }
}

#[derive(Debug)]
pub struct JavaClass {
    pub java_type: String,
    pub static_fields: HashMap<String, JavaValue>,
    pub class_object_id: usize,
}

impl JavaClass {
    pub fn set_static_field(&mut self, name: &str, val: JavaValue) {
        if !self.static_fields.contains_key(name) {
            panic!("NoSuchFieldError: {}", name);
        }

        self.static_fields.insert(String::from(name), val);
    }

    pub fn get_static_field(&mut self, name: &str) -> &JavaValue {
        self.static_fields.get(name).expect("NoSuchFieldError")
    }
}

#[derive(Debug)]
pub struct CallStackFrame {
    pub container_class: String,
    pub container_method: String,
    pub is_native_frame: bool,
    pub access_flags: MethodAccessFlags,
    pub metadata: Option<CodeAttribute>,
    pub instructions: Vec<(usize, Instruction)>,
    pub state: CallStackFrameState,
}

#[derive(Debug, Clone)]
pub struct CallStackFrameState {
    pub line_number: u32,
    pub instruction_offset: usize,
    pub stack: Vec<JavaValue>,
    pub lvt: Vec<JavaValue>,
    pub return_stack_value: Option<JavaValue>,
}

pub struct Heap {
    pub loaded_classes: Vec<JavaClass>,
    pub loaded_classes_lookup: HashMap<String, usize>,
    pub object_heap_map: HashMap<usize, JavaObject>,
    pub array_heap_map: HashMap<usize, JavaArray>,
    pub object_id_offset: usize,
}

pub struct Jvm {
    pub executor: InstructionExecutor,
    pub classpath: Classpath,
    pub call_stack_frames: RefCell<Vec<CallStackFrame>>,
    pub heap: RefCell<Heap>,
}

impl Jvm {
    pub fn new(webjvm: Classpath) -> Jvm {
        Jvm {
            executor: InstructionExecutor::new(),
            classpath: webjvm,
            call_stack_frames: RefCell::new(Vec::new()),
            heap: RefCell::new(Heap {
                loaded_classes: Vec::new(),
                loaded_classes_lookup: HashMap::new(),
                object_heap_map: HashMap::new(),
                array_heap_map: HashMap::new(),
                object_id_offset: 0,
            }),
        }
    }

    pub fn create_stack_frame(cls: &ClassFile, method: &MethodInfo) -> CallStackFrame {
        let container_class = get_constant_string(&cls.const_pool, cls.this_class).clone();
        log(&format!(
            "n, d = {}, {}",
            method.name_index, method.descriptor_index
        ));
        let container_method = get_constant_string(&cls.const_pool, method.name_index).clone()
            + get_constant_string(&cls.const_pool, method.descriptor_index);

        if method.access_flags.contains(MethodAccessFlags::NATIVE) {
            // let parameters_info = method
            //     .attributes
            //     .iter()
            //     .find(|attribute| {
            //         get_constant_string(&cls.const_pool, attribute.attribute_name_index)
            //             == "MethodParameters"
            //     })
            //     .expect("missing MethodParameters");
            // let (_, mp) = method_parameters_attribute_parser(&parameters_info.info).unwrap();

            return CallStackFrame {
                container_class,
                container_method,
                access_flags: method.access_flags,
                is_native_frame: true,
                instructions: Vec::new(),
                state: CallStackFrameState {
                    line_number: 0,
                    instruction_offset: 0,
                    // lvt: vec![JavaValue::InternalUnset; mp.parameters.len()],
                    lvt: vec![JavaValue::InternalUnset; 16], // TODO: fix this hacky workaround
                    stack: Vec::new(),
                    return_stack_value: None,
                },
                metadata: None,
            };
        } else if method.access_flags.contains(MethodAccessFlags::ABSTRACT) {
            panic!("AbstractMethodError");
        }

        let (_, code_attribute) = code_attribute_parser(&method.attributes[0].info).unwrap();
        let (_, instructions) =
            code_parser(&code_attribute.code).expect("error parsing method instructions");
        log(&format!(
            "Creating new stack frame at {}.{} with instructions: {:?}",
            container_class, container_method, instructions
        ));

        CallStackFrame {
            container_class,
            container_method,
            access_flags: method.access_flags,
            is_native_frame: false,
            instructions,
            state: CallStackFrameState {
                line_number: 0,
                instruction_offset: 0,
                lvt: vec![JavaValue::InternalUnset; code_attribute.max_locals as usize],
                stack: Vec::with_capacity(code_attribute.max_stack as usize),
                return_stack_value: None,
            },
            metadata: Some(code_attribute),
        }
    }

    pub fn push_call_stack_frame(&self, frame: CallStackFrame) {
        let mut csf = self.call_stack_frames.borrow_mut();
        csf.push(frame);
    }

    pub fn get_stack_depth(&self) -> usize {
        let csf = self.call_stack_frames.borrow();
        csf.len()
    }

    fn ensure_class_loaded(&self, cls: &str) -> usize {
        match {
            let heap = self.heap.borrow();
            heap.loaded_classes_lookup.get(cls).cloned()
        } {
            Some(id) => id,
            None => {
                let class_file = self
                    .classpath
                    .get_classpath_entry(cls)
                    .expect("NoClassDefError");
                let declared_fields: Vec<&FieldInfo> = class_file
                    .fields
                    .iter()
                    .filter(|field| field.access_flags.contains(FieldAccessFlags::STATIC))
                    .collect();
                let mut static_fields = HashMap::with_capacity(declared_fields.len());
                for field in &declared_fields {
                    static_fields.insert(
                        get_constant_string(&class_file.const_pool, field.name_index).clone(),
                        JavaValue::InternalUnset,
                    );
                }

                let loaded_class = JavaClass {
                    java_type: String::from(cls),
                    static_fields,
                    class_object_id: 0,
                };

                let id = {
                    let mut heap = self.heap.borrow_mut();
                    heap.loaded_classes.push(loaded_class);
                    let id = heap.loaded_classes.len() - 1;
                    heap.loaded_classes_lookup.insert(String::from(cls), id);
                    id
                };

                let env = JniEnv::empty(&self);
                // create java.lang.Class object after registering the class
                let class_object_id = env.new_instance("java/lang/Class");
                env.invoke_instance_method(
                    InvokeType::Special,
                    class_object_id,
                    "java/lang/Class",
                    "<init>",
                    "(Ljava/lang/ClassLoader;)V",
                    &[JavaValue::Object(None)],
                );

                {
                    let mut heap = self.heap.borrow_mut();
                    heap.loaded_classes[id].class_object_id = class_object_id;
                }

                if let Some((static_class, static_initializer)) = self
                    .classpath
                    .get_static_method(class_file, "<clinit>", "()V")
                {
                    let depth = {
                        let csf = self.call_stack_frames.borrow();
                        csf.len()
                    };
                    let clinit_frame = Jvm::create_stack_frame(static_class, static_initializer);
                    log(&format!("Static initializer = {:?}", clinit_frame));
                    self.push_call_stack_frame(clinit_frame);
                    self.executor.step_until_stack_depth(&self, depth);
                }

                id
            }
        }
    }

    pub fn new_instance(&self, root_class_name: &str) -> JavaObject {
        let mut instance_fields = HashMap::new();
        let mut root_class_id = None;

        let mut class_name = root_class_name;
        loop {
            let cls = self
                .classpath
                .get_classpath_entry(&class_name)
                .expect("NoClassDefError");
            let class_id = self.ensure_class_loaded(class_name);
            if root_class_id == None {
                root_class_id = Some(class_id);
            }

            let declared_fields: Vec<&FieldInfo> = cls
                .fields
                .iter()
                .filter(|field| !field.access_flags.contains(FieldAccessFlags::STATIC))
                .collect();
            for field in &declared_fields {
                instance_fields.insert(
                    get_constant_string(&cls.const_pool, field.name_index).clone(),
                    JavaValue::InternalUnset,
                );
            }

            if cls.super_class == 0 {
                break;
            }

            class_name = get_constant_string(&cls.const_pool, cls.super_class);
        }

        JavaObject {
            class_id: root_class_id.unwrap(),
            instance_fields,
        }
    }

    pub fn heap_store_instance(&self, instance: JavaObject) -> usize {
        let mut heap = self.heap.borrow_mut();
        let idx = heap.object_id_offset;
        heap.object_heap_map.insert(idx, instance);
        heap.object_id_offset += 1;

        idx
    }

    pub fn heap_store_array(&self, array: JavaArray) -> usize {
        let mut heap = self.heap.borrow_mut();
        let idx = heap.object_id_offset;
        heap.array_heap_map.insert(idx, array);
        heap.object_id_offset += 1;

        idx
    }

    pub fn create_constant_array(
        &self,
        array_type: JavaArrayType,
        values: Vec<JavaValue>,
    ) -> usize {
        let arr = JavaArray { array_type, values };

        self.heap_store_array(arr)
    }

    pub fn create_empty_array(&self, array_type: JavaArrayType, length: usize) -> usize {
        let values = vec![
            match array_type {
                JavaArrayType::Byte => JavaValue::Byte(0),
                JavaArrayType::Short => JavaValue::Short(0),
                JavaArrayType::Int => JavaValue::Int(0),
                JavaArrayType::Long => JavaValue::Long(0),
                JavaArrayType::Float => JavaValue::Float(0.0),
                JavaArrayType::Double => JavaValue::Double(0.0),
                JavaArrayType::Char => JavaValue::Char(0),
                JavaArrayType::Boolean => JavaValue::Boolean(false),
                JavaArrayType::Object(_) | JavaArrayType::Array(_) => JavaValue::Object(None),
            };
            length
        ];

        let arr = JavaArray { array_type, values };

        self.heap_store_array(arr)
    }

    pub fn create_string_object(&self, inner: &str) -> usize {
        let mut instance = self.new_instance("java/lang/String");

        let chars: Vec<JavaValue> = inner
            .encode_utf16()
            .into_iter()
            .map(|c| JavaValue::Char(c))
            .collect();
        let array_id = self.create_constant_array(JavaArrayType::Char, chars);
        instance.set_field("value", JavaValue::Array(array_id));

        self.heap_store_instance(instance)
    }
}

pub struct InstructionExecutor {}

impl InstructionExecutor {
    pub fn new() -> InstructionExecutor {
        InstructionExecutor {}
    }

    pub fn initialize(&self, jvm: &Jvm) {
        jvm.ensure_class_loaded("java/lang/Object");
        jvm.ensure_class_loaded("java/lang/String");
        jvm.ensure_class_loaded("java/lang/Class");

        let env = JniEnv::empty(jvm);
        let cos = env.new_instance("webjvm/io/ConsoleOutputStream");
        env.invoke_instance_method(
            InvokeType::Special,
            cos,
            "webjvm/io/ConsoleOutputStream",
            "<init>",
            "()V",
            &[],
        );

        let stdout = env.new_instance("java/io/PrintStream");
        env.invoke_instance_method(
            InvokeType::Special,
            stdout,
            "java/io/PrintStream",
            "<init>",
            "(Ljava/io/OutputStream;)V",
            &[JavaValue::Object(Some(cos))],
        );

        let system_id = jvm.ensure_class_loaded("java/lang/System");
        let mut heap = jvm.heap.borrow_mut();
        let system_class = &mut heap.loaded_classes[system_id];
        log(&format!("system class = {:?}", system_class));
        system_class.set_static_field("out", JavaValue::Object(Some(stdout)));
    }

    pub fn step_until_stack_depth(&self, jvm: &Jvm, depth: usize) {
        while {
            let csf = jvm.call_stack_frames.borrow();
            csf.len()
        } > depth
        {
            self.step(jvm);
        }
    }

    fn create_stack_frame(
        &self,
        jvm: &Jvm,
        state: &mut CallStackFrameState,
        invoke_type: InvokeType,
        const_pool: &Vec<ConstantInfo>,
        mr: &MethodRefConstant,
    ) -> CallStackFrame {
        let class_str = get_constant_string(const_pool, mr.class_index);
        jvm.ensure_class_loaded(class_str);

        let method_str = get_constant_name_and_type(const_pool, mr.name_and_type_index);
        let parsed_descriptor = MethodDescriptor::new(method_str.1).expect("bad method descriptor");

        let mut args = Vec::with_capacity(match invoke_type {
            InvokeType::Static => parsed_descriptor.argument_types.len(),
            _ => parsed_descriptor.argument_types.len() + 1,
        });
        for _ in 0..parsed_descriptor.argument_types.len() {
            args.push(state.stack.pop().expect("stack underflow"));
        }

        let instance_id = match invoke_type {
            InvokeType::Static => None,
            _ => {
                let object_instance = state.stack.pop().expect("stack underflow");
                let instance_id = match object_instance {
                    JavaValue::Object(instance_id) => match instance_id {
                        None => panic!("NullPointerException"),
                        Some(inner_id) => inner_id,
                    },
                    _ => panic!("bad object ref"),
                };
                args.push(object_instance);
                Some(instance_id)
            }
        };

        args.reverse();

        let declaring_class_name = match invoke_type {
            InvokeType::Virtual => {
                let heap = jvm.heap.borrow();
                let instance = &heap
                    .object_heap_map
                    .get(&instance_id.unwrap())
                    .expect("bad object ref");
                let class = &heap.loaded_classes[instance.class_id];
                class.java_type.clone()
            }
            _ => class_str.clone(),
        };
        let declaring_class = jvm
            .classpath
            .get_classpath_entry(declaring_class_name.as_str())
            .expect("NoClassDefError");
        let (method_class, method) = jvm
            .classpath
            .get_method(invoke_type, declaring_class, method_str.0, method_str.1)
            .expect("NoSuchMethodError");

        let mut frame = Jvm::create_stack_frame(method_class, method);
        for i in 0..args.len() {
            frame.state.lvt[i] = args.remove(0);
        }
        frame
    }

    fn step_native(&self, jvm: &Jvm, frame: &CallStackFrame) -> Option<JavaValue> {
        let method_name = &frame.container_method[0..frame.container_method.find('(').unwrap()];
        let jni_name = format!(
            "Java_{}_{}",
            frame.container_class.replace("/", "_"),
            method_name
        );
        let method = jvm
            .classpath
            .get_native_method(&jni_name)
            .expect(&format!("UnsatisfiedLinkError: {}", jni_name));
        let env = JniEnv {
            jvm,
            container_class: frame.container_class.clone(),
            container_instance: match !frame.access_flags.contains(MethodAccessFlags::STATIC) {
                true => Some(match frame.state.lvt[0] {
                    JavaValue::Object(id) => id.expect("NullPointerException"),
                    _ => panic!("invalid object instance"),
                }),
                _ => None,
            },
            parameters: frame.state.lvt.clone(),
        };
        method.invoke(&env)
    }

    fn push_constant(
        &self,
        jvm: &Jvm,
        state: &mut CallStackFrameState,
        const_pool: &Vec<ConstantInfo>,
        constant_id: usize,
    ) {
        let value = match &const_pool[constant_id - 1] {
            ConstantInfo::Integer(ic) => JavaValue::Int(ic.value),
            ConstantInfo::Float(fc) => JavaValue::Float(fc.value),
            ConstantInfo::String(sc) => match &const_pool[sc.string_index as usize - 1] {
                ConstantInfo::Utf8(inner) => {
                    let str = inner.utf8_string.clone();
                    let obj = jvm.create_string_object(str.as_str());
                    JavaValue::Object(Some(obj))
                }
                x => panic!("bad string constant definition: {:?}", x),
            },
            ConstantInfo::Class(cc) => {
                let class_name = get_constant_string(&const_pool, cc.name_index);

                let heap = jvm.heap.borrow();
                let class_id = *heap
                    .loaded_classes_lookup
                    .get(class_name)
                    .expect("NoClassDefError");
                let class_object_id = heap.loaded_classes[class_id].class_object_id;

                JavaValue::Object(Some(class_object_id))
            }
            x => panic!("bad constant: {:?}", x),
        };
        state.stack.push(value);
    }

    pub fn step(&self, jvm: &Jvm) {
        let (mut state, insn, container_class, depth) = {
            let is_native_frame = {
                let csf = jvm.call_stack_frames.borrow();
                let frame = csf.last().expect("no stack frame present");
                frame.is_native_frame
            };

            if is_native_frame {
                let return_value = {
                    let csf = jvm.call_stack_frames.borrow();
                    let frame = csf.last().expect("no stack frame present");

                    self.step_native(jvm, &frame)
                };

                let mut csf = jvm.call_stack_frames.borrow_mut();
                csf.pop().unwrap();
                csf.last_mut()
                    .expect("stack underflow")
                    .state
                    .return_stack_value = return_value;

                return;
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

                log(&format!(
                    "Current frame state: in {}.{} -- {:?}",
                    frame.container_class, frame.container_method, state
                ));

                (state, insn, frame.container_class.clone(), depth)
            }
        };

        log(&format!("Next instruction: {:?}", insn));

        macro_rules! use_const_pool {
            () => {{
                &jvm.classpath
                    .get_classpath_entry(container_class.as_str())
                    .unwrap()
                    .const_pool
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
                state.instruction_offset =
                    (state.instruction_offset as isize + $offset as isize) as usize;
            }};
        }

        let expected_offset = state.instruction_offset;

        match &insn.1 {
            Instruction::Aastore => {
                let value = state.stack.pop().expect("stack underflow");
                let index = match state.stack.pop().expect("stack underflow") {
                    JavaValue::Int(i) => i,
                    _ => panic!("invalid array index"),
                };
                let arrayref_id = match state.stack.pop().expect("stack underflow") {
                    JavaValue::Array(id) => id,
                    _ => panic!("invalid array instance ID"),
                };

                let mut heap = jvm.heap.borrow_mut();
                let arrayref = heap
                    .array_heap_map
                    .get_mut(&arrayref_id)
                    .expect("invalid array instance ID");
                arrayref.values[index as usize] = value;
            }
            Instruction::Aconstnull => {
                state.stack.push(JavaValue::Object(None));
            }
            Instruction::Aload(register) => state.stack.push(state.lvt[*register as usize].clone()),
            Instruction::Aload0 => state.stack.push(state.lvt[0].clone()),
            Instruction::Aload1 => state.stack.push(state.lvt[1].clone()),
            Instruction::Aload2 => state.stack.push(state.lvt[2].clone()),
            Instruction::Aload3 => state.stack.push(state.lvt[3].clone()),
            Instruction::Astore(register) => {
                state.lvt[*register as usize] = state.stack.pop().expect("stack underflow")
            }
            Instruction::Astore0 => state.lvt[0] = state.stack.pop().expect("stack underflow"),
            Instruction::Astore1 => state.lvt[1] = state.stack.pop().expect("stack underflow"),
            Instruction::Astore2 => state.lvt[2] = state.stack.pop().expect("stack underflow"),
            Instruction::Astore3 => state.lvt[3] = state.stack.pop().expect("stack underflow"),
            Instruction::Anewarray(type_ref_id) => {
                let const_pool = use_const_pool!();
                let type_str = get_constant_string(const_pool, *type_ref_id);
                let type_id = jvm.ensure_class_loaded(type_str);

                let length = match state.stack.pop().expect("stack underflow") {
                    JavaValue::Int(i) => i as usize,
                    _ => panic!("illegal array length type"),
                };
                let arr = jvm.create_empty_array(JavaArrayType::Object(type_id), length);

                state.stack.push(JavaValue::Array(arr))
            }
            Instruction::Areturn | Instruction::Ireturn => {
                let return_value = state.stack.pop().expect("stack underflow");

                let mut csf = jvm.call_stack_frames.borrow_mut();
                csf.pop().unwrap();

                csf.last_mut()
                    .expect("stack underflow")
                    .state
                    .return_stack_value = Some(return_value);

                return;
            }
            Instruction::Bipush(val) => state.stack.push(JavaValue::Int(*val as i32)),
            Instruction::Checkcast(type_id) => {
                let test = state.stack.last().expect("stack underflow");
                // TODO: test type
            }
            Instruction::Dup => {
                let top = state.stack.last().expect("stack underflow").clone();
                state.stack.push(top);
            }
            Instruction::Goto(offset) => {
                branch_to!(*offset);
            }
            Instruction::Getfield(field_ref_id) => {
                let const_pool = use_const_pool!();
                match &const_pool[*field_ref_id as usize - 1] {
                    ConstantInfo::FieldRef(fr) => {
                        let field_str =
                            get_constant_name_and_type(const_pool, fr.name_and_type_index);

                        let instance_id = match state.stack.pop().expect("stack underflow") {
                            JavaValue::Object(id) => id.expect("NullPointerException"),
                            _ => panic!("invalid object reference"),
                        };

                        let heap = jvm.heap.borrow();
                        let instance = heap
                            .object_heap_map
                            .get(&instance_id)
                            .expect("invalid object reference");

                        let value = instance.get_field(field_str.0).clone();
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
                        let class_id = jvm.ensure_class_loaded(class_str);

                        let field_str =
                            get_constant_name_and_type(const_pool, fr.name_and_type_index);
                        let loaded_class = &mut jvm.heap.borrow_mut().loaded_classes[class_id];
                        let field_value = loaded_class.get_static_field(field_str.0);
                        state.stack.push(field_value.clone());
                    }
                    x => panic!("bad field ref: {:?}", x),
                }
            }
            Instruction::Iconst0 => state.stack.push(JavaValue::Int(0)),
            Instruction::Iconst1 => state.stack.push(JavaValue::Int(1)),
            Instruction::Iconst2 => state.stack.push(JavaValue::Int(2)),
            Instruction::Iconst3 => state.stack.push(JavaValue::Int(3)),
            Instruction::Iconst4 => state.stack.push(JavaValue::Int(4)),
            Instruction::Iconst5 => state.stack.push(JavaValue::Int(5)),
            Instruction::Iconstm1 => state.stack.push(JavaValue::Int(-1)),
            Instruction::Ifne(offset) => {
                let val = state.stack.pop().expect("stack underflow");
                let is_zero = match val {
                    JavaValue::Byte(b) => b == 0,
                    JavaValue::Short(b) => b == 0,
                    JavaValue::Int(b) => b == 0,
                    JavaValue::Long(b) => b == 0,
                    JavaValue::Char(b) => b == 0,
                    JavaValue::Boolean(b) => !b,
                    _ => panic!("ifne expecting integral value"),
                };
                if !is_zero {
                    branch_to!(*offset);
                }
            }
            Instruction::Ifnonnull(offset) => {
                let val = state.stack.pop().expect("stack underflow");
                match val {
                    JavaValue::Object(ptr) => match ptr {
                        Some(_) => branch_to!(*offset),
                        None => (),
                    },
                    _ => panic!("ifnonnull expecting object"),
                };
            }
            Instruction::Ifnull(offset) => {
                let val = state.stack.pop().expect("stack underflow");
                match val {
                    JavaValue::Object(ptr) => match ptr {
                        None => branch_to!(*offset),
                        Some(_) => (),
                    },
                    _ => panic!("ifnull expecting object"),
                };
            }
            Instruction::Iload(register) => state.stack.push(state.lvt[*register as usize].clone()),
            Instruction::Iload0 => state.stack.push(state.lvt[0].clone()),
            Instruction::Iload1 => state.stack.push(state.lvt[1].clone()),
            Instruction::Iload2 => state.stack.push(state.lvt[2].clone()),
            Instruction::Iload3 => state.stack.push(state.lvt[3].clone()),
            Instruction::Invokespecial(method_ref_id) => {
                let const_pool = use_const_pool!();
                match &const_pool[*method_ref_id as usize - 1] {
                    ConstantInfo::MethodRef(mr) => {
                        log(&format!("Stack: {:?}", state.stack));
                        let stack_frame = self.create_stack_frame(
                            jvm,
                            &mut state,
                            InvokeType::Special,
                            const_pool,
                            mr,
                        );

                        log(&format!("Special stack frame = {:?}", stack_frame));

                        {
                            let mut csf = jvm.call_stack_frames.borrow_mut();
                            csf.push(stack_frame);
                        }
                        self.step_until_stack_depth(jvm, depth);
                        update_stack!();
                    }
                    x => panic!("bad method ref: {:?}", x),
                }
            }
            Instruction::Invokestatic(method_ref_id) => {
                let const_pool = use_const_pool!();
                match &const_pool[*method_ref_id as usize - 1] {
                    ConstantInfo::MethodRef(mr) => {
                        let stack_frame = self.create_stack_frame(
                            jvm,
                            &mut state,
                            InvokeType::Static,
                            const_pool,
                            mr,
                        );
                        log(&format!("Static stack frame = {:?}", stack_frame));

                        {
                            let mut csf = jvm.call_stack_frames.borrow_mut();
                            csf.push(stack_frame);
                        }
                        self.step_until_stack_depth(jvm, depth);
                        update_stack!();
                    }
                    x => panic!("bad method ref: {:?}", x),
                }
            }
            Instruction::Invokevirtual(index) | Instruction::Invokeinterface { index, .. } => {
                let const_pool = use_const_pool!();
                let mr = match &const_pool[*index as usize - 1] {
                    ConstantInfo::MethodRef(mr) => mr.clone(),
                    ConstantInfo::InterfaceMethodRef(imr) => MethodRefConstant {
                        class_index: imr.class_index,
                        name_and_type_index: imr.name_and_type_index,
                    },
                    x => panic!("bad method ref: {:?}", x),
                };
                log(&format!("MR = {:?}", mr));
                let stack_frame =
                    self.create_stack_frame(jvm, &mut state, InvokeType::Virtual, const_pool, &mr);

                log(&format!("Virtual stack frame = {:?}", stack_frame));

                {
                    let mut csf = jvm.call_stack_frames.borrow_mut();
                    csf.push(stack_frame);
                }
                self.step_until_stack_depth(jvm, depth);
                update_stack!();
            }
            Instruction::Istore0 => state.lvt[0] = state.stack.pop().expect("stack underflow"),
            Instruction::Istore1 => state.lvt[1] = state.stack.pop().expect("stack underflow"),
            Instruction::Istore2 => state.lvt[2] = state.stack.pop().expect("stack underflow"),
            Instruction::Istore3 => state.lvt[3] = state.stack.pop().expect("stack underflow"),
            Instruction::Iushr => {
                let shift_amount = match state.stack.pop().expect("stack underflow") {
                    JavaValue::Int(val) => val & 0b11111,
                    _ => panic!("invalid shift amount"),
                };
                let value_to_shift = match state.stack.pop().expect("stack underflow") {
                    JavaValue::Int(val) => val,
                    _ => panic!("invalid value"),
                };
                state.stack.push(JavaValue::Int(
                    (value_to_shift as u32 >> shift_amount) as i32,
                ));
            }
            Instruction::Ixor => {
                let rhs = match state.stack.pop().expect("stack underflow") {
                    JavaValue::Int(val) => val & 0b11111,
                    _ => panic!("invalid value"),
                };
                let lhs = match state.stack.pop().expect("stack underflow") {
                    JavaValue::Int(val) => val,
                    _ => panic!("invalid value"),
                };
                state.stack.push(JavaValue::Int(lhs ^ rhs));
            }
            Instruction::Ldc(constant_id) => {
                let const_pool = use_const_pool!();
                self.push_constant(jvm, &mut state, const_pool, *constant_id as usize);
            }
            Instruction::LdcW(constant_id) => {
                let const_pool = use_const_pool!();
                self.push_constant(jvm, &mut state, const_pool, *constant_id as usize);
            }
            Instruction::Monitorenter => {
                // TODO
                state.stack.pop().expect("stack underflow");
            }
            Instruction::Monitorexit => {
                // TODO
                state.stack.pop().expect("stack underflow");
            }
            Instruction::New(type_ref_id) => {
                let const_pool = use_const_pool!();
                let type_str = get_constant_string(const_pool, *type_ref_id);

                let instance = jvm.new_instance(type_str);
                let instance_id = jvm.heap_store_instance(instance);

                state.stack.push(JavaValue::Object(Some(instance_id)))
            }
            Instruction::Pop => {
                state.stack.pop().expect("stack underflow");
            }
            Instruction::Pop2 => {
                state.stack.pop().expect("stack underflow");
                state.stack.pop(); // second pop does not need to succeed
            }
            Instruction::Putfield(field_ref_id) => {
                let const_pool = use_const_pool!();
                match &const_pool[*field_ref_id as usize - 1] {
                    ConstantInfo::FieldRef(fr) => {
                        let field_str =
                            get_constant_name_and_type(const_pool, fr.name_and_type_index);

                        let value = state.stack.pop().expect("stack underflow");
                        let instance_id = match state.stack.pop().expect("stack underflow") {
                            JavaValue::Object(id) => id.expect("NullPointerException"),
                            _ => panic!("invalid object reference"),
                        };

                        let mut heap = jvm.heap.borrow_mut();
                        let instance = heap
                            .object_heap_map
                            .get_mut(&instance_id)
                            .expect("invalid object reference");
                        log(&format!("Instance = {:?}", instance));

                        instance.set_field(field_str.0, value);
                    }
                    x => panic!("bad field ref: {:?}", x),
                }
            }
            Instruction::Putstatic(field_ref_id) => {
                let const_pool = use_const_pool!();
                match &const_pool[*field_ref_id as usize - 1] {
                    ConstantInfo::FieldRef(fr) => {
                        let class_str = get_constant_string(const_pool, fr.class_index);
                        let class_id = jvm.ensure_class_loaded(class_str);

                        let field_str =
                            get_constant_name_and_type(const_pool, fr.name_and_type_index);

                        let loaded_class = &mut jvm.heap.borrow_mut().loaded_classes[class_id];
                        loaded_class.set_static_field(
                            field_str.0,
                            state.stack.pop().expect("stack underflow"),
                        );
                    }
                    x => panic!("bad field ref: {:?}", x),
                }
            }
            Instruction::Return => {
                let mut csf = jvm.call_stack_frames.borrow_mut();
                csf.pop().unwrap();
                return;
            }
            Instruction::Sipush(val) => state.stack.push(JavaValue::Int(*val as i32)),
            x => panic!("unhandled instruction: {:?}", x),
        }

        // if the offset was not changed by an instruction
        if expected_offset == state.instruction_offset {
            let csf = jvm.call_stack_frames.borrow();
            let frame = csf.last().expect("no stack frame present");
            let next_insn_offset = frame
                .instructions
                .iter()
                .find(|insn| insn.0 > state.instruction_offset)
                .expect("invalid offset")
                .0;

            state.instruction_offset = next_insn_offset;
        }
        let mut csf = jvm.call_stack_frames.borrow_mut();
        csf.last_mut().unwrap().state = state;
    }
}
