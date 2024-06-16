use std::path::PathBuf;

use anyhow::Result;
use clap::Parser;
use compiler::{Compiler, ExecutionResult};
use interpreter::run_interpreter;
use memory::Memory;

mod compiler;
mod instruction;
mod interpreter;
mod memory;

#[derive(Parser, Debug)]
struct Args {
    codex: PathBuf,
}

pub struct Machine {
    memory: Memory,
}

impl Machine {
    pub fn new(program: Vec<u32>) -> Self {
        Self {
            memory: Memory::new(program),
        }
    }

    pub fn run_jit(&mut self, mut pc: u32) {
        loop {
            eprintln!("# compiling {} platters...", self.memory.arrays[0].len());
            let mut compiler = Compiler::new();
            let run = compiler.compile(&self.memory.arrays[0]);
            eprintln!("# compiled");
            match run(pc, &mut self.memory) {
                ExecutionResult::Halt => break,
                ExecutionResult::Jump { id, new_pc } => {
                    self.memory.arrays.dup0(id as usize);
                    pc = new_pc;
                }
                ExecutionResult::Panic { reason } => {
                    eprintln!("Panic: {:?}", reason);
                    break;
                }
            }
        }
    }

    pub fn run_interpreter(&mut self, pc: usize) {
        run_interpreter(pc, &mut self.memory)
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
    machine.run_jit(0);
    Ok(())
}
