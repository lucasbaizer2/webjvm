use super::jvm::Jvm;
use crate::{
    model::{
        CallStackFrame, CallStackFrameState, JavaArrayType, JavaThrowable, JavaValue, JavaValueVec,
        RuntimeResult,
    },
    InvokeType, StackTraceElement,
};
use classfile_parser::{
    method_info::{MethodAccessFlags, MethodInfo},
    ClassFile,
};

pub struct JniEnv<'a> {
    pub jvm: &'a Jvm,
    pub container_class: String,
    pub parameters: JavaValueVec,
    pub stack_trace: Vec<StackTraceElement>,
}

impl<'a> JniEnv<'a> {
    pub fn empty(jvm: &'a Jvm) -> JniEnv {
        JniEnv {
            jvm,
            container_class: String::new(),
            parameters: JavaValueVec::new(),
            stack_trace: Vec::new(),
        }
    }

    pub fn new_string(&self, str: &str) -> usize {
        self.jvm.create_string_object(str, false)
    }

    pub fn new_interned_string(&self, str: &str) -> usize {
        self.jvm.create_string_object(str, true)
    }

    pub fn new_array(&self, array_type: JavaArrayType, length: usize) -> usize {
        self.jvm.create_empty_array(array_type, length)
    }

    pub fn get_array_length(&self, array_id: usize) -> usize {
        let heap = self.jvm.heap.borrow();
        let array = heap
            .array_heap_map
            .get(&array_id)
            .expect("invalid array ref");
        array.values.len()
    }

    pub fn get_array_element(&self, array_id: usize, index: usize) -> JavaValue {
        let heap = self.jvm.heap.borrow();
        let array = heap
            .array_heap_map
            .get(&array_id)
            .expect("invalid array ref");
        array.values[index].clone()
    }

    pub fn set_array_element(&self, array_id: usize, index: usize, value: JavaValue) {
        let mut heap = self.jvm.heap.borrow_mut();
        let array = heap
            .array_heap_map
            .get_mut(&array_id)
            .expect("invalid array ref");
        array.values[index] = value;
    }

    pub fn get_main_thread(&self) -> usize {
        let heap = self.jvm.heap.borrow();
        heap.main_thread_object
    }

    pub fn load_class(&self, class: &str, initialize: bool) -> usize {
        self.jvm.ensure_class_loaded(class, initialize).unwrap()
    }

    pub fn get_class_id(&self, class: &str) -> usize {
        self.jvm.ensure_class_loaded(class, true).unwrap()
    }

    pub fn get_superclass(&self, subclass: &str) -> Option<String> {
        let subclass_id = self.load_class(subclass, false);

        let heap = self.jvm.heap.borrow();
        let class = &heap.loaded_classes[subclass_id];
        match class.superclass_id {
            0 => None,
            id => Some(heap.loaded_classes[id].java_type.clone()),
        }
    }

    pub fn get_class_object(&self, class_id: usize) -> usize {
        let heap = self.jvm.heap.borrow();
        let class = &heap.loaded_classes[class_id];
        class.class_object_id
    }

    pub fn get_object_type_name(&self, instance_id: usize) -> String {
        let heap = self.jvm.heap.borrow();
        let obj = &heap
            .object_heap_map
            .get(&instance_id)
            .expect("invalid object ref");
        let class = &heap.loaded_classes[obj.class_id];
        class.java_type.clone()
    }

    pub fn get_string(&self, str_id: usize) -> String {
        let heap = self.jvm.heap.borrow();
        let obj = heap
            .object_heap_map
            .get(&str_id)
            .expect("invalid object ref");
        if obj.class_id != self.get_class_id("java/lang/String") {
            panic!("invalid string ref: {:?}", obj);
        }

        let value_array = obj.get_field(self.jvm, "value").unwrap();
        match value_array {
            JavaValue::Array(ptr) => {
                let heap = self.jvm.heap.borrow();
                let array = &heap.array_heap_map[ptr];

                String::from_utf16(
                    &array
                        .values
                        .iter()
                        .map(|x| match x {
                            JavaValue::Char(val) => *val,
                            _ => panic!("invalid array item"),
                        })
                        .collect::<Vec<u16>>(),
                )
                .expect("invalid string encoding")
            }
            _ => panic!("invalid string value"),
        }
    }

