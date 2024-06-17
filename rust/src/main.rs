use std::path::PathBuf;

use anyhow::Result;
use clap::Parser as _;
use instruction::ParsedInstruction;

mod compiler;
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

#[derive(clap::Args, Debug)]
struct RunArgs {
    #[arg(long)]
    jit: bool,

    codex: PathBuf,
}

#[derive(clap::Args, Debug)]
struct DumpArgs {
    codex: PathBuf,
}

fn main() -> Result<()> {
    let args = Args::try_parse()?;
    match args.command {
        Command::Run(args) => {
            let data = std::fs::read(args.codex)?;
            let program: Vec<u32> = data
                .chunks_exact(4)
                .map(|chunk| u32::from_be_bytes(chunk.try_into().unwrap()))
                .collect();
            if args.jit {
                jit::run(program);
            } else {
                interpreter::run(program);
            }
        }
        Command::Dump(args) => {
            let data = std::fs::read(args.codex)?;
            let program: Vec<u32> = data
                .chunks_exact(4)
                .map(|chunk| u32::from_be_bytes(chunk.try_into().unwrap()))
                .collect();
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
