use crate::JniEnv;
use crate::{exec::jvm::*, model::*, StackTraceElement};
use std::cell::RefCell;

mod instructions;

pub type InstructionHandler = fn(env: InstructionEnvironment) -> RuntimeResult<CallStackFrameState>;

pub fn empty_instruction_handler(env: InstructionEnvironment) -> RuntimeResult<CallStackFrameState> {
    Err(env.jvm.throw_exception("webjvm/lang/UnhandledInstructionError", Some(&format!("0x{:x?}", env.instruction))))
}

lazy_static! {
    pub static ref INSTRUCTION_HANDLERS: Vec<InstructionHandler> = {
        let mut handlers: Vec<InstructionHandler> = vec![empty_instruction_handler; 256];
        instructions::initialize(&mut handlers);
        handlers
    };
}

pub struct InstructionEnvironment<'a, 'b> {
    pub jvm: &'a Jvm,
    pub executor: &'b InstructionExecutor,
    pub instruction_address: usize,
    pub depth: usize,
    pub state: CallStackFrameState,
    pub instruction: u8,
}

pub struct InstructionExecutor {
    instruction_count: RefCell<u64>,
}

impl InstructionExecutor {
    pub fn new() -> InstructionExecutor {
        InstructionExecutor {
            instruction_count: RefCell::new(0),
        }
    }

    pub fn step_until_stack_depth(&self, jvm: &Jvm, depth: usize) -> RuntimeResult<()> {
        while {
            let csf = jvm.call_stack_frames.borrow();
            csf.len()
        } > depth
        {
            self.step(jvm)?;
        }

        Ok(())
    }

    fn get_native_step_env<'a>(&self, jvm: &'a Jvm, frame: &CallStackFrame) -> JniEnv<'a> {
        let csf = jvm.call_stack_frames.borrow();
        let mut stack_trace = Vec::with_capacity(csf.len());
        for frame in csf.iter() {
            stack_trace.push(StackTraceElement {
                class_name: frame.container_class.clone(),
                method: frame.container_method.clone(),
            });
        }
        JniEnv {
            jvm,
            container_class: frame.container_class.clone(),
            parameters: frame.state.lvt.clone(),
            stack_trace,
        }
    }

    pub fn step(&self, jvm: &Jvm) -> RuntimeResult<()> {
        match self.step_unchecked(jvm) {
            Ok(_) => Ok(()),
            Err(ex) => match ex {
                JavaThrowable::Handled(_) => Ok(()),
                JavaThrowable::Unhandled(_) => return Err(ex),
            },
        }
    }

    fn step_unchecked(&self, jvm: &Jvm) -> RuntimeResult<()> {
        let ic = { self.instruction_count.borrow().clone() };
        {
            self.instruction_count.replace(ic + 1);
        }

        let (env, instruction, depth) = {
            let is_native_frame = {
                let csf = jvm.call_stack_frames.borrow();
                let frame = csf.last().expect("no stack frame present");
                frame.is_native_frame
            };

            if is_native_frame {
                let return_value = {
                    let (env, jni_name) = {
                        let csf = jvm.call_stack_frames.borrow();
                        let frame = csf.last().expect("no stack frame present");
                        let env = self.get_native_step_env(jvm, &frame);

                        let method_name = &frame.container_method[0..frame.container_method.find('(').unwrap()];
                        let jni_name = format!("Java_{}_{}", frame.container_class.replace("/", "_"), method_name);

                        (env, jni_name)
                    };

                    let method = match jvm.classpath.get_native_method(&jni_name) {
                        Some(method) => method,
                        None => return Err(jvm.throw_exception("java/lang/UnsatisfiedLinkError", Some(&jni_name))),
                    };
                    method.invoke(&env)?
                };

                let mut csf = jvm.call_stack_frames.borrow_mut();
                csf.pop().unwrap();
                csf.last_mut().expect("stack underflow").state.return_stack_value = return_value;

                return Ok(());
            } else {
                let mut csf = jvm.call_stack_frames.borrow_mut();
                let depth = csf.len();
                let frame = csf.last_mut().expect("no stack frame present");
                let mut state = frame.state.clone();
                let instruction = frame.instructions[frame.state.instruction_offset];

                let instruction_address = frame.state.instruction_offset;
                state.instruction_offset += 1;

                let env = InstructionEnvironment {
                    jvm,
                    depth,
                    state,
                    instruction,
                    instruction_address,
                    executor: self,
                };

                (env, instruction, depth)
            }
        };

        let handler = INSTRUCTION_HANDLERS[instruction as usize];
        let new_state = handler(env)?;

        // match &insn.1 {
        //     Instruction::Athrow => {
        //         let ex = match pop!().as_object().expect("expecting object ref") {
        //             Some(obj) => obj,
        //             None => return Err(jvm.throw_npe()),
        //         };
        //         return Err(jvm.throw_exception_ref(ex));
        //     }
        //     x => return Err(jvm.throw_exception("webjvm/lang/UnhandledInstructionError", Some(&format!("{:?}", x)))),
        // }

        let mut csf = jvm.call_stack_frames.borrow_mut();
        if csf.len() == depth {
            csf.last_mut().unwrap().state = new_state;
        }

        Ok(())
    }
}
