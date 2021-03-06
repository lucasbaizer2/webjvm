#![feature(label_break_value)]
#![allow(clippy::new_without_default)]
#![allow(clippy::result_unit_err)]

#[macro_use]
extern crate lazy_static;

pub mod exec;
pub mod model;
pub mod native;
pub mod util;

use std::{collections::HashMap, io::Cursor};

use classfile_parser::{
    field_info::FieldInfo,
    method_info::{MethodAccessFlags, MethodInfo},
    *,
};
use exec::{env::JniEnv, jvm::*};
use model::*;
use util::*;
use wasm_bindgen::prelude::*;

pub struct StackTraceElement {
    pub class_name: String,
    pub method: String,
}

impl NativeMethod for js_sys::Function {
    fn invoke(&self, env: &JniEnv) -> RuntimeResult<Option<JavaValue>> {
        let res = self.call0(&JsValue::null()).expect("error invoking JavaScript function");
        if res.is_string() {
            let str: String = res.as_string().unwrap();
            return Ok(Some(JavaValue::Object(Some(env.new_string(&str)))));
        } else if res.is_null() {
            return Ok(Some(JavaValue::Object(None)));
        } else if let Some(double) = res.as_f64() {
            return Ok(Some(JavaValue::Double(double)));
        } else if let Some(bool) = res.as_bool() {
            return Ok(Some(JavaValue::Boolean(bool)));
        } else if !res.is_undefined() {
            panic!("Invalid returned value from native JavaScript method");
        }

        Ok(None)
    }

    fn get_name(&self) -> String {
        self.name().into()
    }
}

pub trait NativeMethod {
    fn invoke(&self, env: &JniEnv) -> RuntimeResult<Option<JavaValue>>;

    fn get_name(&self) -> String;
}

pub enum InvokeType {
    Virtual,
    Static,
    Special,
}

pub struct Classpath {
    class_files: HashMap<String, ClassFile>,
    native_methods: HashMap<String, Box<dyn NativeMethod>>,
}

impl Classpath {
    pub fn new() -> Classpath {
        Classpath {
            class_files: HashMap::new(),
            native_methods: HashMap::new(),
        }
    }

    pub fn add_native_method(&mut self, method: Box<dyn NativeMethod>) {
        self.native_methods.insert(method.get_name(), method);
    }

    pub fn add_classpath_entry(&mut self, class_bytes: &[u8]) {
        let cls = classfile_parser::parse_class_bytes(class_bytes).unwrap();
        self.class_files.insert(get_constant_string(&cls.const_pool, cls.this_class).clone(), cls);
    }

    pub fn add_classpath_jar(&mut self, jar_bytes: &[u8]) {
        use std::io::prelude::*;
        use zip::*;

        let mut cursor = Cursor::new(jar_bytes);
        let mut zip = ZipArchive::new(&mut cursor).expect("invalid zip archive");
        for i in 0..zip.len() {
            let mut file = zip.by_index(i).expect("invalid zip file content");
            if file.name().ends_with(".class") {
                let mut bytes = Vec::with_capacity(file.size() as usize);
                file.read_to_end(&mut bytes).expect("error reading zip");
                self.add_classpath_entry(bytes.as_slice());
            }
        }
    }

    #[allow(clippy::borrowed_box)]
    pub fn get_native_method(&self, name: &str) -> Option<&Box<dyn NativeMethod>> {
        self.native_methods.get(name)
    }

    pub fn get_classpath_entry(&self, name: &str) -> Option<&ClassFile> {
        self.class_files.get(name)
    }

