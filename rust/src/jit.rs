use std::collections::HashMap;

use crate::{
    codegen::{cranelift::CraneliftCodeGen, CompiledFunc, CompiledFuncResult},
    instruction::Instruction,
    interpreter::{execute_step, StepResult},
    memory::Memory,
};

const JIT_MAX_INSTRUCTIONS: usize = 1000;
const JIT_HOT_SPOT_THRESHOLD: usize = 100;

fn tracing_run(
    memory: &mut Memory,
    start_pc: usize,
    codegen: &mut CraneliftCodeGen,
    compiled_funcs: &HashMap<usize, CompiledFunc>,
) -> (Option<CompiledFunc>, usize) {
    let mut ctx = codegen.start_function();

    // Start tracing.
    let mut pc = start_pc;
    let mut insts = 0;
    while insts < JIT_MAX_INSTRUCTIONS {
        let inst = Instruction::from_u32(memory.arrays[0][pc]);
        // eprintln!("{:08}: {:?}", pc, inst.parse().unwrap());
        match inst.opcode() {
            0 => ctx.conditional_move(inst.a(), inst.b(), inst.c()),
            1 => ctx.load(inst.a(), inst.b(), inst.c()),
            2 => ctx.store(inst.a(), inst.b(), inst.c()),
            3 => ctx.add(inst.a(), inst.b(), inst.c()),
            4 => ctx.mul(inst.a(), inst.b(), inst.c()),
            5 => ctx.div(inst.a(), inst.b(), inst.c()),
            6 => ctx.nand(inst.a(), inst.b(), inst.c()),
            7 => break,
            8 => ctx.alloc_array(inst.b(), inst.c()),
            9 => ctx.free_array(inst.c()),
            10 => ctx.putc(inst.c()),
            11 => ctx.getc(inst.c()),
            12 => ctx.jump(inst.b(), inst.c(), memory.regs[inst.c()] as usize),
            13 => ctx.immediate(inst.imm_a(), inst.imm_value()),
            _ => break,
        }

        match execute_step(inst, memory) {
            StepResult::Halt => break,
            StepResult::Next => pc += 1,
            StepResult::Jump { id, new_pc } => {
                if id != 0 {
                    break;
                }
                pc = new_pc
            }
        }

        insts += 1;

        if pc == start_pc || compiled_funcs.contains_key(&pc) {
            break;
        }
    }

    let compiled_func = ctx.finalize(pc);

    if insts <= 3 {
        (None, pc)
    } else {
        (Some(compiled_func), pc)
    }
}

pub fn run(program: Vec<u32>) {
    let mut memory = Memory::new(program);
    let mut compiled_funcs: HashMap<usize, CompiledFunc> = HashMap::new();
    let mut hits: HashMap<usize, usize> = HashMap::new();
    let mut codegen = CraneliftCodeGen::new();

    let mut pc = 0;
    loop {
        // Run the JIT function if it exists.
        while let Some(jit_func) = compiled_funcs.get(&pc) {
            match jit_func(&mut memory) {
                CompiledFuncResult::Ok { pc: new_pc } => {
                    pc = new_pc as usize;
                }
                CompiledFuncResult::Jump { id, new_pc } => {
                    if id != 0 {
                        hits.clear();
                        compiled_funcs.clear();
                        memory.arrays.dup0(id as usize);
                    }
                    pc = new_pc as usize;
                }
                CompiledFuncResult::Halt => return,
            }
        }

        // This is a good candidate for tracing.
        {
            let count = hits.entry(pc).or_insert(0);
            *count += 1;
            if *count == JIT_HOT_SPOT_THRESHOLD {
                let (compiled_func, new_pc) =
                    tracing_run(&mut memory, pc, &mut codegen, &compiled_funcs);
                if let Some(compiled_func) = compiled_func {
                    compiled_funcs.insert(pc, compiled_func);
                }
                pc = new_pc;
                // Try the newly compiled function.
                continue;
            }
        }

        // Run the interpreter.
        while !compiled_funcs.contains_key(&pc) {
            let inst = Instruction::from_u32(memory.arrays[0][pc]);
            match execute_step(inst, &mut memory) {
                StepResult::Halt => return,
                StepResult::Next => pc += 1,
                StepResult::Jump { id, new_pc } => {
                    let tracing_candidate = id != 0 || new_pc < pc;
                    if id != 0 {
                        memory.arrays.dup0(id as usize);
                        hits.clear();
                        compiled_funcs.clear();
                    }
                    pc = new_pc;
                    if tracing_candidate {
                        break;
                    }
                }
            }
        }
    }
}
