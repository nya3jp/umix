use std::{
    io::{Read, Write}, ops::{Index, IndexMut}, path::PathBuf
};

use anyhow::Result;
use clap::Parser;
use codegen::ir::{BlockCall, ValueListPool};
use cranelift::{prelude::*, frontend::{FunctionBuilder, FunctionBuilderContext}, prelude::{AbiParam, Type}};
use cranelift_jit::{JITBuilder, JITModule};
use cranelift_module::{default_libcall_names, Linkage, Module};

#[derive(Parser, Debug)]
struct Args {
    codex: PathBuf,
}

#[derive(Clone, Default, Debug)]
struct Arrays {
    arrays: Vec<Option<Vec<u32>>>,
    vacants: Vec<usize>,
}

impl Arrays {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn insert(&mut self, array: Vec<u32>) -> usize {
        match self.vacants.pop() {
            Some(id) => {
                self.arrays[id] = Some(array);
                id
            }
            None => {
                let id = self.arrays.len();
                self.arrays.push(Some(array));
                id
            }
        }
    }

    pub fn remove(&mut self, id: usize) {
        assert!(self.arrays[id].is_some());
        self.arrays[id] = None;
        self.vacants.push(id);
    }

    pub fn dup0(&mut self, id: usize) {
        if id == 0 {
            return;
        }
        self.arrays[0] = self.arrays[id].clone();
    }

    pub fn as_mut_ptr(&mut self) -> *const *mut u32 {
        self.arrays.as_mut_ptr() as *const *mut u32
    }
}

impl Index<usize> for Arrays {
    type Output = [u32];

    fn index(&self, id: usize) -> &Self::Output {
        self.arrays.get(id).unwrap().as_ref().unwrap()
    }
}

impl IndexMut<usize> for Arrays {
    fn index_mut(&mut self, id: usize) -> &mut Self::Output {
        self.arrays.get_mut(id).unwrap().as_mut().unwrap()
    }
}

#[derive(Clone, Debug)]
struct Memory {
    regs: [u32; 8],
    arrays: Arrays,
}

