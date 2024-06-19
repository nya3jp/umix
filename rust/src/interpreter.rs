use std::io::{Read as _, Write as _};

use crate::{instruction::Instruction, memory::Memory};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StepResult {
    Halt,
    Next,
    Jump { id: u32, new_pc: usize },
}

pub fn execute_step(inst: Instruction, memory: &mut Memory) -> StepResult {
    match inst.opcode() {
        0 => {
            if memory.regs[inst.c()] != 0 {
                memory.regs[inst.a()] = memory.regs[inst.b()];
            }
            StepResult::Next
        }
        1 => {
            memory.regs[inst.a()] =
                memory.arrays[memory.regs[inst.b()] as usize][memory.regs[inst.c()] as usize];
            StepResult::Next
        }
        2 => {
            memory.arrays[memory.regs[inst.a()] as usize][memory.regs[inst.b()] as usize] =
                memory.regs[inst.c()];
            StepResult::Next
        }
        3 => {
            memory.regs[inst.a()] = memory.regs[inst.b()].wrapping_add(memory.regs[inst.c()]);
            StepResult::Next
        }
        4 => {
            memory.regs[inst.a()] = memory.regs[inst.b()].wrapping_mul(memory.regs[inst.c()]);
            StepResult::Next
        }
        5 => {
            memory.regs[inst.a()] = memory.regs[inst.b()] / memory.regs[inst.c()];
            StepResult::Next
        }
        6 => {
            memory.regs[inst.a()] = !(memory.regs[inst.b()] & memory.regs[inst.c()]);
            StepResult::Next
        }
        7 => StepResult::Halt,
        8 => {
            let size = memory.regs[inst.c()] as usize;
            let id = memory.arrays.insert(vec![0; size]);
            memory.regs[inst.b()] = id as u32;
            StepResult::Next
        }
        9 => {
            let id = memory.regs[inst.c()];
            memory.arrays.remove(id as usize);
            StepResult::Next
        }
        10 => {
            let value = memory.regs[inst.c()];
            std::io::stdout()
                .write_all(&[value as u8])
                .expect("write error");
            StepResult::Next
        }
        11 => {
            std::io::stdout().flush().expect("flush error");
            let mut buf = [0];
            let size = std::io::stdin().read(&mut buf).expect("read error");
            if size == 0 {
                memory.regs[inst.c()] = !0;
            } else {
                memory.regs[inst.c()] = buf[0] as u32;
            }
            StepResult::Next
        }
        12 => {
            let id = memory.regs[inst.b()];
            let new_pc = memory.regs[inst.c()] as usize;
            StepResult::Jump { id, new_pc }
        }
        13 => {
            memory.regs[inst.imm_a()] = inst.imm_value();
            StepResult::Next
        }
        op => {
            panic!("unknown opcode {op}");
        }
    }
}

pub fn run(program: Vec<u32>) {
    let mut memory = Memory::new(program);
    let mut pc = 0;
    loop {
        let inst = Instruction::from_u32(memory.arrays[0][pc]);
        match execute_step(inst, &mut memory) {
            StepResult::Halt => return,
            StepResult::Next => pc += 1,
            StepResult::Jump { id, new_pc, .. } => {
                if id != 0 {
                    memory.arrays.dup0(id as usize);
                }
                pc = new_pc
            }
        }
    }
}