    pub fn get_field<'a>(
        &self,
        declaring_class: &'a ClassFile,
        field_name: &str,
        field_descriptor: &str,
    ) -> Option<&'a FieldInfo> {
        declaring_class.fields.iter().find(|field| {
            get_constant_string(&declaring_class.const_pool, field.name_index) == field_name
                && get_constant_string(&declaring_class.const_pool, field.descriptor_index) == field_descriptor
        })
    }

    pub fn get_method<'a>(
        &'a self,
        invoke_type: InvokeType,
        declaring_class: &'a ClassFile,
        method_name: &str,
        method_descriptor: &str,
    ) -> Option<(&'a ClassFile, &'a MethodInfo)> {
        // from how I understand it, a special invokation is identical to a virtual one,
        // except the class at the bottom of the inheritance tree is explicity defined
        // and not assumed from the type of the instance being invoked on
        // thus, this method interprets special and virual invocations as the same,
        // since callers of this function must explicity define the bottom of the inheritance tree
        // with the invoke_type parameter

        match invoke_type {
            // InvokeType::Virtual => self.get_special_method(declaring_class, method_name, method_descriptor),
            InvokeType::Static => self.get_static_method(declaring_class, method_name, method_descriptor),
            InvokeType::Special | InvokeType::Virtual => {
                self.get_virtual_method(declaring_class, method_name, method_descriptor)
            }
        }
    }

    pub fn get_virtual_method<'a>(
        &'a self,
        declaring_class: &'a ClassFile,
        method_name: &str,
        method_descriptor: &str,
    ) -> Option<(&'a ClassFile, &'a MethodInfo)> {
        self.get_direct_method(declaring_class, method_name, method_descriptor, false).or_else(|| {
            if declaring_class.super_class == 0 {
                None
            } else {
                let superclass_name = get_constant_string(&declaring_class.const_pool, declaring_class.super_class);
                let superclass = self.get_classpath_entry(superclass_name).expect("class not found");
                self.get_virtual_method(superclass, method_name, method_descriptor)
            }
        })
    }

    pub fn get_static_method<'a>(
        &'a self,
        declaring_class: &'a ClassFile,
        method_name: &str,
        method_descriptor: &str,
    ) -> Option<(&'a ClassFile, &'a MethodInfo)> {
        self.get_direct_method(declaring_class, method_name, method_descriptor, true).or_else(|| {
            if declaring_class.super_class == 0 {
                None
            } else {
                let superclass_name = get_constant_string(&declaring_class.const_pool, declaring_class.super_class);
                let superclass = self.get_classpath_entry(superclass_name).expect("class not found");
                self.get_static_method(superclass, method_name, method_descriptor)
            }
        })
    }

    fn get_direct_method<'a>(
        &'a self,
        declaring_class: &'a ClassFile,
        method_name: &str,
        method_descriptor: &str,
        is_static: bool,
    ) -> Option<(&'a ClassFile, &'a MethodInfo)> {
        declaring_class
            .methods
            .iter()
            .find(|method| {
                if method.access_flags.contains(MethodAccessFlags::STATIC) {
                    if !is_static {
                        return false;
                    }
                } else if is_static {
                    return false;
                }
                let current_name = get_constant_string(&declaring_class.const_pool, method.name_index);
                if current_name == method_name {
                    let current_descriptor = get_constant_string(&declaring_class.const_pool, method.descriptor_index);
                    if current_descriptor == method_descriptor {
                        return true;
                    }
                }

                false
            })
            .map(|method| (declaring_class, method))
    }
}

#[wasm_bindgen]
pub struct WebJvmClasspath {
    classpath: Classpath,
}

#[wasm_bindgen]
impl WebJvmClasspath {
    #[wasm_bindgen(constructor)]
    pub fn new() -> WebJvmClasspath {
        WebJvmClasspath {
            classpath: Classpath::new(),
        }
    }

    #[wasm_bindgen(method, js_class = "WebJvmClasspath", js_name = addNativeMethod)]
    pub fn add_native_method(&mut self, native_method: js_sys::Function) -> Result<(), JsValue> {
        if native_method.name() == "anonymous" {
            return Err("anonymous functions cannot be bound to the JNI".into());
        }

        self.classpath.add_native_method(Box::new(native_method));
        Ok(())
    }

    #[wasm_bindgen(method, js_class = "WebJvmClasspath", js_name = addClasspathEntry)]
    pub fn add_classpath_entry(&mut self, class_bytes: &[u8]) {
        self.classpath.add_classpath_entry(class_bytes);
    }

