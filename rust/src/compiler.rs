use std::io::{Read as _, Write as _};

use codegen::{ir::FuncRef, Context};
use cranelift::{
    prelude::*,
    prelude::{AbiParam, Type},
};
use cranelift_jit::{JITBuilder, JITModule};
use cranelift_module::{default_libcall_names, Linkage, Module};

use crate::memory::Arrays;

fn alloc_array_impl(arrays_real: *mut Arrays, size: u32) -> u32 {
    let arrays: &mut Arrays = unsafe { &mut *arrays_real };
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

pub struct JitCompiler {
    module: JITModule,
    types: CommonTypes,
}

impl JitCompiler {
    pub fn new() -> Self {
        let module = {
            let mut flag_builder = settings::builder();
            flag_builder.set("opt_level", "speed").unwrap();
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
            JITModule::new(jit_builder)
        };

        let types = CommonTypes {
            platter: Type::int(32).unwrap(),
            pointer: module.target_config().pointer_type(),
        };

        Self { module, types }
    }

    pub fn types(&self) -> CommonTypes {
        self.types
    }

    pub fn module(&mut self) -> &mut JITModule {
        &mut self.module
    }

    pub fn declare_refs(&mut self, ctx: &mut Context) -> ExternalRefs {
        let CommonTypes { platter, pointer } = self.types;

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
            .declare_func_in_func(alloc_array_id, &mut ctx.func);

        let mut free_array_signature = self.module.make_signature();
        free_array_signature.params.push(AbiParam::new(pointer));
        free_array_signature.params.push(AbiParam::new(platter));
        let free_array_id = self
            .module
            .declare_function("free_array", Linkage::Import, &free_array_signature)
            .unwrap();
        let free_array_ref = self
            .module
            .declare_func_in_func(free_array_id, &mut ctx.func);

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
            .declare_func_in_func(get_arrays_ptr_id, &mut ctx.func);

        let mut getc_signature = self.module.make_signature();
        getc_signature.returns.push(AbiParam::new(platter));
        let getc_id = self
            .module
            .declare_function("getc", Linkage::Import, &getc_signature)
            .unwrap();
        let getc_ref = self.module.declare_func_in_func(getc_id, &mut ctx.func);

        let mut putc_signature = self.module.make_signature();
        putc_signature.params.push(AbiParam::new(platter));
        let putc_id = self
            .module
            .declare_function("putc", Linkage::Import, &putc_signature)
            .unwrap();
        let putc_ref = self.module.declare_func_in_func(putc_id, &mut ctx.func);

        ExternalRefs {
            alloc_array: alloc_array_ref,
            free_array: free_array_ref,
            get_arrays_ptr: get_arrays_ptr_ref,
            getc: getc_ref,
            putc: putc_ref,
        }
    }
}

pub struct ExternalRefs {
    pub alloc_array: FuncRef,
    pub free_array: FuncRef,
    pub get_arrays_ptr: FuncRef,
    pub getc: FuncRef,
    pub putc: FuncRef,
}

#[derive(Clone, Copy)]
pub struct CommonTypes {
    pub platter: Type,
    pub pointer: Type,
}
