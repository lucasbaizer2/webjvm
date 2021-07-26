use crate::{Classpath, JniEnv, NativeMethod, model::{JavaValue, RuntimeResult}};

pub struct RustMethod {
    pub name: String,
    pub handler: Box<dyn Fn(&JniEnv) -> RuntimeResult<Option<JavaValue>>>,
}

impl NativeMethod for RustMethod {
    fn invoke(&self, env: &JniEnv) -> RuntimeResult<Option<JavaValue>> {
        (self.handler)(env)
    }

    fn get_name(&self) -> String {
        self.name.clone()
    }
}

macro_rules! register_jni {
    ( $cp:ident, $( $func:ident ),* ) => {
        $(
            let method = crate::native::RustMethod {
                name: String::from(stringify!($func)),
                handler: Box::new($func),
            };
            $cp.add_native_method(Box::new(method));
        )*
    };
}

#[allow(non_snake_case)]
pub mod java_lang_Class;
#[allow(non_snake_case)]
pub mod java_lang_ClassLoader;
#[allow(non_snake_case)]
pub mod java_lang_Object;
#[allow(non_snake_case)]
pub mod java_lang_System;
#[allow(non_snake_case)]
pub mod java_lang_Float;
#[allow(non_snake_case)]
pub mod java_lang_Double;
#[allow(non_snake_case)]
pub mod java_lang_Thread;

#[allow(non_snake_case)]
pub mod java_io_FileInputStream;
#[allow(non_snake_case)]
pub mod java_io_FileOutputStream;
#[allow(non_snake_case)]
pub mod java_io_FileDescriptor;

#[allow(non_snake_case)]
pub mod java_security_AccessController;

#[allow(non_snake_case)]
pub mod sun_misc_Unsafe;
#[allow(non_snake_case)]
pub mod sun_misc_VM;

#[allow(non_snake_case)]
pub mod sun_reflect_Reflection;

pub fn initialize(cp: &mut Classpath) {
    java_lang_Object::initialize(cp);
    java_lang_Class::initialize(cp);
    java_lang_ClassLoader::initialize(cp);
    java_lang_System::initialize(cp);
    java_lang_Float::initialize(cp);
    java_lang_Double::initialize(cp);
    java_lang_Thread::initialize(cp);

    java_io_FileInputStream::initialize(cp);
    java_io_FileOutputStream::initialize(cp);
    java_io_FileDescriptor::initialize(cp);
    
    java_security_AccessController::initialize(cp);

    sun_misc_Unsafe::initialize(cp);
    sun_misc_VM::initialize(cp);
    
    sun_reflect_Reflection::initialize(cp);
}
