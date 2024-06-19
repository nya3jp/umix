use std::path::{Path, PathBuf};

use anyhow::Result;
use clap::Parser as _;
use instruction::ParsedInstruction;

mod codegen;
mod instruction;
mod interpreter;
mod jit;
mod memory;

#[derive(clap::Parser, Debug)]
struct Args {
    #[clap(subcommand)]
    command: Command,
}

#[derive(clap::Subcommand, Debug)]
enum Command {
    Run(RunArgs),
    Dump(DumpArgs),
}

#[derive(Clone, Copy, Debug, clap::ValueEnum)]
enum RunMode {
    Jit,
    Interpreter,
}

#[derive(clap::Args, Debug)]
struct RunArgs {
    #[arg(long, default_value = "jit")]
    mode: RunMode,

    codex: PathBuf,
}

#[derive(clap::Args, Debug)]
struct DumpArgs {
    codex: PathBuf,
}

fn load_program(path: &Path) -> Result<Vec<u32>> {
    let data = std::fs::read(path)?;
    let program: Vec<u32> = data
        .chunks_exact(4)
        .map(|chunk| u32::from_be_bytes(chunk.try_into().unwrap()))
        .collect();
    Ok(program)
}

fn main() -> Result<()> {
    let args = Args::try_parse()?;
    match args.command {
        Command::Run(args) => {
            let program = load_program(&args.codex)?;
            match args.mode {
                RunMode::Jit => jit::run(program),
                RunMode::Interpreter => interpreter::run(program),
            }
        }
        Command::Dump(args) => {
            let program = load_program(&args.codex)?;
            for (pc, code) in program.into_iter().enumerate() {
                match ParsedInstruction::from_u32(code) {
                    Some(inst) => println!("{pc:08}: {inst:?}"),
                    None => println!("{pc:08}: [0x{code:08x}]"),
                }
            }
        }
    }

    Ok(())
}
