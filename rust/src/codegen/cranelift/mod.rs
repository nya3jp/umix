use codegen::Context;
use cranelift::{
    frontend::{FunctionBuilder, FunctionBuilderContext},
    prelude::AbiParam,
    prelude::*,
};
use cranelift_jit::{JITBuilder, JITModule};
use cranelift_module::{default_libcall_names, Module as _};
use externals::declare_externals;

use crate::{
    codegen::cranelift::externals::{register_externals, ExternalRefs},
    memory::{Arrays, Memory},
};

use super::{CompiledFunc, CompiledFuncResult, RESULT_JUMP, RESULT_OK};

mod externals;

struct FunctionParams {
    pub arrays: Value,
}

struct FunctionVars {
    pub regs: Vec<Variable>,
    pub arrays_ptr: Variable,
}

struct FunctionBlocks {
    pub return_: Block,
}

pub struct CraneliftCodeGen {
    builder_ctx: FunctionBuilderContext,
    module: JITModule,
}

impl CraneliftCodeGen {
    pub fn new() -> Self {
        let mut flag_builder = settings::builder();
        flag_builder.set("opt_level", "speed").unwrap();
        let isa_builder = cranelift_native::builder().unwrap();
        let isa = isa_builder
            .finish(settings::Flags::new(flag_builder))
            .unwrap();

        let mut jit_builder = JITBuilder::with_isa(isa, default_libcall_names());
        register_externals(&mut jit_builder);
        let module = JITModule::new(jit_builder);

        Self {
            builder_ctx: FunctionBuilderContext::new(),
            module,
        }
    }

    pub fn start_function(&mut self) -> CraneliftCodeGenContext {
        let platter = Type::int(32).unwrap();
        let pointer = self.module.target_config().pointer_type();

        let mut ctx = Box::new(self.module.make_context());
        // ctx.set_disasm(true);
        let mut builder = FunctionBuilder::new(
            // SAFETY: ctx is essentially pinned.
            unsafe { std::mem::transmute(&mut ctx.func) },
            &mut self.builder_ctx,
        );
        let refs = declare_externals(&mut self.module, builder.func);

        builder.func.signature.params = vec![
            AbiParam::new(pointer), // regs
            AbiParam::new(pointer), // arrays
            AbiParam::new(pointer), // result
        ];

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

        let regs: Vec<Variable> = (0..8)
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

        let arrays_ptr = Variable::new(8);
        builder.declare_var(arrays_ptr, pointer);
        {
            let inst = builder.ins().call(refs.get_arrays_ptr, &[arrays_value]);
            let arrays_ptr_value = builder.inst_results(inst)[0];
            builder.def_var(arrays_ptr, arrays_ptr_value);
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
            for (i, reg_var) in regs.iter().enumerate() {
                let value = builder.use_var(*reg_var);
                builder.ins().store(
                    MemFlags::trusted(),
                    value,
                    regs_value,
                    (i as u32 * platter.bytes()) as i32,
                );
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

        CraneliftCodeGenContext {
            module: &mut self.module,
            ctx,
            builder,
            params: FunctionParams {
                arrays: arrays_value,
            },
            vars: FunctionVars { regs, arrays_ptr },
            blocks: FunctionBlocks {
                return_: return_block,
            },
            refs,
        }
    }
}

pub struct CraneliftCodeGenContext<'codegen> {
    module: &'codegen mut JITModule,
    ctx: Box<Context>,
    builder: FunctionBuilder<'codegen>,
    params: FunctionParams,
    vars: FunctionVars,
    blocks: FunctionBlocks,
    refs: ExternalRefs,
}

impl CraneliftCodeGenContext<'_> {
    pub fn conditional_move(&mut self, a: usize, b: usize, c: usize) {
        let cond = self.builder.use_var(self.vars.regs[c]);
        let then_block = self.builder.create_block();
        let next_block = self.builder.create_block();
        self.builder
            .ins()
            .brif(cond, then_block, &[], next_block, &[]);
        self.builder.seal_block(then_block);

        self.builder.switch_to_block(then_block);
        let value = self.builder.use_var(self.vars.regs[b]);
        self.builder.def_var(self.vars.regs[a], value);
        self.builder.ins().jump(next_block, &[]);
        self.builder.seal_block(next_block);

        self.builder.switch_to_block(next_block);
    }

