use std::{
    io::{Read, Write},
    path::PathBuf,
};

use anyhow::Result;
use clap::Parser;
use slab::Slab;

#[derive(Parser, Debug)]
struct Args {
    codex: PathBuf,
}

pub struct Machine {
    arrays: Slab<Vec<u32>>,
    regs: Vec<u32>,
    pc: usize,
}

impl Machine {
    pub fn new(program: Vec<u32>) -> Self {
        let mut arrays = Slab::new();
        let id = arrays.insert(program);
        assert_eq!(id, 0);
        Self {
            arrays,
            regs: vec![0; 8],
            pc: 0,
        }
    }

    pub fn run(&mut self) {
        let mut stdin = std::io::stdin().lock();
        let mut stdout = std::io::stdout().lock();
        loop {
            let instruction = self.arrays[0][self.pc];
            //eprintln!("pc: {}, inst: 0x{:08x}", self.pc, instruction);
            match instruction >> 28 {
                0 => {
                    let a = (instruction >> 6) & 7;
                    let b = (instruction >> 3) & 7;
                    let c = instruction & 7;
                    if self.regs[c as usize] != 0 {
                        self.regs[a as usize] = self.regs[b as usize];
                    }
                    self.pc += 1;
                }
                1 => {
                    let a = (instruction >> 6) & 7;
                    let b = (instruction >> 3) & 7;
                    let c = instruction & 7;
                    self.regs[a as usize] =
                        self.arrays[self.regs[b as usize] as usize][self.regs[c as usize] as usize];
                    self.pc += 1;
                }
                2 => {
                    let a = (instruction >> 6) & 7;
                    let b = (instruction >> 3) & 7;
                    let c = instruction & 7;
                    self.arrays[self.regs[a as usize] as usize][self.regs[b as usize] as usize] =
                        self.regs[c as usize];
                    self.pc += 1;
                }
                3 => {
                    let a = (instruction >> 6) & 7;
                    let b = (instruction >> 3) & 7;
                    let c = instruction & 7;
                    self.regs[a as usize] =
                        self.regs[b as usize].wrapping_add(self.regs[c as usize]);
                    self.pc += 1;
                }
                4 => {
                    let a = (instruction >> 6) & 7;
                    let b = (instruction >> 3) & 7;
                    let c = instruction & 7;
                    self.regs[a as usize] =
                        self.regs[b as usize].wrapping_mul(self.regs[c as usize]);
                    self.pc += 1;
                }
                5 => {
                    let a = (instruction >> 6) & 7;
                    let b = (instruction >> 3) & 7;
                    let c = instruction & 7;
                    self.regs[a as usize] = self.regs[b as usize] / self.regs[c as usize];
                    self.pc += 1;
                }
                6 => {
                    let a = (instruction >> 6) & 7;
                    let b = (instruction >> 3) & 7;
                    let c = instruction & 7;
                    self.regs[a as usize] = !(self.regs[b as usize] & self.regs[c as usize]);
                    self.pc += 1;
                }
                7 => break,
                8 => {
                    let b = (instruction >> 3) & 7;
                    let c = instruction & 7;
                    let size = self.regs[c as usize] as usize;
                    let id = self.arrays.insert(vec![0; size]);
                    self.regs[b as usize] = id as u32;
                    self.pc += 1;
                }
                9 => {
                    let c = instruction & 7;
                    let id = self.regs[c as usize];
                    self.arrays.remove(id as usize);
                    self.pc += 1;
                }
                10 => {
                    let c = instruction & 7;
                    let value = self.regs[c as usize];
                    stdout.write_all(&[value as u8]).expect("write error");
                    self.pc += 1;
                }
                11 => {
                    let c = instruction & 7;
                    stdout.flush().expect("flush error");
                    let mut buf = [0];
                    let size = stdin.read(&mut buf).expect("read error");
                    if size == 0 {
                        self.regs[c as usize] = !0;
                    } else {
                        self.regs[c as usize] = buf[0] as u32;
                    }
                    self.pc += 1;
                }
                12 => {
                    let b = (instruction >> 3) & 7;
                    let c = instruction & 7;
                    let id = self.regs[b as usize];
                    if id != 0 {
                        let (program, array) = self
                            .arrays
                            .get2_mut(0, id as usize)
                            .expect("invalid array id");
                        program.clone_from(array);
                    }
                    self.pc = self.regs[c as usize] as usize;
                }
                13 => {
                    let a = (instruction >> 25) & 7;
                    let value = instruction & 0x1ffffff;
                    self.regs[a as usize] = value;
                    self.pc += 1;
                }
                op => {
                    panic!("Unknown opcode {op}");
                }
            }
        }
    }
}

fn main() -> Result<()> {
    let args = Args::try_parse()?;
    let data = std::fs::read(args.codex)?;
    let program = data
        .chunks_exact(4)
        .map(|chunk| u32::from_be_bytes(chunk.try_into().unwrap()))
        .collect();

    let mut machine = Machine::new(program);
    machine.run();
    Ok(())
}
