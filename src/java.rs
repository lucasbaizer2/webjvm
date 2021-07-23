#[derive(Debug)]
pub struct MethodDescriptor {
    pub argument_types: Vec<String>,
    pub return_type: String,
}

impl MethodDescriptor {
    fn read_token(desc: &Vec<char>, mut offset: usize) -> Result<(String, usize), ()> {
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
        if chars[0] != '(' {
            return Err(());
        }

        let mut argument_types = Vec::new();
        let mut offset = 1;
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