impl Memory {
    pub fn new(program: Vec<u32>) -> Self {
        let mut arrays = Arrays::new();
        let id = arrays.insert(program);
        assert_eq!(id, 0);
        Self {
            regs: [0; 8],
            arrays,
        }
    }
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
        eprintln!("### compiling...");
        let mut compiler = Compiler::new();
        let run = compiler.compile(&self.memory.arrays[0]);
        eprintln!("### compiled!");
        loop {
            eprintln!("### running from {}", pc);
            match run(pc, &mut self.memory) {
                ExecutionResult::Halt => break,
                ExecutionResult::Jump { id, new_pc } => {
                    self.memory.arrays.dup0(id as usize);
                    pc = new_pc;
                }
            }
        }
    }

    pub fn run_interpreter(&mut self, mut pc: usize) {
        let mut stdin = std::io::stdin().lock();
        let mut stdout = std::io::stdout().lock();
        loop {
            let instruction = self.memory.arrays[0][pc];
            //eprintln!("pc: {}, inst: 0x{:08x}", pc, instruction);
            match instruction >> 28 {
                0 => {
                    let a = (instruction >> 6) & 7;
                    let b = (instruction >> 3) & 7;
                    let c = instruction & 7;
                    if self.memory.regs[c as usize] != 0 {
                        self.memory.regs[a as usize] = self.memory.regs[b as usize];
                    }
                    pc += 1;
                }
                1 => {
                    let a = (instruction >> 6) & 7;
                    let b = (instruction >> 3) & 7;
                    let c = instruction & 7;
                    self.memory.regs[a as usize] =
                        self.memory.arrays[self.memory.regs[b as usize] as usize][self.memory.regs[c as usize] as usize];
                    pc += 1;
                }
                2 => {
                    let a = (instruction >> 6) & 7;
                    let b = (instruction >> 3) & 7;
                    let c = instruction & 7;
                    self.memory.arrays[self.memory.regs[a as usize] as usize][self.memory.regs[b as usize] as usize] =
                        self.memory.regs[c as usize];
                    pc += 1;
                }
                3 => {
                    let a = (instruction >> 6) & 7;
                    let b = (instruction >> 3) & 7;
                    let c = instruction & 7;
                    self.memory.regs[a as usize] =
                        self.memory.regs[b as usize].wrapping_add(self.memory.regs[c as usize]);
                    pc += 1;
                }
                4 => {
                    let a = (instruction >> 6) & 7;
                    let b = (instruction >> 3) & 7;
                    let c = instruction & 7;
                    self.memory.regs[a as usize] =
                        self.memory.regs[b as usize].wrapping_mul(self.memory.regs[c as usize]);
                    pc += 1;
                }
                5 => {
                    let a = (instruction >> 6) & 7;
                    let b = (instruction >> 3) & 7;
                    let c = instruction & 7;
                    self.memory.regs[a as usize] = self.memory.regs[b as usize] / self.memory.regs[c as usize];
                    pc += 1;
                }
                6 => {
                    let a = (instruction >> 6) & 7;
                    let b = (instruction >> 3) & 7;
                    let c = instruction & 7;
                    self.memory.regs[a as usize] = !(self.memory.regs[b as usize] & self.memory.regs[c as usize]);
                    pc += 1;
                }
                7 => break,
                8 => {
                    let b = (instruction >> 3) & 7;
                    let c = instruction & 7;
                    let size = self.memory.regs[c as usize] as usize;
                    let id = self.memory.arrays.insert(vec![0; size]);
                    self.memory.regs[b as usize] = id as u32;
                    pc += 1;
                }
                9 => {
                    let c = instruction & 7;
                    let id = self.memory.regs[c as usize];
                    self.memory.arrays.remove(id as usize);
                    pc += 1;
                }
                10 => {
                    let c = instruction & 7;
                    let value = self.memory.regs[c as usize];
                    stdout.write_all(&[value as u8]).expect("write error");
                    pc += 1;
                }
                11 => {
                    let c = instruction & 7;
                    stdout.flush().expect("flush error");
                    let mut buf = [0];
                    let size = stdin.read(&mut buf).expect("read error");
                    if size == 0 {
                        self.memory.regs[c as usize] = !0;
                    } else {
                        self.memory.regs[c as usize] = buf[0] as u32;
                    }
                    pc += 1;
                }
                12 => {
                    let b = (instruction >> 3) & 7;
                    let c = instruction & 7;
                    let id = self.memory.regs[b as usize];
                    if id != 0 {
                        self.memory.arrays.dup0(id as usize);
                    }
                    pc = self.memory.regs[c as usize] as usize;
                }
                13 => {
                    let a = (instruction >> 25) & 7;
                    let value = instruction & 0x1ffffff;
                    self.memory.regs[a as usize] = value;
                    pc += 1;
                }
                op => {
                    panic!("Unknown opcode {op}");
                }
            }
        }
    }
}

#[derive(Clone, Copy, Debug)]
enum ExecutionResult {
    Halt,
    Jump { id: u32, new_pc: u32 },
}

