use cranelift::prelude::*;
use cranelift_frontend::Position;
use cranelift_module::{default_libcall_names, Linkage, Module};
use cranelift_simplejit::{SimpleJITBackend, SimpleJITBuilder};
use std::mem;

fn main() {
    let mut module: Module<SimpleJITBackend> =
        Module::new(SimpleJITBuilder::new(default_libcall_names()));
    let mut ctx = module.make_context();
    let mut func_ctx = FunctionBuilderContext::new();
    let mut position = Position::default();

    let mut sig_a = module.make_signature();
    sig_a.params.push(AbiParam::new(types::I32));
    sig_a.returns.push(AbiParam::new(types::I32));

    let mut sig_b = module.make_signature();
    sig_b.returns.push(AbiParam::new(types::I32));

    let func_a = module
        .declare_function("a", Linkage::Local, &sig_a)
        .unwrap();
    let func_b = module
        .declare_function("b", Linkage::Local, &sig_b)
        .unwrap();

    ctx.func.signature = sig_a;
    ctx.func.name = ExternalName::user(0, func_a.as_u32());
    {
        let mut bcx: FunctionBuilder = FunctionBuilder::new(&mut ctx.func, &mut func_ctx, &mut position);
        let ebb = bcx.create_ebb();

        bcx.switch_to_block(ebb);
        bcx.append_ebb_params_for_function_params(ebb);
        let param = bcx.ebb_params(ebb)[0];
        let cst = bcx.ins().iconst(types::I32, 37);
        let add = bcx.ins().iadd(cst, param);
        bcx.ins().return_(&[add]);
        bcx.seal_all_blocks();
        bcx.finalize();
    }
    module.define_function(func_a, &mut ctx).unwrap();
    module.clear_context(&mut ctx);

    ctx.func.signature = sig_b;
    ctx.func.name = ExternalName::user(0, func_b.as_u32());
    {
        let mut bcx: FunctionBuilder = FunctionBuilder::new(&mut ctx.func, &mut func_ctx, &mut position);
        let ebb = bcx.create_ebb();

        bcx.switch_to_block(ebb);
        let local_func = module.declare_func_in_func(func_a, &mut bcx.func);
        let arg = bcx.ins().iconst(types::I32, 5);
        let call = bcx.ins().call(local_func, &[arg]);
        let value = {
            let results = bcx.inst_results(call);
            assert_eq!(results.len(), 1);
            results[0].clone()
        };
        bcx.ins().return_(&[value]);
        bcx.seal_all_blocks();
        bcx.finalize();
    }
    module.define_function(func_b, &mut ctx).unwrap();
    module.clear_context(&mut ctx);

    // Perform linking.
    module.finalize_definitions();

    // Get a raw pointer to the generated code.
    let code_b = module.get_finalized_function(func_b);

    // Cast it to a rust function pointer type.
    let ptr_b = unsafe { mem::transmute::<_, fn() -> u32>(code_b) };

    // Call it!
    let res = ptr_b();

    assert_eq!(res, 42);
}
