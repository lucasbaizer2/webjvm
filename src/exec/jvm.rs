use crate::model::*;
use crate::{util::*, Classpath, InvokeType, JniEnv};
use classfile_parser::ClassAccessFlags;
use classfile_parser::{
    attribute_info::code_attribute_parser,
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
                interned_string_map: HashMap::new(),
                object_id_offset: 0,
                main_thread_object: 0,
            }),
        }
    }

    pub fn is_instance_of(&self, val: &JavaValue, compare_type: &str) -> RuntimeResult<bool> {
        let res = match val {
            JavaValue::Object(instance) => match instance {
                Some(instance_id) => {
                    let class_id = {
                        let heap = self.heap.borrow();
                        let obj = heap.object_heap_map.get(&instance_id).expect("bad object ref");
                        obj.class_id
                    };
                    self.is_assignable_from(compare_type, class_id)?
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

    pub fn create_stack_frame(&self, cls: &ClassFile, method: &MethodInfo) -> RuntimeResult<CallStackFrame> {
        let container_class = get_constant_string(&cls.const_pool, cls.this_class).clone();
        let container_method_descriptor = get_constant_string(&cls.const_pool, method.descriptor_index);
        let container_method =
            get_constant_string(&cls.const_pool, method.name_index).clone() + container_method_descriptor;

        if method.access_flags.contains(MethodAccessFlags::NATIVE) {
            let container_class = container_class.replace("$", "_00024");

            let md = MethodDescriptor::new(container_method_descriptor).expect("bad method descriptor");
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

            return Ok(CallStackFrame {
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
            });
        } else if method.access_flags.contains(MethodAccessFlags::ABSTRACT) {
            return Err(self.throw_exception(
                "java/lang/AbstractMethodError",
                Some(&format!("{}.{}", container_class, container_method)),
            ));
        } else {
            let (_, code_attribute) = code_attribute_parser(&method.attributes[0].info).unwrap();
            let instructions = code_attribute.code.clone();

            Ok(CallStackFrame {
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
            })
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

    pub fn get_class_name_from_id(&self, id: usize) -> String {
        let heap = self.heap.borrow();
        heap.loaded_classes[id].java_type.clone()
    }

    pub fn ensure_class_loaded(&self, cls: &str, initialize: bool) -> RuntimeResult<usize> {
        match {
            let heap = self.heap.borrow();
            heap.loaded_classes_lookup.get(cls).cloned()
        } {
            Some(id) => {
                if initialize {
                    let is_initialized = {
                        let heap = self.heap.borrow();
                        let class = &heap.loaded_classes[id];
                        class.is_initialized
                    };
                    if !is_initialized {
                        self.initialize_class(id)?;
                    }
                }

                Ok(id)
            }
            None => {
                let mut loaded_class = match cls.chars().next().unwrap() {
                    '[' => JavaClass {
                        java_type: String::from(cls),
                        class_id: 0,
                        access_flags: ClassAccessFlags::PUBLIC,
                        superclass_id: None,
                        static_fields: HashMap::new(),
                        class_object_id: 0,
                        is_array_type: true,
                        is_primitive_type: false,
                        direct_interfaces: vec![
                            String::from("java/io/Serializable"),
                            String::from("java/lang/Cloneable"),
                        ],
                        is_initialized: true,
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
                            class_id: 0,
                            access_flags: ClassAccessFlags::PUBLIC,
                            superclass_id: None,
                            static_fields: HashMap::new(),
                            class_object_id: 0,
                            is_array_type: false,
                            is_primitive_type: true,
                            direct_interfaces: Vec::new(),
                            is_initialized: true,
                        },
                        _ => {
                            let class_file = match self.classpath.get_classpath_entry(cls) {
                                Some(file) => file,
                                None => {
                                    return Err(self.throw_exception("java/lang/NoClassDefError", Some(cls)));
                                }
                            };

                            let superclass_id = match class_file.super_class {
                                0 => None,
                                id => Some(self.ensure_class_loaded(
                                    &get_constant_string(&class_file.const_pool, id),
                                    initialize,
                                )?),
                            };

                            let declared_fields: Vec<&FieldInfo> = class_file
                                .fields
                                .iter()
                                .filter(|field| field.access_flags.contains(FieldAccessFlags::STATIC))
                                .collect();
                            let mut static_fields = HashMap::with_capacity(declared_fields.len());
                            for field in &declared_fields {
                                static_fields.insert(
                                    get_constant_string(&class_file.const_pool, field.name_index).clone(),
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
                                class_id: 0,
                                access_flags: class_file.access_flags,
                                superclass_id,
                                static_fields,
                                class_object_id: 0,
                                is_array_type: false,
                                is_primitive_type: false,
                                direct_interfaces,
                                is_initialized: false,
                            }
                        }
                    },
                };

                let id = {
                    let mut heap = self.heap.borrow_mut();
                    let id = heap.loaded_classes.len();
                    loaded_class.class_id = id;
                    heap.loaded_classes.push(loaded_class);
                    heap.loaded_classes_lookup.insert(String::from(cls), id);

                    id
                };

                let env = JniEnv::empty(&self);
                // create java.lang.Class object after registering the class
                let lang_class_id = self.ensure_class_loaded("java/lang/Class", false)?;
                let class_object_id = env.new_instance(lang_class_id);
                env.invoke_instance_method(
                    InvokeType::Special,
                    class_object_id,
                    lang_class_id,
                    "<init>",
                    "(Ljava/lang/ClassLoader;)V",
                    &[JavaValue::Object(None)],
                )?;

                let is_class_type = {
                    let mut heap = self.heap.borrow_mut();
                    let is_class_type = {
                        let cls = heap.loaded_classes.get_mut(id).unwrap();
                        cls.class_object_id = class_object_id;
                        !cls.is_array_type && !cls.is_primitive_type
                    };

                    let java_class_obj = heap.object_heap_map.get_mut(&class_object_id).unwrap();
                    java_class_obj.internal_metadata.insert(String::from("class_id"), InternalMetadata::Numeric(id));
                    java_class_obj
                        .internal_metadata
                        .insert(String::from("class_name"), InternalMetadata::Text(String::from(cls)));

                    is_class_type
                };

                if is_class_type && initialize {
                    self.initialize_class(id)?;
                }

                Ok(id)
            }
        }
    }

    pub fn initialize_class(&self, class_id: usize) -> RuntimeResult<()> {
        {
            let heap = self.heap.borrow();
            if heap.loaded_classes[class_id].is_initialized {
                return Ok(());
            }
        }

        let cls = self.get_class_name_from_id(class_id);
        let class_file = match self.classpath.get_classpath_entry(&cls) {
            Some(file) => file,
            None => return Err(self.throw_exception("java/lang/NoClassDefError", Some(&cls))),
        };

        if class_file.super_class != 0 {
            let superclass = get_constant_string(&class_file.const_pool, class_file.super_class);
            let superclass_id = self.ensure_class_loaded(superclass, false)?;
            self.initialize_class(superclass_id)?;
        }

        if let Some(_) = self.classpath.get_static_method(class_file, "<clinit>", "()V") {
            {
                let mut heap = self.heap.borrow_mut();
                heap.loaded_classes[class_id].is_initialized = true;
            }

            let env = JniEnv::empty(self);
            env.invoke_static_method(class_id, "<clinit>", "()V", &[])?;
        }

        Ok(())
    }

    pub fn is_assignable_from(&self, superclass: &str, subclass_id: usize) -> RuntimeResult<bool> {
        let heap = self.heap.borrow();
        let mut current_class = &heap.loaded_classes[subclass_id];
        Ok('l: loop {
            if &current_class.java_type == superclass {
                break true;
            }
            for interface in &current_class.direct_interfaces {
                if interface == superclass {
                    break 'l true;
                }
            }

            let cls = match self.classpath.get_classpath_entry(&current_class.java_type) {
                Some(file) => file,
                None => return Err(self.throw_exception("java/lang/NoClassDefError", Some(&current_class.java_type))),
            };
            if cls.super_class == 0 {
                break 'l superclass == "java/lang/Object";
            }
            let superclass_name = get_constant_string(&cls.const_pool, cls.super_class);
            let class_id = heap.loaded_classes_lookup.get(superclass_name).expect("invalid superclass");
            current_class = &heap.loaded_classes[*class_id];
        })
    }

    pub fn throw_npe(&self) -> JavaThrowable {
        self.throw_exception("java/lang/NullPointerException", None)
    }

    pub fn throw_exception_ref(&self, reference: usize) -> JavaThrowable {
        let exception_class = {
            let heap = self.heap.borrow();
            let obj = &heap.object_heap_map.get(&reference).expect("expecting object ref");
            let cls = &heap.loaded_classes[obj.class_id];
            cls.java_type.clone()
        };

        let detail_str = {
            let env = JniEnv::empty(self);
            let detail_field = env.get_field(reference, "detailMessage");

            match detail_field.as_object().unwrap() {
                Some(id) => Some(env.get_string(id)),
                _ => None,
            }
        };
        return self.throw_exception(&exception_class, detail_str.as_deref());

        // {
        //     let env = JniEnv::empty(self);

        //     let detail_field = env.get_field(reference, "detailMessage");
        //     let detail_str = match detail_field.as_object().unwrap() {
        //         Some(id) => env.get_string(id),
        //         _ => String::from("no message"),
        //     };
        //     log_error(&format!("{}: {}", exception_class, detail_str));

        //     let arr = env.get_field(reference, "stackTrace").as_array().unwrap();
        //     let len = env.get_array_length(arr);
        //     log_error(&format!("len: {}", len));
        //     for i in 0..len {
        //         let e = env.get_array_element(arr, i).as_object().unwrap().unwrap();
        //         let res = env
        //             .invoke_instance_method(
        //                 InvokeType::Virtual,
        //                 e,
        //                 "java/lang/Object",
        //                 "toString",
        //                 "()Ljava/lang/String;",
        //                 &[],
        //             )
        //             .unwrap()
        //             .unwrap()
        //             .as_object()
        //             .unwrap()
        //             .unwrap();
        //         let res_str = env.get_string(res);
        //         log_error(&res_str);
        //     }
        // }

        // {
        //     let mut csf = self.call_stack_frames.borrow_mut();
        //     let top_frame = csf.last_mut().unwrap();
        //     if let Some(metadata) = top_frame.metadata.as_ref() {
        //         for exception_item in &metadata.exception_table {
        //             if top_frame.state.instruction_offset >= exception_item.start_pc as usize
        //                 && top_frame.state.instruction_offset <= exception_item.end_pc as usize
        //             {
        //                 let container_class = self.classpath.get_classpath_entry(&top_frame.container_class).unwrap();
        //                 let catch_type = get_constant_string(&container_class.const_pool, exception_item.catch_type);
        //                 // TODO: make this polymorphic
        //                 if &exception_class == catch_type {
        //                     // let top_frame_mut = csf
        //                     top_frame.state.instruction_offset = exception_item.handler_pc as usize;
        //                     return JavaThrowable::Handled(reference);
        //                 }
        //             }
        //         }
        //     }
        // }

        // return JavaThrowable::Unhandled(reference);
    }

    pub fn throw_exception(&self, exception_class: &str, message: Option<&str>) -> JavaThrowable {
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

        let stacktrace = {
            let csf = self.call_stack_frames.borrow();
            let mut stacktrace = String::new();
            for i in 0..csf.len() {
                let frame = &csf[csf.len() - i - 1];
                let source = match frame.is_native_frame {
                    true => "<native method>",
                    false => "<unknown source>",
                };
                writeln!(&mut stacktrace, "    at {}.{}:{}", frame.container_class, frame.container_method, source)
                    .unwrap();
            }
            stacktrace
        };

        loop {
            let mut csf = self.call_stack_frames.borrow_mut();
            if csf.len() == 1 {
                break;
            }
            let top_frame = csf.last_mut().unwrap();
            if let Some(metadata) = top_frame.metadata.as_ref() {
                for exception_item in &metadata.exception_table {
                    if top_frame.state.instruction_offset >= exception_item.start_pc as usize
                        && top_frame.state.instruction_offset <= exception_item.end_pc as usize
                    {
                        if exception_item.catch_type == 0 {
                            // TODO: finally blocks
                            return JavaThrowable::Unhandled(0);
                        }
                        let container_class = self.classpath.get_classpath_entry(&top_frame.container_class).unwrap();
                        let catch_type = get_constant_string(&container_class.const_pool, exception_item.catch_type);
                        // TODO: make this polymorphic
                        if exception_class == catch_type {
                            // let top_frame_mut = csf
                            top_frame.state.instruction_offset = exception_item.handler_pc as usize;
                            return JavaThrowable::Handled(0);
                        }
                    }
                }
            }
            csf.pop().unwrap();
        }

        if let Some(message_str) = message {
            log_error(&format!("{}: {}\n{}", exception_class, message_str, stacktrace));
        } else {
            log_error(&format!("{}\n{}", exception_class, stacktrace));
        }

        return JavaThrowable::Unhandled(0);
    }

    pub fn new_instance(&self, root_class_id: usize) -> RuntimeResult<JavaObject> {
        let mut instance_fields = HashMap::new();

        let mut class_name = {
            let heap = self.heap.borrow();
            &heap.loaded_classes[root_class_id].java_type.clone()
        };
        loop {
            self.ensure_class_loaded(class_name, true)?;
            let cls = self.classpath.get_classpath_entry(&class_name).unwrap();

            let declared_fields: Vec<&FieldInfo> =
                cls.fields.iter().filter(|field| !field.access_flags.contains(FieldAccessFlags::STATIC)).collect();
            for field in &declared_fields {
                instance_fields.insert(
                    get_constant_string(&cls.const_pool, field.name_index).clone(),
                    JavaValue::default(get_constant_string(&cls.const_pool, field.descriptor_index)),
                );
            }

            if cls.super_class == 0 {
                break;
            }

            class_name = get_constant_string(&cls.const_pool, cls.super_class);
        }

        Ok(JavaObject {
            class_id: root_class_id,
            instance_fields,
            internal_metadata: HashMap::new(),
        })
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

    pub fn create_constant_array(&self, array_type: JavaArrayType, values: Vec<JavaValue>) -> usize {
        let arr = JavaArray {
            array_type,
            values,
        };

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

        let arr = JavaArray {
            array_type,
            values,
        };

        self.heap_store_array(arr)
    }

    pub fn create_string_object(&self, inner: &str, intern: bool) -> usize {
        // let owned = String::from(inner);
        if intern {
            let heap = self.heap.borrow();
            if let Some(id) = heap.interned_string_map.get(inner) {
                return *id;
            }
        }

        let string_class = self.ensure_class_loaded("java/lang/String", true).unwrap();
        let mut instance = self.new_instance(string_class).unwrap();

        let chars: Vec<JavaValue> = inner.encode_utf16().into_iter().map(|c| JavaValue::Char(c)).collect();
        let array_id = self.create_constant_array(JavaArrayType::Char, chars);
        instance.set_field(self, "value", JavaValue::Array(array_id)).unwrap();

        let id = self.heap_store_instance(instance);
        if intern {
            let mut heap = self.heap.borrow_mut();
            heap.interned_string_map.insert(String::from(inner), id);
        }
        id
    }
}
