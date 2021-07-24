use crate::{exec::JavaValue, Classpath, JniEnv, NativeMethod};

pub struct RustMethod {
    pub name: String,
    pub handler: Box<dyn Fn(&JniEnv) -> Option<JavaValue>>,
}

impl NativeMethod for RustMethod {
    fn invoke(&self, env: &JniEnv) -> Option<JavaValue> {
        (self.handler)(env)
    }

    fn get_name(&self) -> String {
        self.name.clone()
    }
}

macro_rules! register_jni {
    ( $cp:ident, $func:ident ) => {
        let method = crate::native::RustMethod {
            name: String::from(stringify!($func)),
            handler: Box::new($func),
        };
        $cp.add_native_method(Box::new(method));
    };
}

#[allow(non_snake_case)]
pub mod System;
#[allow(non_snake_case)]
pub mod Object;
#[allow(non_snake_case)]
pub mod Class;
#[allow(non_snake_case)]
pub mod Unsafe;

pub fn initialize(cp: &mut Classpath) {
    System::initialize(cp);
    Object::initialize(cp);
    Class::initialize(cp);
    Unsafe::initialize(cp);
}
