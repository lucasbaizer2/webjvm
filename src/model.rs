use std::{
    collections::HashMap,
    ops::{Index, IndexMut},
};

use crate::exec::jvm::Jvm;
use classfile_parser::{
    attribute_info::CodeAttribute, code_attribute::Instruction, method_info::MethodAccessFlags,
};

#[derive(Debug, Clone, PartialEq)]
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
    Internal {
        is_unset: bool,
        is_higher_bits: bool,
    },
}

impl JavaValue {
    pub fn default(descriptor: &str) -> JavaValue {
        match descriptor.chars().next().expect("invalid descriptor") {
            'B' => JavaValue::Byte(0),
            'S' => JavaValue::Short(0),
            'I' => JavaValue::Int(0),
            'J' => JavaValue::Long(0),
            'F' => JavaValue::Float(0.0),
            'D' => JavaValue::Double(0.0),
            'C' => JavaValue::Char(0),
            'Z' => JavaValue::Boolean(false),
            'L' | '[' => JavaValue::Object(None),
            _ => panic!("invalid descriptor"),
        }
    }

    pub fn as_int(&self) -> Result<i32, ()> {
        match self {
            JavaValue::Byte(x) => Ok(*x as i32),
            JavaValue::Short(x) => Ok(*x as i32),
            JavaValue::Int(x) => Ok(*x as i32),
            JavaValue::Char(x) => Ok(*x as i32),
            JavaValue::Boolean(x) => Ok(match x {
                true => 1,
                false => 0,
            }),
            _ => Err(()),
        }
    }

    pub fn as_long(&self) -> Result<i64, ()> {
        match self {
            JavaValue::Long(x) => Ok(*x),
            _ => Err(()),
        }
    }

    pub fn is_object(&self) -> bool {
        match self {
            JavaValue::Object(_) => true,
            _ => false,
        }
    }

    pub fn is_array(&self) -> bool {
        match self {
            JavaValue::Array(_) => true,
            _ => false,
        }
    }

    pub fn as_array(&self) -> Result<usize, ()> {
        match self {
            JavaValue::Array(x) => Ok(*x),
            _ => Err(()),
        }
    }

    pub fn as_object(&self) -> Result<Option<usize>, ()> {
        match self {
            JavaValue::Object(x) => Ok(*x),
            _ => Err(()),
        }
    }

    pub fn as_float(&self) -> Result<f32, ()> {
        match self {
            JavaValue::Float(x) => Ok(*x),
            _ => Err(()),
        }
    }

    pub fn as_double(&self) -> Result<f64, ()> {
        match self {
            JavaValue::Double(x) => Ok(*x),
            _ => Err(()),
        }
    }

    pub fn as_boolean(&self) -> Result<bool, ()> {
        Ok(self.as_int()? == 1)
    }
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
    pub internal_metadata: HashMap<String, String>,
    pub instance_fields: HashMap<String, JavaValue>,
}

impl JavaObject {
    pub fn set_field(&mut self, jvm: &Jvm, name: &str, val: JavaValue) -> RuntimeResult<()> {
        if !self.instance_fields.contains_key(name) {
            Err(jvm.throw_exception("java/lang/NoSuchFieldError", Some(name)))
        } else {
            self.instance_fields.insert(String::from(name), val);
            Ok(())
        }
    }

    pub fn get_field(&self, jvm: &Jvm, name: &str) -> RuntimeResult<&JavaValue> {
        match self.instance_fields.get(name) {
            Some(val) => Ok(val),
            None => Err(jvm.throw_exception("java/lang/NoSuchFieldError", Some(name))),
        }
    }

    pub fn get_internal_metadata(&self, name: &str) -> Option<&String> {
        self.internal_metadata.get(name)
    }

    pub fn set_internal_metadata(&mut self, name: &str, value: &str) {
        self.internal_metadata
            .insert(String::from(name), String::from(value));
    }
}