struct Compiler {
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
    std::io::stdout().write_all(&[value as u8]).expect("write error");
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
            jit_builder.symbol("getc", getc_impl as _);
            jit_builder.symbol("putc", putc_impl as _);
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
        ];
        ctx.func.signature.returns = vec![
            AbiParam::new(platter), // code
            AbiParam::new(platter), // array id
            AbiParam::new(platter), // new pc
        ];

        let mut builder = FunctionBuilder::new(&mut ctx.func, &mut self.func_ctx);

        // Declare external functions.
        let mut alloc_array_signature = self.module.make_signature();
        alloc_array_signature.params.push(AbiParam::new(pointer));
        alloc_array_signature.params.push(AbiParam::new(platter));
        alloc_array_signature.returns.push(AbiParam::new(platter));
        let alloc_array_id = self.module.declare_function("alloc_array", Linkage::Import, &alloc_array_signature).unwrap();
        let alloc_array_ref = self.module.declare_func_in_func(alloc_array_id, builder.func);

        let mut free_array_signature = self.module.make_signature();
        free_array_signature.params.push(AbiParam::new(pointer));
        free_array_signature.params.push(AbiParam::new(platter));
        let free_array_id = self.module.declare_function("free_array", Linkage::Import, &free_array_signature).unwrap();
        let free_array_ref = self.module.declare_func_in_func(free_array_id, builder.func);

        let mut getc_signature = self.module.make_signature();
        getc_signature.returns.push(AbiParam::new(platter));
        let getc_id = self.module.declare_function("getc", Linkage::Import, &getc_signature).unwrap();
        let getc_ref = self.module.declare_func_in_func(getc_id, builder.func);

        let mut putc_signature = self.module.make_signature();
        putc_signature.params.push(AbiParam::new(platter));
        let putc_id = self.module.declare_function("putc", Linkage::Import, &putc_signature).unwrap();
        let putc_ref = self.module.declare_func_in_func(putc_id, builder.func);

        // Declare fundamental blocks.
        let entry_block = builder.create_block();
        let panic_block = builder.create_block();
        let jump_block = builder.create_block();

        // Create the entry block.
        builder.switch_to_block(entry_block);
        builder.seal_block(entry_block);
        builder.append_block_params_for_function_params(entry_block);
        let init_pc = builder.block_params(entry_block)[0];
        let regs_ptr = builder.block_params(entry_block)[1];
        let arrays_ptr = builder.block_params(entry_block)[2];
        let arrays_real = builder.block_params(entry_block)[3];
        let regs: Vec<Variable> = (0..8).map(|i| {
            let var = Variable::new(i);
            let value = builder.ins().load(platter, MemFlags::trusted(), regs_ptr, (i * 4) as i32);
            builder.declare_var(var, platter);
            builder.def_var(var, value);
            var
        }).collect();
        builder.ins().jump(jump_block, &[init_pc]);

        // Create the panic block.
        builder.switch_to_block(panic_block);
        builder.ins().trap(TrapCode::UnreachableCodeReached);

        // Prepare instruction blocks and the corresponding jump table.
        let inst_blocks: Vec<(Block, u32)> = program.iter().map(|inst| (builder.create_block(), *inst)).collect();

        let pool = &mut builder.func.dfg.value_lists;
        let jump_table_data = JumpTableData::new(
            BlockCall::new(panic_block, &[], pool),
            &inst_blocks.iter().copied().map(|(block, _)| BlockCall::new(block, &[], pool)).collect::<Vec<_>>(),
        );
        let jump_table = builder.create_jump_table(jump_table_data);

        // Create the jump block.
        builder.switch_to_block(jump_block);
        builder.append_block_param(jump_block, platter); // pc
        let new_pc = builder.block_params(jump_block)[0];
        builder.ins().br_table(new_pc, jump_table);

        // Create the instruction blocks.
        for (pc, (block, code)) in inst_blocks.iter().copied().enumerate() {
            let next_block = inst_blocks.get(pc + 1).map(|(block, _)| *block).unwrap_or(panic_block);
            builder.switch_to_block(block);
            builder.seal_block(block);

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
                    let then_block = builder.create_block();
                    builder.ins().brif(id, then_block, &[], panic_block, &[]);

                    builder.switch_to_block(then_block);
                    builder.seal_block(then_block);
                    let id64 = builder.ins().uextend(pointer, id);
                    let array_dist = builder.ins().imul_imm(id64, pointer.bytes() as i64);
                    let array_ptr = builder.ins().iadd(arrays_ptr, array_dist);
                    let array = builder.ins().load(pointer, MemFlags::trusted(), array_ptr, 0);
                    let offset64 = builder.ins().uextend(pointer, offset);
                    let value_dist = builder.ins().imul_imm(offset64, platter.bytes() as i64);
                    let value_ptr = builder.ins().iadd(array, value_dist);
                    let value = builder.ins().load(platter, MemFlags::trusted(), value_ptr, 0);
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
                    let then_block = builder.create_block();
                    builder.ins().brif(id, then_block, &[], panic_block, &[]);

                    builder.switch_to_block(then_block);
                    builder.seal_block(then_block);
                    let id64 = builder.ins().uextend(pointer, id);
                    let array_dist = builder.ins().imul_imm(id64, pointer.bytes() as i64);
                    let array_ptr = builder.ins().iadd(arrays_ptr, array_dist);
                    let array = builder.ins().load(pointer, MemFlags::trusted(), array_ptr, 0);
                    let offset64 = builder.ins().uextend(pointer, offset);
                    let value_dist = builder.ins().imul_imm(offset64, platter.bytes() as i64);
                    let value_ptr = builder.ins().iadd(array, value_dist);
                    builder.ins().store(MemFlags::trusted(), value, value_ptr, 0);
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
                        builder.ins().store(MemFlags::trusted(), value, regs_ptr, (i * 4) as i32);
                    }
                    let values = vec![
                        builder.ins().iconst(platter, 0),
                        builder.ins().iconst(platter, 0),
                        builder.ins().iconst(platter, 0),
                    ];
                    builder.ins().return_(&values);
                },
                8 => {
                    let b = ((code >> 3) & 7) as usize;
                    let c = (code & 7) as usize;
                    let size = builder.use_var(regs[c]);
                    let inst = builder.ins().call(alloc_array_ref, &[arrays_real, size]);
                    let id = builder.inst_results(inst)[0];
                    builder.def_var(regs[b], id);
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
                    builder.ins().brif(id, far_block, &[], jump_block, &[new_pc]);

                    builder.switch_to_block(far_block);
                    builder.seal_block(far_block);
                    for (i, reg) in regs.iter().enumerate() {
                        let value = builder.use_var(*reg);
                        builder.ins().store(MemFlags::trusted(), value, regs_ptr, (i * 4) as i32);
                    }
                    let values = vec![
                        builder.ins().iconst(platter, 1), // jump
                        id,
                        new_pc,
                    ];
                    builder.ins().return_(&values);
                }
                13 => {
                    let a = ((code >> 25) & 7) as usize;
                    let immediate = (code & 0x1ffffff) as i64;
                    let value = builder.ins().iconst(platter, immediate);
                    builder.def_var(regs[a], value);
                    builder.ins().jump(next_block, &[]);
                }
                _ => {
                    builder.ins().jump(panic_block, &[]);
                }
            }
        }

        // Finalize the function.
        builder.seal_all_blocks();
        builder.finalize();

        let func_id = self.module.declare_function("main", Linkage::Export, &ctx.func.signature).unwrap();
        self.module.define_function(func_id, &mut ctx).unwrap();
        self.module.clear_context(&mut ctx);
        self.module.finalize_definitions().unwrap();

        let jit_func_ptr = self.module.get_finalized_function(func_id);
        // TODO: Manage the lifetime of the JIT function.
        let jit_func: fn(u32, *mut u32, *const *mut u32, *mut Arrays) -> (u32, u32, u32) = unsafe { std::mem::transmute(jit_func_ptr) };

        // Create a Rust function convenient for calling the generated function.
        move |pc: u32, memory: &mut Memory| -> ExecutionResult {
            eprintln!("### calling jit function");
            let (code, id, new_pc) = jit_func(pc, memory.regs.as_mut_ptr(), memory.arrays.as_mut_ptr(), &mut memory.arrays);
            eprintln!("### returned from jit function");
            match code {
                0 => ExecutionResult::Halt,
                1 => ExecutionResult::Jump { id, new_pc },
                _ => panic!("Unknown return code {code}"),
            }
        }
    }
}

