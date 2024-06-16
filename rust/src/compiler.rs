use std::io::{Read as _, Write as _};

use codegen::ir::BlockCall;
use cranelift::{
    frontend::{FunctionBuilder, FunctionBuilderContext},
    prelude::*,
    prelude::{AbiParam, Type},
};
use cranelift_jit::{JITBuilder, JITModule};
use cranelift_module::{default_libcall_names, Linkage, Module};

use crate::{
    instruction::Instruction,
    memory::{Arrays, Memory},
};

#[derive(Clone, Copy, Debug)]
#[repr(C, u32)]
#[allow(dead_code)] // Some discriminants are constructed in the JIT code.
pub enum ExecutionResult {
    Halt = 0,
    Panic { reason: PanicReason } = 1,
    Jump { id: u32, new_pc: u32 } = 2,
}

#[derive(Clone, Copy, Debug, strum::FromRepr)]
#[repr(u32)]
pub enum PanicReason {
    JumpOutOfRange = 1,
    ReachedEndOfProgram = 2,
    InvalidInstruction = 3,
}

pub struct Compiler {
    func_ctx: FunctionBuilderContext,
    module: JITModule,
}

fn alloc_array_impl(arrays_real: *mut Arrays, size: u32) -> u32 {
    let arrays = unsafe { &mut *arrays_real };
    arrays.insert(vec![0; size as usize]) as u32
}

fn free_array_impl(arrays_real: *mut Arrays, id: u32) {
    let arrays = unsafe { &mut *arrays_real };
    arrays.remove(id as usize);
}

fn get_arrays_ptr_impl(arrays_real: *mut Arrays) -> *mut *mut u32 {
    let arrays = unsafe { &mut *arrays_real };
    arrays.as_mut_ptr()
}

fn getc_impl() -> u32 {
    let mut buf = [0];
    let size = std::io::stdin().read(&mut buf).expect("read error");
    if size == 0 {
        !0
    } else {
        buf[0] as u32
    }
}

fn putc_impl(value: u32) {
    std::io::stdout()
        .write_all(&[value as u8])
        .expect("write error");
}

fn trace_impl(pc: u32, value: u32) {
    match Instruction::from_u32(value) {
        Some(inst) => eprintln!("> @ {:04x}: {:?}", pc, inst),
        None => eprintln!("> @ {:04x}: 0x{:08x}", pc, value),
    }
}

fn print_ptr_impl(p: *const ()) {
    eprintln!("> p=0x{:x}", p as usize);
}

impl Compiler {
    pub fn new() -> Self {
        let module = {
            let mut flag_builder = settings::builder();
            flag_builder.set("use_colocated_libcalls", "false").unwrap();
            flag_builder.set("is_pic", "false").unwrap();
            let isa_builder = cranelift_native::builder().unwrap();
            let isa = isa_builder
                .finish(settings::Flags::new(flag_builder))
                .unwrap();

            let mut jit_builder = JITBuilder::with_isa(isa, default_libcall_names());
            jit_builder.symbol("alloc_array", alloc_array_impl as _);
            jit_builder.symbol("free_array", free_array_impl as _);
            jit_builder.symbol("get_arrays_ptr", get_arrays_ptr_impl as _);
            jit_builder.symbol("getc", getc_impl as _);
            jit_builder.symbol("putc", putc_impl as _);
            jit_builder.symbol("trace", trace_impl as _);
            jit_builder.symbol("print_ptr", print_ptr_impl as _);
            JITModule::new(jit_builder)
        };

        Self {
            func_ctx: FunctionBuilderContext::new(),
            module,
        }
    }