    pub fn new_instance(&self, class: &str) -> usize {
        let obj = self.jvm.new_instance(class).unwrap();
        self.jvm.heap_store_instance(obj)
    }

    pub fn set_static_field(&self, class_name: &str, field_name: &str, value: JavaValue) {
        let mut heap = self.jvm.heap.borrow_mut();
        let lookup = heap.loaded_classes_lookup[class_name];
        let cls = &mut heap.loaded_classes[lookup];
        cls.set_static_field(field_name, value);
    }

    pub fn set_field(&self, instance_id: usize, field_name: &str, value: JavaValue) {
        let mut heap = self.jvm.heap.borrow_mut();
        let obj = heap
            .object_heap_map
            .get_mut(&instance_id)
            .expect("invalid instance ID");
        obj.set_field(self.jvm, field_name, value).unwrap();
    }

    pub fn get_field(&self, instance_id: usize, field_name: &str) -> JavaValue {
        let heap = self.jvm.heap.borrow();
        let obj = heap
            .object_heap_map
            .get(&instance_id)
            .expect("invalid instance ID");
        obj.get_field(self.jvm, field_name).unwrap().clone()
    }

    pub fn set_internal_metadata(&self, instance_id: usize, field_name: &str, value: &str) {
        let mut heap = self.jvm.heap.borrow_mut();
        let obj = heap
            .object_heap_map
            .get_mut(&instance_id)
            .expect("invalid instance ID");
        obj.set_internal_metadata(field_name, value);
    }

    pub fn get_internal_metadata(&self, instance_id: usize, field_name: &str) -> Option<String> {
        let heap = self.jvm.heap.borrow();
        let obj = heap
            .object_heap_map
            .get(&instance_id)
            .expect("invalid instance ID");
        obj.get_internal_metadata(field_name).cloned()
    }

    fn invoke_method(
        &self,
        method_class: &ClassFile,
        method: &MethodInfo,
        params: JavaValueVec,
    ) -> RuntimeResult<Option<JavaValue>> {
        let mut frame = self.jvm.create_stack_frame(method_class, method).unwrap();
        let mut index = 0;
        for i in 0..params.len() {
            frame.state.lvt[index] = params[i].clone();
            index += 1;
            match params[i] {
                JavaValue::Internal { is_higher_bits, .. } => {
                    if is_higher_bits {
                        index += 1;
                    }
                }
                _ => (),
            }
        }

        let depth = self.jvm.get_stack_depth();
        self.jvm.push_call_stack_frame(frame);
        self.jvm.executor.step_until_stack_depth(self.jvm, depth)?;

        let mut csf = self.jvm.call_stack_frames.borrow_mut();
        let this_frame = csf.last_mut().unwrap();
        if let Some(rsv) = &this_frame.state.return_stack_value {
            let clone = rsv.clone();
            this_frame.state.return_stack_value = None;
            Ok(Some(clone))
        } else {
            Ok(None)
        }
    }

    pub fn invoke_static_method(
        &self,
        class_name: &str,
        method_name: &str,
        method_descriptor: &str,
        params: &[JavaValue],
    ) -> RuntimeResult<Option<JavaValue>> {
        let class = self.get_class_file(&class_name);
        let (method_class, method) = match self.jvm.classpath.get_method(
            InvokeType::Static,
            class,
            method_name,
            method_descriptor,
        ) {
            Some(method) => method,
            None => {
                return Err(self.jvm.throw_exception(
                    "java/lang/NoSuchMethodError",
                    Some(&format!(
                        "{}.{}{}",
                        class_name, method_name, method_descriptor
                    )),
                ))
            }
        };
        self.invoke_method(
            method_class,
            method,
            JavaValueVec::from_vec(params.to_vec()),
        )
    }

