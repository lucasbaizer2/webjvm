pub mod exec;
pub mod util;

use std::io::Cursor;

use classfile_parser::{
    method_info::{MethodAccessFlags, MethodInfo},
    *,
};
use exec::*;
use util::*;
use wasm_bindgen::prelude::*;

pub struct Classpath {
    class_files: Vec<ClassFile>,
}

impl Classpath {
    pub fn new() -> Classpath {
        Classpath {
            class_files: Vec::new(),
        }
    }

    pub fn add_classpath_entry(&mut self, class_bytes: &[u8]) {
        let cls = classfile_parser::parse_class_bytes(class_bytes).unwrap();
        log(get_constant_string(&cls.const_pool, cls.this_class));
        self.class_files.push(cls);
    }

    pub fn add_classpath_jar(&mut self, jar_bytes: &[u8]) {
        use std::io::prelude::*;
        use zip::*;

        let mut cursor = Cursor::new(jar_bytes);
        let mut zip = ZipArchive::new(&mut cursor).expect("invalid zip archive");
        for i in 0..zip.len() {
            let mut file = zip.by_index(i).expect("invalid zip file content");
            let mut bytes = Vec::with_capacity(file.size() as usize);
            file.read_to_end(&mut bytes).expect("error reading zip");

            self.add_classpath_entry(bytes.as_slice());
            // let cls = classfile_parser::parse_class_bytes(bytes.as_slice()).unwrap();
            // self.add_classpath_entry(cls);
        }
    }

    pub fn get_classpath_entry(&self, name: &str) -> Option<&ClassFile> {
        self.class_files
            .iter()
            .find(|c| get_constant_string(&c.const_pool, c.this_class) == name)
    }

    pub fn get_virtual_method<'a>(
        &self,
        declaring_class: &'a ClassFile,
        method_name: &str,
        method_descriptor: &str,
    ) -> Option<&'a MethodInfo> {
        declaring_class.methods.iter().find(|method| {
            if !method.access_flags.contains(MethodAccessFlags::STATIC) {
                let current_name =
                    get_constant_string(&declaring_class.const_pool, method.name_index);
                if current_name == method_name {
                    let current_descriptor =
                        get_constant_string(&declaring_class.const_pool, method.descriptor_index);
                    if current_descriptor == method_descriptor {
                        return true;
                    }
                }
            }

            false
        })
        /*.or_else(|| {
            if declaring_class.super_class == 0 {
                None
            } else {
                let superclass_name = get_constant_string(
                    &declaring_class.const_pool,
                    declaring_class.super_class,
                );
                let superclass = self
                    .get_classpath_entry(superclass_name)
                    .expect("class not found");
                self.get_virtual_method(superclass, method_name, method_descriptor)
            }
        })*/
    }

    pub fn get_main_method(&self) -> Option<(&ClassFile, &MethodInfo)> {
        let mut classes: Vec<&ClassFile> = self.class_files.iter().map(|x| x).collect();
        classes.reverse();

        for file in classes {
            match file.methods.iter().find(|m| {
                m.access_flags == MethodAccessFlags::PUBLIC | MethodAccessFlags::STATIC
                    && get_constant_string(&file.const_pool, m.name_index) == "main"
                    && get_constant_string(&file.const_pool, m.descriptor_index)
                        == "([Ljava/lang/String;)V"
            }) {
                Some(method) => return Some((file, method)),
                None => (),
            };
        }

        None
    }
}

#[wasm_bindgen]
pub struct WebJvmRuntime {
    classpath: Classpath,
}

#[wasm_bindgen]
impl WebJvmRuntime {
    #[wasm_bindgen(constructor)]
    pub fn new() -> WebJvmRuntime {
        WebJvmRuntime {
            classpath: Classpath::new(),
        }
    }

    #[wasm_bindgen(method, js_class = "WebJvmRuntime", js_name = addClasspathEntry)]
    pub fn add_classpath_entry(&mut self, class_bytes: &[u8]) {
        self.classpath.add_classpath_entry(class_bytes);
    }

    #[wasm_bindgen(method, js_class = "WebJvmRuntime", js_name = addClasspathJar)]
    pub fn add_classpath_jar(&mut self, jar_bytes: &[u8]) {
        self.classpath.add_classpath_jar(jar_bytes);
    }

    #[wasm_bindgen(method, js_class = "WebJvmRuntime", js_name = executeMain)]
    pub fn execute_main(self) -> Result<(), JsValue> {
        let mut executor = JvmExecutor::new(self.classpath);
        let frame = {
            let (main_class, main_method) = executor
                .classpath
                .get_main_method()
                .expect("no main method found on classpath");
            log(&format!(
                "Main method: {:?}",
                get_constant_string(&main_class.const_pool, main_class.this_class)
            ));

            JvmExecutor::create_stack_frame(main_class, main_method)
        };
        executor.push_call_stack_frame(frame);
        while !executor.is_stack_empty() {
            executor.step();
        }

        Ok(())
    }
}

#[wasm_bindgen(start)]
pub fn main() {
    std::panic::set_hook(Box::new(console_error_panic_hook::hook));
}