    pub fn compile(&mut self, program: &[u32]) -> impl Fn(u32, &mut Memory) -> ExecutionResult {
        let platter = Type::int(32).unwrap();
        let pointer = self.module.target_config().pointer_type();

        // Declare the function signature.
        let mut ctx = self.module.make_context();
        ctx.func.signature.params = vec![
            AbiParam::new(platter), // pc
            AbiParam::new(pointer), // regs_ptr
            AbiParam::new(pointer), // arrays_ptr
            AbiParam::new(pointer), // arrays_real
            AbiParam::new(pointer), // result_ptr
        ];

        let mut builder = FunctionBuilder::new(&mut ctx.func, &mut self.func_ctx);

        // Declare external functions.
        let mut alloc_array_signature = self.module.make_signature();
        alloc_array_signature.params.push(AbiParam::new(pointer));
        alloc_array_signature.params.push(AbiParam::new(platter));
        alloc_array_signature.returns.push(AbiParam::new(platter));
        let alloc_array_id = self
            .module
            .declare_function("alloc_array", Linkage::Import, &alloc_array_signature)
            .unwrap();
        let alloc_array_ref = self
            .module
            .declare_func_in_func(alloc_array_id, builder.func);

        let mut free_array_signature = self.module.make_signature();
        free_array_signature.params.push(AbiParam::new(pointer));
        free_array_signature.params.push(AbiParam::new(platter));
        let free_array_id = self
            .module
            .declare_function("free_array", Linkage::Import, &free_array_signature)
            .unwrap();
        let free_array_ref = self
            .module
            .declare_func_in_func(free_array_id, builder.func);

        let mut get_arrays_ptr_signature = self.module.make_signature();
        get_arrays_ptr_signature.params.push(AbiParam::new(pointer));
        get_arrays_ptr_signature
            .returns
            .push(AbiParam::new(pointer));
        let get_arrays_ptr_id = self
            .module
            .declare_function("get_arrays_ptr", Linkage::Import, &get_arrays_ptr_signature)
            .unwrap();
        let get_arrays_ptr_ref = self
            .module
            .declare_func_in_func(get_arrays_ptr_id, builder.func);

        let mut getc_signature = self.module.make_signature();
        getc_signature.returns.push(AbiParam::new(platter));
        let getc_id = self
            .module
            .declare_function("getc", Linkage::Import, &getc_signature)
            .unwrap();
        let getc_ref = self.module.declare_func_in_func(getc_id, builder.func);

        let mut putc_signature = self.module.make_signature();
        putc_signature.params.push(AbiParam::new(platter));
        let putc_id = self
            .module
            .declare_function("putc", Linkage::Import, &putc_signature)
            .unwrap();
        let putc_ref = self.module.declare_func_in_func(putc_id, builder.func);

        let mut trace_signature = self.module.make_signature();
        trace_signature.params.push(AbiParam::new(platter));
        trace_signature.params.push(AbiParam::new(platter));
        let trace_id = self
            .module
            .declare_function("trace", Linkage::Import, &trace_signature)
            .unwrap();
        let trace_ref = self.module.declare_func_in_func(trace_id, builder.func);

        let mut print_ptr_signature = self.module.make_signature();
        print_ptr_signature.params.push(AbiParam::new(pointer));
        let print_ptr_id = self
            .module
            .declare_function("print_ptr", Linkage::Import, &print_ptr_signature)
            .unwrap();
        let print_ptr_ref = self.module.declare_func_in_func(print_ptr_id, builder.func);

        // Declare fundamental blocks.
        let entry_block = builder.create_block();
        let panic_block = builder.create_block();
        let jump_block = builder.create_block();
        let jump_default_block = builder.create_block();
        let sentinel_block = builder.create_block();

        // Create the entry block.
        builder.switch_to_block(entry_block);
        builder.seal_block(entry_block);
        builder.append_block_params_for_function_params(entry_block);
        let init_pc = builder.block_params(entry_block)[0];
        let regs_ptr = builder.block_params(entry_block)[1];
        let arrays_ptr = builder.block_params(entry_block)[2];
        let arrays_real = builder.block_params(entry_block)[3];
        let result_ptr = builder.block_params(entry_block)[4];
        let regs: Vec<Variable> = (0..8)
            .map(|i| {
                let var = Variable::new(i);
                let value =
                    builder
                        .ins()
                        .load(platter, MemFlags::trusted(), regs_ptr, (i * 4) as i32);
                builder.declare_var(var, platter);
                builder.def_var(var, value);
                var
            })
            .collect();
        let arrays_var = Variable::new(8);
        builder.declare_var(arrays_var, pointer);
        builder.def_var(arrays_var, arrays_ptr);
        builder.ins().jump(jump_block, &[init_pc]);

        // Create the panic block.
        builder.switch_to_block(panic_block);
        builder.append_block_param(panic_block, platter); // reason
        builder.append_block_param(panic_block, platter); // arg
        let reason = builder.block_params(panic_block)[0];
        let arg = builder.block_params(panic_block)[1];
        for (i, reg) in regs.iter().enumerate() {
            let value = builder.use_var(*reg);
            builder
                .ins()
                .store(MemFlags::trusted(), value, regs_ptr, (i * 4) as i32);
        }
        let code = builder.ins().iconst(platter, 1);
        builder
            .ins()
            .store(MemFlags::trusted(), code, result_ptr, 0);
        builder.ins().store(
            MemFlags::trusted(),
            reason,
            result_ptr,
            platter.bytes() as i32,
        );
        builder.ins().store(
            MemFlags::trusted(),
            arg,
            result_ptr,
            platter.bytes() as i32 * 2,
        );
        builder.ins().return_(&[]);

        // Prepare instruction blocks.
        let inst_blocks: Vec<(Block, u32)> = program
            .iter()
            .map(|inst| (builder.create_block(), *inst))
            .collect();

        // Create the jump block.
        builder.switch_to_block(jump_block);
        builder.append_block_param(jump_block, platter); // new_pc
        let new_pc = builder.block_params(jump_block)[0];
        let pool = &mut builder.func.dfg.value_lists;
        let jump_table_data = JumpTableData::new(
            BlockCall::new(jump_default_block, &[new_pc], pool),
            &inst_blocks
                .iter()
                .copied()
                .map(|(block, _)| BlockCall::new(block, &[], pool))
                .collect::<Vec<_>>(),
        );
        let jump_table = builder.create_jump_table(jump_table_data);
        builder.ins().br_table(new_pc, jump_table);

        // Create the jump default block.
        builder.switch_to_block(jump_default_block);
        builder.seal_block(jump_default_block);
        builder.append_block_param(jump_default_block, platter); // new_pc
        let reason = builder
            .ins()
            .iconst(platter, PanicReason::JumpOutOfRange as u32 as i64);
        builder.ins().jump(panic_block, &[reason, new_pc]);

        // Create the sentinel block.
        builder.switch_to_block(sentinel_block);
        let reason = builder
            .ins()
            .iconst(platter, PanicReason::ReachedEndOfProgram as u32 as i64);
        let arg = builder.ins().iconst(platter, 0);
        builder.ins().jump(panic_block, &[reason, arg]);

        // Create the instruction blocks.
        for (pc, (block, code)) in inst_blocks.iter().copied().enumerate() {
            let next_block = inst_blocks
                .get(pc + 1)
                .map(|(block, _)| *block)
                .unwrap_or(sentinel_block);
            builder.switch_to_block(block);
            builder.seal_block(block);

            // TODO: Disable tracing.
            // let pc_value = builder.ins().iconst(platter, pc as i64);
            // let code_value = builder.ins().iconst(platter, code as i64);
            // builder.ins().call(trace_ref, &[pc_value, code_value]);

            match code >> 28 {
                0 => {
                    let a = ((code >> 6) & 7) as usize;
                    let b = ((code >> 3) & 7) as usize;
                    let c = (code & 7) as usize;
                    let value = builder.use_var(regs[b]);
                    let cond = builder.use_var(regs[c]);
                    let then_block = builder.create_block();
                    builder.ins().brif(cond, then_block, &[], next_block, &[]);

                    builder.switch_to_block(then_block);
                    builder.seal_block(then_block);
                    builder.def_var(regs[a], value);
                    builder.ins().jump(next_block, &[]);
                }
                1 => {
                    let a = ((code >> 6) & 7) as usize;
                    let b = ((code >> 3) & 7) as usize;
                    let c = (code & 7) as usize;
                    let id = builder.use_var(regs[b]);
                    let offset = builder.use_var(regs[c]);
                    let id64 = builder.ins().uextend(pointer, id);
                    let offset64 = builder.ins().uextend(pointer, offset);
                    let arrays_ptr = builder.use_var(arrays_var);
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
                    builder.def_var(regs[a], value);
                    builder.ins().jump(next_block, &[]);
                }
                2 => {
                    let a = ((code >> 6) & 7) as usize;
                    let b = ((code >> 3) & 7) as usize;
                    let c = (code & 7) as usize;
                    let id = builder.use_var(regs[a]);
                    let offset = builder.use_var(regs[b]);
                    let value = builder.use_var(regs[c]);
                    let id64 = builder.ins().uextend(pointer, id);
                    let offset64 = builder.ins().uextend(pointer, offset);
                    let arrays_ptr = builder.use_var(arrays_var);
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
                    builder.ins().jump(next_block, &[]);
                }
                3 => {
                    let a = ((code >> 6) & 7) as usize;
                    let b = ((code >> 3) & 7) as usize;
                    let c = (code & 7) as usize;
                    let lhs = builder.use_var(regs[b]);
                    let rhs = builder.use_var(regs[c]);
                    let value = builder.ins().iadd(lhs, rhs);
                    builder.def_var(regs[a], value);
                    builder.ins().jump(next_block, &[]);
                }
                4 => {
                    let a = ((code >> 6) & 7) as usize;
                    let b = ((code >> 3) & 7) as usize;
                    let c = (code & 7) as usize;
                    let lhs = builder.use_var(regs[b]);
                    let rhs = builder.use_var(regs[c]);
                    let value = builder.ins().imul(lhs, rhs);
                    builder.def_var(regs[a], value);
                    builder.ins().jump(next_block, &[]);
                }
                5 => {
                    let a = ((code >> 6) & 7) as usize;
                    let b = ((code >> 3) & 7) as usize;
                    let c = (code & 7) as usize;
                    let lhs = builder.use_var(regs[b]);
                    let rhs = builder.use_var(regs[c]);
                    let value = builder.ins().udiv(lhs, rhs);
                    builder.def_var(regs[a], value);
                    builder.ins().jump(next_block, &[]);
                }
                6 => {
                    let a = ((code >> 6) & 7) as usize;
                    let b = ((code >> 3) & 7) as usize;
                    let c = (code & 7) as usize;
                    let lhs = builder.use_var(regs[b]);
                    let rhs = builder.use_var(regs[c]);
                    let and_value = builder.ins().band(lhs, rhs);
                    let nand_value = builder.ins().bnot(and_value);
                    builder.def_var(regs[a], nand_value);
                    builder.ins().jump(next_block, &[]);
                }
                7 => {
                    for (i, reg) in regs.iter().enumerate() {
                        let value = builder.use_var(*reg);
                        builder
                            .ins()
                            .store(MemFlags::trusted(), value, regs_ptr, (i * 4) as i32);
                    }
                    let code = builder.ins().iconst(platter, 0);
                    builder
                        .ins()
                        .store(MemFlags::trusted(), code, result_ptr, 0);
                    builder.ins().return_(&[]);
                }
                8 => {
                    let b = ((code >> 3) & 7) as usize;
                    let c = (code & 7) as usize;
                    let size = builder.use_var(regs[c]);
                    let inst = builder.ins().call(alloc_array_ref, &[arrays_real, size]);
                    let id = builder.inst_results(inst)[0];
                    let inst = builder.ins().call(get_arrays_ptr_ref, &[arrays_real]);
                    let new_arrays_ptr = builder.inst_results(inst)[0];
                    builder.def_var(regs[b], id);
                    builder.def_var(arrays_var, new_arrays_ptr);
                    builder.ins().jump(next_block, &[]);
                }
                9 => {
                    let c = (code & 7) as usize;
                    let id = builder.use_var(regs[c]);
                    builder.ins().call(free_array_ref, &[arrays_real, id]);
                    builder.ins().jump(next_block, &[]);
                }
                10 => {
                    let c = (code & 7) as usize;
                    let value = builder.use_var(regs[c]);
                    builder.ins().call(putc_ref, &[value]);
                    builder.ins().jump(next_block, &[]);
                }
                11 => {
                    let c = (code & 7) as usize;
                    let inst = builder.ins().call(getc_ref, &[]);
                    let id = builder.inst_results(inst)[0];
                    builder.def_var(regs[c], id);
                    builder.ins().jump(next_block, &[]);
                }
                12 => {
                    let b = ((code >> 3) & 7) as usize;
                    let c = (code & 7) as usize;
                    let id = builder.use_var(regs[b]);
                    let new_pc = builder.use_var(regs[c]);
                    let far_block = builder.create_block();
                    builder
                        .ins()
                        .brif(id, far_block, &[], jump_block, &[new_pc]);

                    builder.switch_to_block(far_block);
                    builder.seal_block(far_block);
                    for (i, reg) in regs.iter().enumerate() {
                        let value = builder.use_var(*reg);
                        builder
                            .ins()
                            .store(MemFlags::trusted(), value, regs_ptr, (i * 4) as i32);
                    }
                    let code = builder.ins().iconst(platter, 2);
                    builder
                        .ins()
                        .store(MemFlags::trusted(), code, result_ptr, 0);
                    builder.ins().store(
                        MemFlags::trusted(),
                        id,
                        result_ptr,
                        platter.bytes() as i32,
                    );
                    builder.ins().store(
                        MemFlags::trusted(),
                        new_pc,
                        result_ptr,
                        platter.bytes() as i32 * 2,
                    );
                    builder.ins().return_(&[]);
                }
                13 => {
                    let a = ((code >> 25) & 7) as usize;
                    let immediate = (code & 0x1ffffff) as i64;
                    let value = builder.ins().iconst(platter, immediate);
                    builder.def_var(regs[a], value);
                    builder.ins().jump(next_block, &[]);
                }
                _ => {
                    let reason = builder
                        .ins()
                        .iconst(platter, PanicReason::InvalidInstruction as u32 as i64);
                    let code = builder.ins().iconst(platter, code as i64);
                    builder.ins().jump(panic_block, &[reason, code]);
                }
            }
        }

        // Finalize the function.
        builder.seal_all_blocks();
        builder.finalize();

        let func_id = self
            .module
            .declare_function("main", Linkage::Export, &ctx.func.signature)
            .unwrap();
        self.module.define_function(func_id, &mut ctx).unwrap();
        self.module.clear_context(&mut ctx);
        self.module.finalize_definitions().unwrap();

        let jit_func_ptr = self.module.get_finalized_function(func_id);
        // TODO: Manage the lifetime of the JIT function.
        let jit_func: extern "C" fn(
            u32,
            *mut u32,
            *const *mut u32,
            *mut Arrays,
            *mut ExecutionResult,
        ) = unsafe { std::mem::transmute(jit_func_ptr) };

        // Create a Rust function convenient for calling the generated function.
        move |pc: u32, memory: &mut Memory| -> ExecutionResult {
            let mut result = ExecutionResult::Halt;
            jit_func(
                pc,
                memory.regs.as_mut_ptr(),
                memory.arrays.as_mut_ptr(),
                &mut memory.arrays,
                &mut result,
            );
            result
        }
    }
}