#[derive(Debug)]
pub struct JavaClass {
    pub java_type: String,
    pub direct_interfaces: Vec<String>,
    pub is_array_type: bool,
    pub is_primitive_type: bool,
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
pub struct JavaValueVec {
    vec: Vec<JavaValue>,
}

impl Index<usize> for JavaValueVec {
    type Output = JavaValue;

    fn index(&self, index: usize) -> &Self::Output {
        &self.vec[index]
    }
}

impl IndexMut<usize> for JavaValueVec {
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        &mut self.vec[index]
    }
}

impl JavaValueVec {
    pub fn new() -> JavaValueVec {
        JavaValueVec { vec: Vec::new() }
    }

    pub fn with_capacity(capacity: usize) -> JavaValueVec {
        JavaValueVec {
            vec: Vec::with_capacity(capacity),
        }
    }

    pub fn from_vec(vec: Vec<JavaValue>) -> JavaValueVec {
        JavaValueVec { vec }
    }

    pub fn push_exact(&mut self, val: JavaValue) {
        self.vec.push(val);
    }

    pub fn push(&mut self, val: JavaValue) {
        match val {
            JavaValue::Long(_) | JavaValue::Double(_) => {
                self.vec.push(val);
                self.vec.push(JavaValue::Internal {
                    is_unset: false,
                    is_higher_bits: true,
                });
            }
            x => self.vec.push(x),
        };
    }

    pub fn remove(&mut self, index: usize) -> JavaValue {
        self.vec.remove(index)
    }

    pub fn pop(&mut self) -> Option<JavaValue> {
        self.vec.pop()
    }

    pub fn pop_full(&mut self) -> Option<JavaValue> {
        let popped = self.vec.pop();
        match popped {
            Some(ret) => match ret {
                JavaValue::Internal { is_higher_bits, .. } => {
                    if is_higher_bits {
                        self.vec.pop()
                    } else {
                        Some(ret)
                    }
                }
                _ => Some(ret),
            },
            None => None,
        }
    }

    pub fn last(&self) -> Option<&JavaValue> {
        self.vec.last()
    }

    pub fn last_full(&self) -> Option<&JavaValue> {
        match self.vec.last() {
            Some(val) => match val {
                JavaValue::Internal { is_higher_bits, .. } => {
                    if *is_higher_bits {
                        self.vec.get(self.vec.len() - 2)
                    } else {
                        Some(val)
                    }
                }
                _ => Some(val),
            },
            None => None,
        }
    }

    // pub fn get_index(&self, index: usize) -> &JavaValue {
    //     let mut count = 0;
    //     for val in &self.vec {
    //         if count == index {
    //             return self.vec.get(count).unwrap();
    //         }

    //         let v: Vec<String> = Vec::new();
    //         let x = v[0];

    //         match val {
    //             JavaValue::Internal { is_higher_bits, .. } => {
    //                 if !is_higher_bits {
    //                     count += 1;
    //                 }
    //             }
    //             _ => count += 1,
    //         }
    //     }

    //     panic!("out of bounds");
    // }

    pub fn insert(&mut self, index: usize, element: JavaValue) {
        self.vec.insert(index, element);
    }

    pub fn len(&self) -> usize {
        self.vec.len()
    }

    pub fn reverse(&mut self) {
        self.vec.reverse();
    }
}

#[derive(Debug, Clone)]
pub struct CallStackFrameState {
    pub line_number: u32,
    pub instruction_offset: usize,
    pub stack: JavaValueVec,
    pub lvt: JavaValueVec,
    pub return_stack_value: Option<JavaValue>,
}

pub struct Heap {
    pub loaded_classes: Vec<JavaClass>,
    pub loaded_classes_lookup: HashMap<String, usize>,
    pub object_heap_map: HashMap<usize, JavaObject>,
    pub array_heap_map: HashMap<usize, JavaArray>,
    pub object_id_offset: usize,
    pub main_thread_object: usize,
}

pub type RuntimeResult<T> = std::result::Result<T, JavaThrowable>;

#[derive(Debug)]
pub enum JavaThrowable {
    Handled,
    Unhandled,
}