    pub fn load(&mut self, a: usize, b: usize, c: usize) {
        let platter = Type::int(32).unwrap();
        let pointer = self.module.target_config().pointer_type();

        let id = self.builder.use_var(self.vars.regs[b]);
        let offset = self.builder.use_var(self.vars.regs[c]);
        let id64 = self.builder.ins().uextend(pointer, id);
        let offset64 = self.builder.ins().uextend(pointer, offset);
        let arrays_ptr = self.builder.use_var(self.vars.arrays_ptr);
        let array_dist = self.builder.ins().imul_imm(id64, pointer.bytes() as i64);
        let array_ptr = self.builder.ins().iadd(arrays_ptr, array_dist);
        let array = self
            .builder
            .ins()
            .load(pointer, MemFlags::trusted(), array_ptr, 0);
        let value_dist = self
            .builder
            .ins()
            .imul_imm(offset64, platter.bytes() as i64);
        let value_ptr = self.builder.ins().iadd(array, value_dist);
        let value = self
            .builder
            .ins()
            .load(platter, MemFlags::trusted(), value_ptr, 0);
        self.builder.def_var(self.vars.regs[a], value);
    }

    pub fn store(&mut self, a: usize, b: usize, c: usize) {
        let platter = Type::int(32).unwrap();
        let pointer = self.module.target_config().pointer_type();

        let id = self.builder.use_var(self.vars.regs[a]);
        let offset = self.builder.use_var(self.vars.regs[b]);
        let value = self.builder.use_var(self.vars.regs[c]);
        let id64 = self.builder.ins().uextend(pointer, id);
        let offset64 = self.builder.ins().uextend(pointer, offset);
        let arrays_ptr = self.builder.use_var(self.vars.arrays_ptr);
        let array_dist = self.builder.ins().imul_imm(id64, pointer.bytes() as i64);
        let array_ptr = self.builder.ins().iadd(arrays_ptr, array_dist);
        let array = self
            .builder
            .ins()
            .load(pointer, MemFlags::trusted(), array_ptr, 0);
        let value_dist = self
            .builder
            .ins()
            .imul_imm(offset64, platter.bytes() as i64);
        let value_ptr = self.builder.ins().iadd(array, value_dist);
        self.builder
            .ins()
            .store(MemFlags::trusted(), value, value_ptr, 0);
    }

    pub fn add(&mut self, a: usize, b: usize, c: usize) {
        let lhs = self.builder.use_var(self.vars.regs[b]);
        let rhs = self.builder.use_var(self.vars.regs[c]);
        let value = self.builder.ins().iadd(lhs, rhs);
        self.builder.def_var(self.vars.regs[a], value);
    }

    pub fn mul(&mut self, a: usize, b: usize, c: usize) {
        let lhs = self.builder.use_var(self.vars.regs[b]);
        let rhs = self.builder.use_var(self.vars.regs[c]);
        let value = self.builder.ins().imul(lhs, rhs);
        self.builder.def_var(self.vars.regs[a], value);
    }

    pub fn div(&mut self, a: usize, b: usize, c: usize) {
        let lhs = self.builder.use_var(self.vars.regs[b]);
        let rhs = self.builder.use_var(self.vars.regs[c]);
        let value = self.builder.ins().udiv(lhs, rhs);
        self.builder.def_var(self.vars.regs[a], value);
    }

    pub fn nand(&mut self, a: usize, b: usize, c: usize) {
        let lhs = self.builder.use_var(self.vars.regs[b]);
        let rhs = self.builder.use_var(self.vars.regs[c]);
        let and_value = self.builder.ins().band(lhs, rhs);
        let nand_value = self.builder.ins().bnot(and_value);
        self.builder.def_var(self.vars.regs[a], nand_value);
    }