    #[wasm_bindgen(method, js_class = "WebJvmClasspath", js_name = addClasspathJar)]
    pub fn add_classpath_jar(&mut self, jar_bytes: &[u8]) {
        self.classpath.add_classpath_jar(jar_bytes);
    }
}

#[wasm_bindgen]
pub struct WebJvmRuntime {
    jvm: Jvm,
}

#[wasm_bindgen]
impl WebJvmRuntime {
    #[wasm_bindgen(constructor)]
    pub fn new(wc: WebJvmClasspath) -> WebJvmRuntime {
        let mut classpath = wc.classpath;
        native::initialize(&mut classpath);
        WebJvmRuntime {
            jvm: Jvm::new(classpath),
        }
    }

    #[wasm_bindgen(method, js_class = "WebJvmRuntime", js_name = getClassName)]
    pub fn get_class_name(&self, id: usize) -> String {
        let heap = self.jvm.heap.borrow();
        heap.loaded_classes[id].java_type.clone()
    }

    #[wasm_bindgen(method, js_class = "WebJvmRuntime", js_name = dumpClass)]
    pub fn dump_class(&self, name: &str) -> String {
        let heap = self.jvm.heap.borrow();
        format!("{:?}", heap.loaded_classes[heap.loaded_classes_lookup[name]])
    }

    #[wasm_bindgen(method, js_class = "WebJvmRuntime", js_name = getObject)]
    pub fn get_object(&self, id: usize) -> String {
        let heap = self.jvm.heap.borrow();
        let obj = &heap.object_heap_map.get(&id).unwrap();
        format!("{:?}", obj)
    }

    #[wasm_bindgen(method, js_class = "WebJvmRuntime", js_name = getString)]
    pub fn get_string(&self, id: usize) -> String {
        let env = JniEnv::empty(&self.jvm);
        env.get_string(id)
    }

    #[wasm_bindgen(method, js_class = "WebJvmRuntime", js_name = getStacktrace)]
    pub fn get_stacktrace(&self) {
        self.jvm.throw_npe();
    }

    #[wasm_bindgen(method, js_class = "WebJvmRuntime", js_name = executeMain)]
    pub fn execute_main(&mut self, class_name: &str) -> Result<(), JsValue> {
        let frame = {
            let main_class = self.jvm.classpath.get_classpath_entry(class_name).expect("main class not found");
            let main_method = self
                .jvm
                .classpath
                .get_static_method(main_class, "main", "([Ljava/lang/String;)V")
                .expect("main method not found")
                .1;
            self.jvm.create_stack_frame(main_class, main_method).unwrap()
        };

        exec::env::initialize(&mut self.jvm).unwrap();
        self.jvm.push_call_stack_frame(frame);
        self.jvm.executor.step_until_stack_depth(&self.jvm, 1).unwrap();

        Ok(())
    }
}

#[wasm_bindgen(start)]
pub fn main() {
    std::panic::set_hook(Box::new(console_error_panic_hook::hook));
}

#[cfg(test)]
mod tests {
    #[test]
    pub fn get_unimplemented_opcodes() {
        use crate::exec::interpreter::*;

        let mut total = 0;
        for i in 0..0xca {
            if INSTRUCTION_HANDLERS[i] as usize == empty_instruction_handler as usize {
                println!("Unhandled opcode: 0x{:x?}", i);
                total += 1;
            }
        }

        println!("Total unhandled opcodes: {}", total);
    }

    #[test]
    pub fn run_tests() {
        use crate::*;
        let mut cp = WebJvmClasspath::new();
        cp.add_classpath_jar(include_bytes!("../test/java/rt.jar"));
        cp.add_classpath_entry(include_bytes!("../test/java/MainTest.class"));

        println!("Executing JVM...");
        let mut rt = WebJvmRuntime::new(cp);
        rt.execute_main("MainTest").unwrap();
        println!("Finished executing!");
    }
}
