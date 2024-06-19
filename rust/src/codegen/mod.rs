use crate::memory::Memory;

pub mod cranelift;

pub type CompiledFunc = Box<dyn Fn(&mut Memory) -> CompiledFuncResult>;

const RESULT_OK: u32 = 0;
const RESULT_JUMP: u32 = 1;
const RESULT_HALT: u32 = 2;

#[derive(Clone, Copy, Debug)]
#[repr(C, u32)]
#[allow(dead_code)] // Some discriminants are constructed in the JIT code.
pub enum CompiledFuncResult {
    Ok { pc: u32 } = RESULT_OK,
    Jump { id: u32, new_pc: u32 } = RESULT_JUMP,
    Halt = RESULT_HALT,
}
