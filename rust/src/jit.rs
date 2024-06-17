use std::{
    collections::BTreeMap,
    io::{Read as _, Write as _},
};

use compiler::{CommonTypes, JitCompiler};
use cranelift::{
    frontend::{FunctionBuilder, FunctionBuilderContext},
    prelude::*,
    prelude::AbiParam,
};
use cranelift_module::Module as _;

use crate::{instruction::Instruction, memory::{Arrays, Memory}};

mod compiler;

type JitFunc = Box<dyn Fn(&mut Memory) -> JitFuncResult>;

#[derive(Clone, Copy, Debug)]
#[repr(C, u32)]
#[allow(dead_code)] // Some discriminants are constructed in the JIT code.
enum JitFuncResult {
    Halt = 0,
    Jump { id: u32, new_pc: u32 } = 1,
    Complete { pc: u32 } = 2,
    Miss { pc: u32 } = 3,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum StepResult {
    Halt,
    Next,
    Jump { id: u32, new_pc: usize },
}

fn execute_step(inst: Instruction, memory: &mut Memory) -> StepResult {
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
            if id != 0 {
                memory.arrays.dup0(id as usize);
            }
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

fn tracing_run(
    memory: &mut Memory,
    start_pc: usize,
    func_ctx: &mut FunctionBuilderContext,
    compiler: &mut JitCompiler,
) -> (Option<JitFunc>, usize) {
    let CommonTypes { platter, pointer } = compiler.types();

    let mut ctx = compiler.module().make_context();
    ctx.func.signature.params = vec![
        AbiParam::new(pointer), // regs
        AbiParam::new(pointer), // arrays
        AbiParam::new(pointer), // result
    ];
    let refs = compiler.declare_refs(&mut ctx);

    let mut builder = FunctionBuilder::new(&mut ctx.func, func_ctx);

    let entry_block = builder.create_block();
    let return_block = builder.create_block();
    let main_block = builder.create_block();

    // Create the entry block.
    builder.seal_block(entry_block);
    builder.switch_to_block(entry_block);
    builder.append_block_params_for_function_params(entry_block);
    let regs_value = builder.block_params(entry_block)[0];
    let arrays_value = builder.block_params(entry_block)[1];
    let result_value = builder.block_params(entry_block)[2];

    let reg_vars: Vec<Variable> = (0..8)
        .map(|i| {
            let var = Variable::new(i);
            builder.declare_var(var, platter);
            let value =
                builder
                    .ins()
                    .load(platter, MemFlags::trusted(), regs_value, (i * 4) as i32);
            builder.def_var(var, value);
            var
        })
        .collect();

    let arrays_ptr_var = Variable::new(8);
    builder.declare_var(arrays_ptr_var, pointer);
    {
        let inst = builder.ins().call(refs.get_arrays_ptr, &[arrays_value]);
        let arrays_ptr_value = builder.inst_results(inst)[0];
        builder.def_var(arrays_ptr_var, arrays_ptr_value);
    }
    builder.ins().jump(main_block, &[]);
    builder.seal_block(main_block);

    // Create the return block.
    {
        builder.switch_to_block(return_block);
        builder.append_block_param(return_block, platter); // code
        builder.append_block_param(return_block, platter); // arg1
        builder.append_block_param(return_block, platter); // arg2
        let code = builder.block_params(return_block)[0];
        let arg1 = builder.block_params(return_block)[1];
        let arg2 = builder.block_params(return_block)[2];

        // Save registers.
        for (i, reg_var) in reg_vars.iter().enumerate() {
            let value = builder.use_var(*reg_var);
            builder
                .ins()
                .store(MemFlags::trusted(), value, regs_value, (i as u32 * platter.bytes()) as i32);
        }

        // Save results.
        builder
            .ins()
            .store(MemFlags::trusted(), code, result_value, 0);
        builder.ins().store(
            MemFlags::trusted(),
            arg1,
            result_value,
            platter.bytes() as i32,
        );
        builder.ins().store(
            MemFlags::trusted(),
            arg2,
            result_value,
            platter.bytes() as i32 * 2,
        );

        builder.ins().return_(&[]);
    }

    // Start the main block.
    builder.switch_to_block(main_block);

    // Start tracing.
    let mut pc = start_pc;
    for _ in 0..1000 {
        let inst = Instruction::from_u32(memory.arrays[0][pc]);
        match inst.opcode() {
            0 => {
                let cond = builder.use_var(reg_vars[inst.c()]);
                let then_block = builder.create_block();
                let next_block = builder.create_block();
                builder.ins().brif(cond, then_block, &[], next_block, &[]);
                builder.seal_block(then_block);

                builder.switch_to_block(then_block);
                let value = builder.use_var(reg_vars[inst.b()]);
                builder.def_var(reg_vars[inst.a()], value);
                builder.ins().jump(next_block, &[]);
                builder.seal_block(next_block);

                builder.switch_to_block(next_block);
            }
            1 => {
                let id = builder.use_var(reg_vars[inst.b()]);
                let offset = builder.use_var(reg_vars[inst.c()]);
                let id64 = builder.ins().uextend(pointer, id);
                let offset64 = builder.ins().uextend(pointer, offset);
                let arrays_ptr = builder.use_var(arrays_ptr_var);
                let array_dist = builder.ins().imul_imm(id64, pointer.bytes() as i64);
                let array_ptr = builder.ins().iadd(arrays_ptr, array_dist);
                let array = builder
                    .ins()
                    .load(pointer, MemFlags::trusted(), array_ptr, 0);
                let value_dist = builder.ins().imul_imm(offset64, platter.bytes() as i64);
                let value_ptr = builder.ins().iadd(array, value_dist);
                let value = builder
                    .ins()
                    .load(platter, MemFlags::trusted(), value_ptr, 0);
                builder.def_var(reg_vars[inst.a()], value);
            }
            2 => {
                let id = builder.use_var(reg_vars[inst.a()]);
                let offset = builder.use_var(reg_vars[inst.b()]);
                let value = builder.use_var(reg_vars[inst.c()]);
                let id64 = builder.ins().uextend(pointer, id);
                let offset64 = builder.ins().uextend(pointer, offset);
                let arrays_ptr = builder.use_var(arrays_ptr_var);
                let array_dist = builder.ins().imul_imm(id64, pointer.bytes() as i64);
                let array_ptr = builder.ins().iadd(arrays_ptr, array_dist);
                let array = builder
                    .ins()
                    .load(pointer, MemFlags::trusted(), array_ptr, 0);
                let value_dist = builder.ins().imul_imm(offset64, platter.bytes() as i64);
                let value_ptr = builder.ins().iadd(array, value_dist);
                builder
                    .ins()
                    .store(MemFlags::trusted(), value, value_ptr, 0);
            }
            3 => {
                let lhs = builder.use_var(reg_vars[inst.b()]);
                let rhs = builder.use_var(reg_vars[inst.c()]);
                let value = builder.ins().iadd(lhs, rhs);
                builder.def_var(reg_vars[inst.a()], value);
            }
            4 => {
                let lhs = builder.use_var(reg_vars[inst.b()]);
                let rhs = builder.use_var(reg_vars[inst.c()]);
                let value = builder.ins().imul(lhs, rhs);
                builder.def_var(reg_vars[inst.a()], value);
            }
            5 => {
                let lhs = builder.use_var(reg_vars[inst.b()]);
                let rhs = builder.use_var(reg_vars[inst.c()]);
                let value = builder.ins().udiv(lhs, rhs);
                builder.def_var(reg_vars[inst.a()], value);
            }
            6 => {
                let lhs = builder.use_var(reg_vars[inst.b()]);
                let rhs = builder.use_var(reg_vars[inst.c()]);
                let and_value = builder.ins().band(lhs, rhs);
                let nand_value = builder.ins().bnot(and_value);
                builder.def_var(reg_vars[inst.a()], nand_value);
            }
            7 => return (None, pc),
            8 => {
                let size = builder.use_var(reg_vars[inst.c()]);
                let call = builder.ins().call(refs.alloc_array, &[arrays_value, size]);
                let id = builder.inst_results(call)[0];
                let call = builder.ins().call(refs.get_arrays_ptr, &[arrays_value]);
                let new_arrays_ptr_value = builder.inst_results(call)[0];
                builder.def_var(reg_vars[inst.b()], id);
                builder.def_var(arrays_ptr_var, new_arrays_ptr_value);
            }
            9 => {
                let id = builder.use_var(reg_vars[inst.c()]);
                builder.ins().call(refs.free_array, &[arrays_value, id]);
            }
            10 => {
                let value = builder.use_var(reg_vars[inst.c()]);
                builder.ins().call(refs.putc, &[value]);
            }
            11 => {
                let call = builder.ins().call(refs.getc, &[]);
                let value = builder.inst_results(call)[0];
                builder.def_var(reg_vars[inst.c()], value);
            }
            12 => {
                let id = builder.use_var(reg_vars[inst.b()]);
                let new_pc = builder.use_var(reg_vars[inst.c()]);

                let far_block = builder.create_block();
                let near_block = builder.create_block();
                let miss_block = builder.create_block();
                let next_block = builder.create_block();

                builder
                    .ins()
                    .brif(id, far_block, &[], near_block, &[]);
                builder.seal_block(far_block);
                builder.seal_block(near_block);

                builder.switch_to_block(far_block);
                let code = builder.ins().iconst(platter, 1); // JitFuncResult::Jump
                builder.ins().jump(return_block, &[code, id, new_pc]);

                builder.switch_to_block(near_block);
                let cond = builder.ins().icmp_imm(IntCC::Equal, new_pc, memory.regs[inst.c()] as i64);
                builder.ins().brif(cond, next_block, &[], miss_block, &[]);
                builder.seal_block(miss_block);
                builder.seal_block(next_block);

                builder.switch_to_block(miss_block);
                let code = builder.ins().iconst(platter, 3); // JitFuncResult::Miss
                let zero = builder.ins().iconst(platter, 0);
                builder.ins().jump(return_block, &[code, new_pc, zero]);

                builder.switch_to_block(next_block);
            }
            13 => {
                let value = builder.ins().iconst(platter, inst.imm_value() as i64);
                builder.def_var(reg_vars[inst.imm_a()], value);
            }
            _ => return (None, pc),
        }

        match execute_step(inst, memory) {
            StepResult::Halt => return (None, pc),
            StepResult::Next => pc += 1,
            StepResult::Jump { id, new_pc } => {
                if id != 0 {
                    return (None, pc);
                }
                pc = new_pc
            }
        }

        if pc == start_pc {
            break;
        }
    }

    let code = builder.ins().iconst(platter, 2); // JitFuncResult::Complete
    let pc_value = builder.ins().iconst(platter, pc as i64);
    let zero = builder.ins().iconst(platter, 0);
    builder.ins().jump(return_block, &[code, pc_value, zero]);

    // Finalize the function.
    builder.seal_all_blocks();
    builder.finalize();

    let func_id = compiler.module().declare_anonymous_function(&ctx.func.signature).unwrap();
    compiler.module().define_function(func_id, &mut ctx).unwrap();
    if let Some(vcode) = ctx.compiled_code().unwrap().vcode.as_ref() {
        eprintln!("{}", vcode);
    }
    compiler.module().finalize_definitions().unwrap();

    let jit_func_ptr = compiler.module().get_finalized_function(func_id);
    // TODO: Manage the lifetime of the JIT function.
    let jit_func: extern "C" fn(
        &mut [u32; 8],
        &mut Arrays,
        &mut JitFuncResult,
    ) = unsafe { std::mem::transmute(jit_func_ptr) };

    // Create a Rust function convenient for calling the generated function.
    let jit_func = Box::new(move |memory: &mut Memory| -> JitFuncResult {
        let mut result = JitFuncResult::Halt;
        jit_func(
            &mut memory.regs,
            &mut memory.arrays,
            &mut result,
        );
        result
    });
    (Some(jit_func), pc)
}

pub fn run(program: Vec<u32>) {
    let mut memory = Memory::new(program);
    let mut hits: BTreeMap<usize, usize> = BTreeMap::new();
    let mut func_ctx = FunctionBuilderContext::new();
    let mut compiler = JitCompiler::new();
    let mut jit_funcs: BTreeMap<usize, JitFunc> = BTreeMap::new();

    let mut pc = 0;
    loop {
        // Run the JIT function if it exists.
        if let Some(jit_func) = jit_funcs.get(&pc) {
            // eprintln!("# calling jit function: pc={}", pc);
            match jit_func(&mut memory) {
                JitFuncResult::Halt => break,
                JitFuncResult::Jump { id, new_pc } => {
                    if id != 0 {
                        hits.clear();
                        jit_funcs.clear();
                        memory.arrays.dup0(id as usize);
                    }
                    pc = new_pc as usize;
                }
                JitFuncResult::Complete { pc: new_pc } => {
                    // eprintln!("# jit function: complete");
                    pc = new_pc as usize;
                }
                JitFuncResult::Miss { pc: new_pc } => {
                    // eprintln!("# jit function: miss! (new_pc={})", new_pc);
                    pc = new_pc as usize;
                }
            }
            continue;
        }

        // Run the interpreter.
        let inst = Instruction::from_u32(memory.arrays[0][pc]);
        match execute_step(inst, &mut memory) {
            StepResult::Halt => break,
            StepResult::Next => pc += 1,
            StepResult::Jump { id, new_pc } => {
                let backward = new_pc < pc;
                pc = new_pc;
                if id != 0 {
                    hits.clear();
                    jit_funcs.clear();
                }
                if id == 0 && backward && !jit_funcs.contains_key(&pc) {
                    let count = hits.entry(pc).or_insert(0);
                    *count += 1;
                    if *count > 100 {
                        // eprintln!("# tracing: pc={}", pc);
                        let (compiled_func, new_pc) = tracing_run(&mut memory, pc, &mut func_ctx, &mut compiler);
                        if let Some(compiled_func) = compiled_func {
                            // eprintln!("# tracing success");
                            jit_funcs.insert(pc, compiled_func);
                        } else {
                            // eprintln!("# tracing failed!");
                        }
                        pc = new_pc;
                    }
                }
            }
        }
    }
}