    pub fn invoke_instance_method(
        &self,
        invoke_type: InvokeType,
        instance_id: usize,
        declaring_class: &str,
        method_name: &str,
        method_descriptor: &str,
        params: &[JavaValue],
    ) -> RuntimeResult<Option<JavaValue>> {
        let class_name = match invoke_type {
            InvokeType::Virtual => {
                let heap = self.jvm.heap.borrow();
                let obj = heap
                    .object_heap_map
                    .get(&instance_id)
                    .expect("invalid object ref");
                let class = &heap.loaded_classes[obj.class_id];
                class.java_type.clone()
            }
            InvokeType::Special => String::from(declaring_class),
            _ => panic!("invalid invoke type"),
        };
        let class = self.get_class_file(&class_name);
        let (method_class, method) =
            match self
                .jvm
                .classpath
                .get_method(invoke_type, class, method_name, method_descriptor)
            {
                Some(method) => method,
                None => {
                    return Err(self.jvm.throw_exception(
                        "java/lang/NoSuchMethodError",
                        Some(&format!(
                            "{}.{}{}",
                            class_name, method_name, method_descriptor
                        )),
                    ))
                }
            };

        let mut params_with_instance = Vec::with_capacity(params.len() + 1);
        params_with_instance.push(JavaValue::Object(Some(instance_id)));
        for val in params {
            params_with_instance.push(val.clone());
        }

        self.invoke_method(
            method_class,
            method,
            JavaValueVec::from_vec(params_with_instance),
        )
    }

    pub fn throw_exception(&self, exception_class: &str, message: Option<&str>) -> JavaThrowable {
        self.jvm.throw_exception(exception_class, message)
    }

    pub fn get_current_instance(&self) -> usize {
        self.parameters[0]
            .as_object()
            .expect("expecting object parameter")
            .unwrap()
    }

    pub fn get_class_file(&self, class_name: &str) -> &ClassFile {
        self.jvm
            .classpath
            .get_classpath_entry(&class_name)
            .unwrap_or_else(|| {
                self.jvm
                    .throw_exception("java/lang/NoClassDefError", Some(&class_name));
                panic!();
            })
    }
}

pub fn initialize(jvm: &Jvm) {
    let virtual_frame = CallStackFrame {
        container_class: String::from("webjvm/lang/Main"),
        container_method: String::from("main()V"),
        access_flags: MethodAccessFlags::empty(),
        instructions: Vec::new(),
        is_native_frame: false,
        metadata: None,
        state: CallStackFrameState {
            instruction_offset: 0,
            line_number: 0,
            lvt: JavaValueVec::new(),
            return_stack_value: None,
            stack: JavaValueVec::new(),
        },
    };
    jvm.push_call_stack_frame(virtual_frame);

    let required_classes = vec![
        "java/lang/Object",
        "java/lang/String",
        "java/lang/Class",
        "java/lang/Cloneable",
        "java/io/Serializable",
    ];
    for cls in &required_classes {
        jvm.ensure_class_loaded(cls, true).unwrap();
    }

    let env = JniEnv::empty(jvm);

    {
        let system_thread_group = env.new_instance("java/lang/ThreadGroup");
        env.invoke_instance_method(
            InvokeType::Special,
            system_thread_group,
            "java/lang/ThreadGroup",
            "<init>",
            "()V",
            &[],
        )
        .unwrap();

        let main_thread = env.new_instance("java/lang/Thread");
        env.set_field(
            main_thread,
            "name",
            JavaValue::Object(Some(env.new_string("main"))),
        );
        env.set_field(
            main_thread,
            "group",
            JavaValue::Object(Some(system_thread_group)),
        );
        env.set_field(main_thread, "priority", JavaValue::Int(5));
        let mut heap = jvm.heap.borrow_mut();
        heap.main_thread_object = main_thread;
    }

    // let cos = env.new_instance("webjvm/io/ConsoleOutputStream");
    // env.invoke_instance_method(
    //     InvokeType::Special,
    //     cos,
    //     "webjvm/io/ConsoleOutputStream",
    //     "<init>",
    //     "(Z)V",
    //     &[JavaValue::Boolean(false)],
    // );

    // let stdout = env.new_instance("java/io/PrintStream");
    // env.invoke_instance_method(
    //     InvokeType::Special,
    //     stdout,
    //     "java/io/PrintStream",
    //     "<init>",
    //     "(Ljava/io/OutputStream;)V",
    //     &[JavaValue::Object(Some(cos))],
    // );

    // let system_id = jvm.ensure_class_loaded("java/lang/System", true);
    // let mut heap = jvm.heap.borrow_mut();
    // let system_class = &mut heap.loaded_classes[system_id];
    // system_class.set_static_field("out", JavaValue::Object(Some(stdout)));

    env.invoke_static_method("java/lang/System", "initializeSystemClass", "()V", &[])
        .unwrap();
}
