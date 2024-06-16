use std::io::{Read as _, Write as _};

use crate::memory::Memory;

pub fn run_interpreter(mut pc: usize, memory: &mut Memory) {
    let mut stdin = std::io::stdin().lock();
    let mut stdout = std::io::stdout().lock();
    loop {
        let instruction = memory.arrays[0][pc];
        //eprintln!("pc: {}, inst: 0x{:08x}", pc, instruction);
        match instruction >> 28 {
            0 => {
                let a = (instruction >> 6) & 7;
                let b = (instruction >> 3) & 7;
                let c = instruction & 7;
                if memory.regs[c as usize] != 0 {
                    memory.regs[a as usize] = memory.regs[b as usize];
                }
                pc += 1;
            }
            1 => {
                let a = (instruction >> 6) & 7;
                let b = (instruction >> 3) & 7;
                let c = instruction & 7;
                memory.regs[a as usize] = memory.arrays[memory.regs[b as usize] as usize]
                    [memory.regs[c as usize] as usize];
                pc += 1;
            }
            2 => {
                let a = (instruction >> 6) & 7;
                let b = (instruction >> 3) & 7;
                let c = instruction & 7;
                memory.arrays[memory.regs[a as usize] as usize][memory.regs[b as usize] as usize] =
                    memory.regs[c as usize];
                pc += 1;
            }
            3 => {
                let a = (instruction >> 6) & 7;
                let b = (instruction >> 3) & 7;
                let c = instruction & 7;
                memory.regs[a as usize] =
                    memory.regs[b as usize].wrapping_add(memory.regs[c as usize]);
                pc += 1;
            }
            4 => {
                let a = (instruction >> 6) & 7;
                let b = (instruction >> 3) & 7;
                let c = instruction & 7;
                memory.regs[a as usize] =
                    memory.regs[b as usize].wrapping_mul(memory.regs[c as usize]);
                pc += 1;
            }
            5 => {
                let a = (instruction >> 6) & 7;
                let b = (instruction >> 3) & 7;
                let c = instruction & 7;
                memory.regs[a as usize] = memory.regs[b as usize] / memory.regs[c as usize];
                pc += 1;
            }
            6 => {
                let a = (instruction >> 6) & 7;
                let b = (instruction >> 3) & 7;
                let c = instruction & 7;
                memory.regs[a as usize] = !(memory.regs[b as usize] & memory.regs[c as usize]);
                pc += 1;
            }
            7 => break,
            8 => {
                let b = (instruction >> 3) & 7;
                let c = instruction & 7;
                let size = memory.regs[c as usize] as usize;
                let id = memory.arrays.insert(vec![0; size]);
                memory.regs[b as usize] = id as u32;
                pc += 1;
            }
            9 => {
                let c = instruction & 7;
                let id = memory.regs[c as usize];
                memory.arrays.remove(id as usize);
                pc += 1;
            }
            10 => {
                let c = instruction & 7;
                let value = memory.regs[c as usize];
                stdout.write_all(&[value as u8]).expect("write error");
                pc += 1;
            }
            11 => {
                let c = instruction & 7;
                stdout.flush().expect("flush error");
                let mut buf = [0];
                let size = stdin.read(&mut buf).expect("read error");
                if size == 0 {
                    memory.regs[c as usize] = !0;
                } else {
                    memory.regs[c as usize] = buf[0] as u32;
                }
                pc += 1;
            }
            12 => {
                let b = (instruction >> 3) & 7;
                let c = instruction & 7;
                let id = memory.regs[b as usize];
                if id != 0 {
                    memory.arrays.dup0(id as usize);
                }
                pc = memory.regs[c as usize] as usize;
            }
            13 => {
                let a = (instruction >> 25) & 7;
                let value = instruction & 0x1ffffff;
                memory.regs[a as usize] = value;
                pc += 1;
            }
            op => {
                panic!("Unknown opcode {op}");
            }
        }
    }
}
