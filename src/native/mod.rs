#![allow(non_snake_case)]

use crate::{
    model::{JavaValue, RuntimeResult},
    Classpath, JniEnv, NativeMethod,
};

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

mod java_lang_Class;
mod java_lang_ClassLoader;
mod java_lang_Double;
mod java_lang_Float;
mod java_lang_Object;
mod java_lang_Runtime;
mod java_lang_String;
mod java_lang_System;
mod java_lang_Thread;
mod java_lang_Throwable;

mod java_lang_reflect_Array;

mod java_io_FileDescriptor;
mod java_io_FileInputStream;
mod java_io_FileOutputStream;
mod java_io_UnixFileSystem;

mod java_security_AccessController;

mod java_util_concurrent_atomic_AtomicLong;

mod sun_misc_Signal;
mod sun_misc_URLClassPath;
mod sun_misc_Unsafe;
mod sun_misc_VM;

mod sun_reflect_NativeConstructorAccessorImpl;
mod sun_reflect_Reflection;

pub fn initialize(cp: &mut Classpath) {
    java_lang_Object::initialize(cp);
    java_lang_String::initialize(cp);
    java_lang_Class::initialize(cp);
    java_lang_ClassLoader::initialize(cp);
    java_lang_System::initialize(cp);
    java_lang_Float::initialize(cp);
    java_lang_Double::initialize(cp);
    java_lang_Thread::initialize(cp);
    java_lang_Throwable::initialize(cp);
    java_lang_Runtime::initialize(cp);

    java_lang_reflect_Array::initialize(cp);

    java_io_FileInputStream::initialize(cp);
    java_io_FileOutputStream::initialize(cp);
    java_io_FileDescriptor::initialize(cp);
    java_io_UnixFileSystem::initialize(cp);

    java_security_AccessController::initialize(cp);

    java_util_concurrent_atomic_AtomicLong::initialize(cp);

    sun_misc_Unsafe::initialize(cp);
    sun_misc_VM::initialize(cp);
    sun_misc_Signal::initialize(cp);
    sun_misc_URLClassPath::initialize(cp);

    sun_reflect_Reflection::initialize(cp);
    sun_reflect_NativeConstructorAccessorImpl::initialize(cp);
}
