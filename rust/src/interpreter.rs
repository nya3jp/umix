use std::io::{Read as _, Write as _};

use crate::memory::Memory;

pub fn run(program: Vec<u32>) {
    let mut memory = Memory::new(program);
    let mut stdin = std::io::stdin().lock();
    let mut stdout = std::io::stdout().lock();
    let mut pc = 0;
    loop {
        let instruction = memory.arrays[0][pc];
        //eprintln!("pc: {}, inst: 0x{:08x}", pc, instruction);
        match instruction >> 28 {
            0 => {
                let a = ((instruction >> 6) & 7) as usize;
                let b = ((instruction >> 3) & 7) as usize;
                let c = (instruction & 7) as usize;
                if memory.regs[c] != 0 {
                    memory.regs[a] = memory.regs[b];
                }
                pc += 1;
            }
            1 => {
                let a = ((instruction >> 6) & 7) as usize;
                let b = ((instruction >> 3) & 7) as usize;
                let c = (instruction & 7) as usize;
                memory.regs[a] = memory.arrays[memory.regs[b] as usize]
                    [memory.regs[c] as usize];
                pc += 1;
            }
            2 => {
                let a = ((instruction >> 6) & 7) as usize;
                let b = ((instruction >> 3) & 7) as usize;
                let c = (instruction & 7) as usize;
                memory.arrays[memory.regs[a] as usize][memory.regs[b] as usize] =
                    memory.regs[c];
                pc += 1;
            }
            3 => {
                let a = ((instruction >> 6) & 7) as usize;
                let b = ((instruction >> 3) & 7) as usize;
                let c = (instruction & 7) as usize;
                memory.regs[a] =
                    memory.regs[b].wrapping_add(memory.regs[c]);
                pc += 1;
            }
            4 => {
                let a = ((instruction >> 6) & 7) as usize;
                let b = ((instruction >> 3) & 7) as usize;
                let c = (instruction & 7) as usize;
                memory.regs[a] =
                    memory.regs[b].wrapping_mul(memory.regs[c]);
                pc += 1;
            }
            5 => {
                let a = ((instruction >> 6) & 7) as usize;
                let b = ((instruction >> 3) & 7) as usize;
                let c = (instruction & 7) as usize;
                memory.regs[a] = memory.regs[b] / memory.regs[c];
                pc += 1;
            }
            6 => {
                let a = ((instruction >> 6) & 7) as usize;
                let b = ((instruction >> 3) & 7) as usize;
                let c = (instruction & 7) as usize;
                memory.regs[a] = !(memory.regs[b] & memory.regs[c]);
                pc += 1;
            }
            7 => break,
            8 => {
                let b = ((instruction >> 3) & 7) as usize;
                let c = (instruction & 7) as usize;
                let size = memory.regs[c] as usize;
                let id = memory.arrays.insert(vec![0; size]);
                memory.regs[b] = id as u32;
                pc += 1;
            }
            9 => {
                let c = (instruction & 7) as usize;
                let id = memory.regs[c];
                memory.arrays.remove(id as usize);
                pc += 1;
            }
            10 => {
                let c = (instruction & 7) as usize;
                let value = memory.regs[c];
                stdout.write_all(&[value as u8]).expect("write error");
                pc += 1;
            }
            11 => {
                let c = (instruction & 7) as usize;
                stdout.flush().expect("flush error");
                let mut buf = [0];
                let size = stdin.read(&mut buf).expect("read error");
                if size == 0 {
                    memory.regs[c] = !0;
                } else {
                    memory.regs[c] = buf[0] as u32;
                }
                pc += 1;
            }
            12 => {
                let b = ((instruction >> 3) & 7) as usize;
                let c = (instruction & 7) as usize;
                let id = memory.regs[b];
                let new_pc = memory.regs[c] as usize;
                if id != 0 {
                    memory.arrays.dup0(id as usize);
                }
                pc = new_pc;
            }
            13 => {
                let a = ((instruction >> 25) & 7) as usize;
                let value = instruction & 0x1ffffff;
                memory.regs[a] = value;
                pc += 1;
            }
            op => {
                panic!("Unknown opcode {op}");
            }
        }
    }
}