fn print_hello() {
    println!("Hello, world!");
}

fn hogehoge_compile() -> Result<fn() -> u32> {
    let mut func_ctx = FunctionBuilderContext::new();

    let mut module = {
        let mut builder = JITBuilder::new(default_libcall_names())?;
        builder.symbol("print_hello", print_hello as _);
        JITModule::new(builder)
    };

    let print_signature = module.make_signature();
    let print_id = module.declare_function("print_hello", Linkage::Import, &print_signature)?;

    let mut ctx = module.make_context();

    let platter = Type::int(32).unwrap();
    ctx.func.signature.returns.push(AbiParam::new(platter));

    let mut builder = FunctionBuilder::new(&mut ctx.func, &mut func_ctx);
    let entry_block = builder.create_block();
    builder.switch_to_block(entry_block);
    builder.seal_block(entry_block);
    let lhs = builder.ins().iconst(platter, 11);
    let rhs = builder.ins().iconst(platter, 22);
    let value = builder.ins().iadd(lhs, rhs);

    let print_ref = module.declare_func_in_func(print_id, builder.func);

    builder.ins().call(print_ref, &[]);
    builder.ins().return_(&[value]);
    builder.finalize();

    let func_id = module.declare_function("main", Linkage::Export, &ctx.func.signature)?;
    module.define_function(func_id, &mut ctx)?;
    module.clear_context(&mut ctx);
    module.finalize_definitions()?;
    let code_ptr = module.get_finalized_function(func_id);
    Ok(unsafe { std::mem::transmute(code_ptr) })
}

fn main() -> Result<()> {
    // eprintln!("answer = {}", compile()?());
    // return Ok(());

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
