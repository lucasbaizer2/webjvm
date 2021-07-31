use std::{
    collections::HashMap,
    ops::{Index, IndexMut},
};

use crate::exec::jvm::Jvm;
use classfile_parser::{attribute_info::CodeAttribute, method_info::MethodAccessFlags, ClassAccessFlags};

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
        matches!(self, JavaValue::Object(_))
    }

    pub fn is_array(&self) -> bool {
        matches!(self, JavaValue::Array(_))
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

    pub fn as_byte(&self) -> Result<i8, ()> {
        match self {
            JavaValue::Byte(x) => Ok(*x),
            _ => Err(()),
        }
    }

    pub fn as_boolean(&self) -> Result<bool, ()> {
        Ok(self.as_int()? == 1)
    }
}

#[derive(Debug, Clone)]
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

#[derive(Debug, Clone)]
pub enum InternalMetadata {
    Text(String),
    Numeric(usize),
}

impl InternalMetadata {
    pub fn into_string(self) -> String {
        match self {
            InternalMetadata::Text(val) => val,
            _ => panic!(),
        }
    }

    pub fn into_usize(self) -> usize {
        match self {
            InternalMetadata::Numeric(val) => val,
            _ => panic!(),
        }
    }
}

#[derive(Debug)]
pub struct JavaObject {
    pub class_id: usize,
    pub internal_metadata: HashMap<String, InternalMetadata>,
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

    pub fn get_internal_metadata(&self, name: &str) -> Option<&InternalMetadata> {
        self.internal_metadata.get(name)
    }

    pub fn set_internal_metadata(&mut self, name: &str, value: InternalMetadata) {
        self.internal_metadata.insert(String::from(name), value);
    }

    pub fn remove_internal_metadata(&mut self, name: &str) -> Option<InternalMetadata> {
        self.internal_metadata.remove(name)
    }
}

#[derive(Debug)]
pub struct JavaClass {
    pub java_type: String,
    pub access_flags: ClassAccessFlags,
    pub class_id: usize,
    pub superclass_id: Option<usize>,
    pub direct_interfaces: Vec<String>,
    pub is_array_type: bool,
    pub is_primitive_type: bool,
    pub static_fields: HashMap<String, JavaValue>,
    pub class_object_id: usize,
    pub is_initialized: bool,
}

impl JavaClass {
    fn get_static_field_declarer(jvm: &Jvm, root_class: &str, name: &str) -> RuntimeResult<usize> {
        jvm.ensure_class_loaded(root_class, true)?;

        let heap = jvm.heap.borrow();
        let mut current_class = &heap.loaded_classes[heap.loaded_classes_lookup[root_class]];
        loop {
            if current_class.static_fields.contains_key(name) {
                return Ok(current_class.class_id);
            }

            match current_class.superclass_id {
                Some(id) => current_class = &heap.loaded_classes[id],
                None => {
                    return Err(
                        jvm.throw_exception("java/lang/NoSuchFieldError", Some(&format!("{}.{}", root_class, name)))
                    )
                }
            }
        }
    }

    pub fn set_static_field(jvm: &Jvm, root_class: &str, name: &str, val: JavaValue) -> RuntimeResult<()> {
        let declarer = JavaClass::get_static_field_declarer(jvm, root_class, name)?;

        let mut heap = jvm.heap.borrow_mut();
        let loaded_class = &mut heap.loaded_classes[declarer];
        loaded_class.static_fields.insert(String::from(name), val);

        Ok(())
    }

    pub fn get_static_field(jvm: &Jvm, root_class: &str, name: &str) -> RuntimeResult<JavaValue> {
        let declarer = JavaClass::get_static_field_declarer(jvm, root_class, name)?;

        let mut heap = jvm.heap.borrow_mut();
        let loaded_class = &mut heap.loaded_classes[declarer];
        Ok(loaded_class.static_fields.get(name).unwrap().clone())
    }
}

#[derive(Debug)]
pub struct CallStackFrame {
    pub container_class: String,
    pub container_method: String,
    pub is_native_frame: bool,
    pub access_flags: MethodAccessFlags,
    pub metadata: Option<CodeAttribute>,
    pub instructions: Vec<u8>,
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
        JavaValueVec {
            vec: Vec::new(),
        }
    }

    pub fn jvm_debug(&self, jvm: &Jvm) -> String {
        let string_values: Vec<String> = self
            .vec
            .iter()
            .map(|val| match val {
                JavaValue::Object(inner) => match inner {
                    Some(id) => {
                        let heap = jvm.heap.borrow();
                        let cid = heap.object_heap_map[id].class_id;
                        heap.loaded_classes[cid].java_type.clone()
                    }
                    None => String::from("null"),
                },
                other => format!("{:?}", other),
            })
            .collect();
        format!("{:?}", string_values)
    }

    pub fn with_capacity(capacity: usize) -> JavaValueVec {
        JavaValueVec {
            vec: Vec::with_capacity(capacity),
        }
    }

    pub fn from_vec(vec: Vec<JavaValue>) -> JavaValueVec {
        JavaValueVec {
            vec,
        }
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
                JavaValue::Internal {
                    is_higher_bits,
                    ..
                } => {
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
                JavaValue::Internal {
                    is_higher_bits,
                    ..
                } => {
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

    pub fn insert(&mut self, index: usize, element: JavaValue) {
        self.vec.insert(index, element);
    }

    pub fn len(&self) -> usize {
        self.vec.len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
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
    pub interned_string_map: HashMap<String, usize>,
    pub object_id_offset: usize,
    pub main_thread_object: usize,
}

pub type RuntimeResult<T> = std::result::Result<T, JavaThrowable>;

#[derive(Debug)]
pub enum JavaThrowable {
    Handled(usize),
    Unhandled(usize),
}

#[derive(Debug)]
pub struct MethodDescriptor {
    pub argument_types: Vec<String>,
    pub return_type: String,
}

impl MethodDescriptor {
    fn read_token(desc: &[char], mut offset: usize) -> Result<(String, usize), ()> {
        let mut token = String::with_capacity(1);
        while desc[offset] == '[' {
            token.push(desc[offset]);
            offset += 1;
        }
        if offset == desc.len() {
            return Err(());
        }
        match desc[offset] {
            'B' | 'S' | 'I' | 'J' | 'F' | 'D' | 'C' | 'Z' | 'V' => {
                token.push(desc[offset]);
                offset += 1;
            }
            'L' => {
                while desc[offset] != ';' {
                    token.push(desc[offset]);
                    offset += 1;
                }
                token.push(';');
                offset += 1;
            }
            _ => return Err(()),
        }

        Ok((token, offset))
    }

    pub fn new(desc: &str) -> Result<MethodDescriptor, ()> {
        let chars: Vec<char> = desc.chars().collect();
        let open_paren = chars.iter().position(|ch| *ch == '(').unwrap();
        let mut argument_types = Vec::new();
        let mut offset = open_paren + 1;
        while offset < chars.len() && chars[offset] != ')' {
            let (token, new_offset) = MethodDescriptor::read_token(&chars, offset)?;
            argument_types.push(token);
            offset = new_offset;
        }
        if chars[offset] != ')' {
            return Err(());
        }
        offset += 1;
        let (return_type, _) = MethodDescriptor::read_token(&chars, offset)?;

        Ok(MethodDescriptor {
            argument_types,
            return_type,
        })
    }
}
