use crate::model::*;
use crate::{java::MethodDescriptor, util::*, Classpath, InvokeType, JniEnv};
use classfile_parser::{
    attribute_info::code_attribute_parser,
    code_attribute::code_parser,
    field_info::{FieldAccessFlags, FieldInfo},
    method_info::{MethodAccessFlags, MethodInfo},
    ClassFile,
};
use std::fmt::Write;
use std::{cell::RefCell, collections::HashMap, usize};

use super::interpreter::InstructionExecutor;

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
                main_thread_object: 0,
            }),
        }
    }

    pub fn create_stack_frame(&self, cls: &ClassFile, method: &MethodInfo) -> CallStackFrame {
        let container_class = get_constant_string(&cls.const_pool, cls.this_class).clone();
        let container_method_descriptor =
            get_constant_string(&cls.const_pool, method.descriptor_index);
        let container_method = get_constant_string(&cls.const_pool, method.name_index).clone()
            + container_method_descriptor;

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
            let md =
                MethodDescriptor::new(container_method_descriptor).expect("bad method descriptor");
            let mut lvt_len = md
                .argument_types
                .iter()
                .map(|jt| match jt.as_str() {
                    "D" | "J" => 2,
                    _ => 1,
                })
                .sum();
            if !method.access_flags.contains(MethodAccessFlags::STATIC) {
                lvt_len += 1;
            }

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
                    lvt: JavaValueVec::from_vec(vec![
                        JavaValue::Internal {
                            is_unset: true,
                            is_higher_bits: false
                        };
                        lvt_len
                    ]), // TODO: fix this hacky workaround
                    stack: JavaValueVec::new(),
                    return_stack_value: None,
                },
                metadata: None,
            };
        } else if method.access_flags.contains(MethodAccessFlags::ABSTRACT) {
            // panic!("AbstractMethodError");
            self.throw_exception(
                "java/lang/AbstractMethodError",
                Some(&format!("{}.{}", container_class, container_method)),
            );
        } else {
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
                    lvt: JavaValueVec::from_vec(vec![
                        JavaValue::Internal {
                            is_unset: true,
                            is_higher_bits: false
                        };
                        code_attribute.max_locals as usize
                    ]),
                    stack: JavaValueVec::with_capacity(code_attribute.max_stack as usize),
                    return_stack_value: None,
                },
                metadata: Some(code_attribute),
            }
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

    pub fn ensure_class_loaded(&self, cls: &str, initialize: bool) -> usize {
        match {
            let heap = self.heap.borrow();
            heap.loaded_classes_lookup.get(cls).cloned()
        } {
            Some(id) => id,
            None => {
                let loaded_class = match cls.chars().next().unwrap() {
                    '[' => JavaClass {
                        java_type: String::from(cls),
                        static_fields: HashMap::new(),
                        class_object_id: 0,
                        is_array_type: true,
                        is_primitive_type: false,
                        direct_interfaces: vec![
                            String::from("java/io/Serializable"),
                            String::from("java/lang/Cloneable"),
                        ],
                    },
                    x => match x {
                        'B' | 'S' | 'I' | 'J' | 'F' | 'D' | 'C' | 'Z' => JavaClass {
                            java_type: String::from(match x {
                                'B' => "byte",
                                'S' => "short",
                                'I' => "int",
                                'J' => "long",
                                'F' => "float",
                                'D' => "double",
                                'C' => "char",
                                'Z' => "boolean",
                                _ => panic!(),
                            }),
                            static_fields: HashMap::new(),
                            class_object_id: 0,
                            is_array_type: false,
                            is_primitive_type: true,
                            direct_interfaces: Vec::new(),
                        },
                        _ => {
                            let class_file =
                                self.classpath.get_classpath_entry(cls).unwrap_or_else(|| {
                                    self.throw_exception("java/lang/NoClassDefError", Some(cls))
                                });

                            if class_file.super_class != 0 {
                                self.ensure_class_loaded(
                                    &get_constant_string(
                                        &class_file.const_pool,
                                        class_file.super_class,
                                    ),
                                    initialize,
                                );
                            }

                            let declared_fields: Vec<&FieldInfo> = class_file
                                .fields
                                .iter()
                                .filter(|field| {
                                    field.access_flags.contains(FieldAccessFlags::STATIC)
                                })
                                .collect();
                            let mut static_fields = HashMap::with_capacity(declared_fields.len());
                            for field in &declared_fields {
                                static_fields.insert(
                                    get_constant_string(&class_file.const_pool, field.name_index)
                                        .clone(),
                                    JavaValue::default(get_constant_string(
                                        &class_file.const_pool,
                                        field.descriptor_index,
                                    )),
                                );
                            }

                            let direct_interfaces = class_file
                                .interfaces
                                .iter()
                                .map(|id| get_constant_string(&class_file.const_pool, *id).clone())
                                .collect();

                            JavaClass {
                                java_type: String::from(cls),
                                static_fields,
                                class_object_id: 0,
                                is_array_type: false,
                                is_primitive_type: false,
                                direct_interfaces,
                            }
                        }
                    },
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

                let is_class_type = {
                    let mut heap = self.heap.borrow_mut();
                    let is_class_type = {
                        let cls = heap.loaded_classes.get_mut(id).unwrap();
                        cls.class_object_id = class_object_id;
                        !cls.is_array_type && !cls.is_primitive_type
                    };

                    let java_class_obj = heap.object_heap_map.get_mut(&class_object_id).unwrap();
                    java_class_obj
                        .internal_metadata
                        .insert(String::from("class_name"), String::from(cls));

                    is_class_type
                };

                if is_class_type && initialize {
                    self.initialize_class(cls);
                }

                id
            }
        }
    }

    pub fn initialize_class(&self, cls: &str) {
        let class_file = self
            .classpath
            .get_classpath_entry(cls)
            .unwrap_or_else(|| self.throw_exception("java/lang/NoClassDefError", Some(cls)));
        if let Some(_) = self
            .classpath
            .get_static_method(class_file, "<clinit>", "()V")
        {
            let env = JniEnv::empty(self);
            env.invoke_static_method(cls, "<clinit>", "()V", &[]);
        }
    }

    pub fn throw_npe(&self) -> ! {
        self.throw_exception("java/lang/NullPointerException", None);
    }

    pub fn throw_exception(&self, exception_class: &str, message: Option<&str>) -> ! {
        // let env = JniEnv::empty(self);
        // let msg = env.new_string(message);

        // let ex = env.new_instance(exception_class);
        // env.invoke_instance_method(
        //     InvokeType::Special,
        //     ex,
        //     exception_class,
        //     "<init>",
        //     "(Ljava/lang/String;)V",
        //     &[JavaValue::Object(Some(msg))],
        // );
        // env.invoke_instance_method(
        //     InvokeType::Virtual,
        //     ex,
        //     exception_class,
        //     "printStackTrace",
        //     "()V",
        //     &[],
        // );

        let csf = self.call_stack_frames.borrow();
        let mut stacktrace = String::new();
        for i in 0..csf.len() {
            let frame = &csf[csf.len() - i - 1];
            let source = match frame.is_native_frame {
                true => "<native method>",
                false => "<unknown source>",
            };
            write!(
                &mut stacktrace,
                "    at {}.{}:{}\n",
                frame.container_class, frame.container_method, source
            )
            .unwrap();
        }

        if let Some(message_str) = message {
            panic!("{}: {}\n{}", exception_class, message_str, stacktrace);
        } else {
            panic!("{} {}", exception_class, stacktrace);
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
                .expect(&format!("NoClassDefError: {}", class_name));
            let class_id = self.ensure_class_loaded(class_name, true);
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
                    JavaValue::default(get_constant_string(
                        &cls.const_pool,
                        field.descriptor_index,
                    )),
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
            internal_metadata: HashMap::new(),
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
        instance.set_field(self, "value", JavaValue::Array(array_id));

        self.heap_store_instance(instance)
    }
}
