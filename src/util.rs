use classfile_parser::constant_info::ConstantInfo;

pub fn log(str: &str) {
    #[allow(unused_unsafe)]
    unsafe {
        web_sys::console::log_1(&str.into());
    }
}

pub fn get_constant_string(const_pool: &Vec<ConstantInfo>, constant_index: u16) -> &String {
    match &const_pool[constant_index as usize - 1] {
        ConstantInfo::Utf8(str) => &str.utf8_string,
        ConstantInfo::Class(cls) => get_constant_string(const_pool, cls.name_index),
        ConstantInfo::String(str) => get_constant_string(const_pool, str.string_index),
        x => panic!("no string defined for constant info: {:?}", x),
    }
}

pub fn get_constant_name_and_type(
    const_pool: &Vec<ConstantInfo>,
    name_and_type_index: u16,
) -> (&String, &String) {
    match &const_pool[name_and_type_index as usize - 1] {
        ConstantInfo::NameAndType(nat) => (
            get_constant_string(const_pool, nat.name_index),
            get_constant_string(const_pool, nat.descriptor_index),
        ),
        x => panic!("bad name and type: {:?}", x),
    }
}