    pub fn alloc_array(&mut self, b: usize, c: usize) {
        let size = self.builder.use_var(self.vars.regs[c]);
        let call = self
            .builder
            .ins()
            .call(self.refs.alloc_array, &[self.params.arrays, size]);
        let id = self.builder.inst_results(call)[0];
        let call = self
            .builder
            .ins()
            .call(self.refs.get_arrays_ptr, &[self.params.arrays]);
        let new_arrays_ptr_value = self.builder.inst_results(call)[0];
        self.builder.def_var(self.vars.regs[b], id);
        self.builder
            .def_var(self.vars.arrays_ptr, new_arrays_ptr_value);
    }

    pub fn free_array(&mut self, c: usize) {
        let id = self.builder.use_var(self.vars.regs[c]);
        self.builder
            .ins()
            .call(self.refs.free_array, &[self.params.arrays, id]);
    }

    pub fn putc(&mut self, c: usize) {
        let value = self.builder.use_var(self.vars.regs[c]);
        self.builder.ins().call(self.refs.putc, &[value]);
    }

    pub fn getc(&mut self, c: usize) {
        let call = self.builder.ins().call(self.refs.getc, &[]);
        let value = self.builder.inst_results(call)[0];
        self.builder.def_var(self.vars.regs[c], value);
    }

    pub fn jump(&mut self, b: usize, c: usize, expected_pc: usize) {
        let platter = Type::int(32).unwrap();

        let id = self.builder.use_var(self.vars.regs[b]);
        let new_pc = self.builder.use_var(self.vars.regs[c]);

        let far_block = self.builder.create_block();
        let near_block = self.builder.create_block();
        let miss_block = self.builder.create_block();
        let next_block = self.builder.create_block();

        self.builder.ins().brif(id, far_block, &[], near_block, &[]);
        self.builder.seal_block(far_block);
        self.builder.seal_block(near_block);

        self.builder.switch_to_block(far_block);
        let code = self.builder.ins().iconst(platter, RESULT_JUMP as i64);
        self.builder
            .ins()
            .jump(self.blocks.return_, &[code, id, new_pc]);

        self.builder.switch_to_block(near_block);
        let cond = self
            .builder
            .ins()
            .icmp_imm(IntCC::Equal, new_pc, expected_pc as i64);
        self.builder
            .ins()
            .brif(cond, next_block, &[], miss_block, &[]);
        self.builder.seal_block(miss_block);
        self.builder.seal_block(next_block);

        self.builder.switch_to_block(miss_block);
        let code = self.builder.ins().iconst(platter, RESULT_OK as i64);
        let zero = self.builder.ins().iconst(platter, 0);
        self.builder
            .ins()
            .jump(self.blocks.return_, &[code, new_pc, zero]);

        self.builder.switch_to_block(next_block);
    }

    pub fn immediate(&mut self, a: usize, imm: u32) {
        let platter = Type::int(32).unwrap();

        let value = self.builder.ins().iconst(platter, imm as i64);
        self.builder.def_var(self.vars.regs[a], value);
    }

    pub fn finalize(mut self, pc: usize) -> CompiledFunc {
        let platter = Type::int(32).unwrap();

        let code = self.builder.ins().iconst(platter, RESULT_OK as i64);
        let pc_value = self.builder.ins().iconst(platter, pc as i64);
        let zero = self.builder.ins().iconst(platter, 0);
        self.builder
            .ins()
            .jump(self.blocks.return_, &[code, pc_value, zero]);

        // Finalize the function.
        self.builder.seal_all_blocks();
        self.builder.finalize();

        let func_id = self
            .module
            .declare_anonymous_function(&self.ctx.func.signature)
            .unwrap();
        self.module.define_function(func_id, &mut self.ctx).unwrap();
        if let Some(vcode) = self.ctx.compiled_code().unwrap().vcode.as_ref() {
            eprintln!("{}", vcode);
        }
        self.module.finalize_definitions().unwrap();

        let jit_func_ptr = self.module.get_finalized_function(func_id);
        let jit_func: extern "C" fn(&mut [u32; 8], &mut Arrays, &mut CompiledFuncResult) =
            unsafe { std::mem::transmute(jit_func_ptr) };

        // Create a Rust function convenient for calling the generated function.
        Box::new(move |memory: &mut Memory| -> CompiledFuncResult {
            let mut result = CompiledFuncResult::Halt;
            jit_func(&mut memory.regs, &mut memory.arrays, &mut result);
            result
        })
    }
}
