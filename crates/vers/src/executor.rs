use wasmer::ValueType;

use {
    crate::version::Version,
    std::path::PathBuf,
    wasmer::{imports, Instance, Module, Store, Value},
    wasmer_compiler_cranelift::Cranelift,
    wasmer_engine_universal::Universal,
    wasmer_wasi::WasiState,
};

pub fn execute_wasm_get_assets<P: Into<PathBuf>>(mod_path: P) -> crate::Result<()> {
    let mod_path: PathBuf = mod_path.into();
    eyre::bail!("Not implemented")
}

pub fn execute_wasm_latest_version<P: Into<PathBuf>>(mod_path: P) -> crate::Result<Version> {
    let mod_path: PathBuf = mod_path.into();
    eyre::bail!("Not implemented")
}

pub fn execute_wasm_test<P: Into<PathBuf>>(mod_path: P) -> crate::Result<()> {
    let mod_path: PathBuf = mod_path.into();
    execute_wasm_module(&mod_path, "run", &["hello world"])?;
    Ok(())
}

pub fn execute_wasm_list_versions<P: Into<PathBuf>>(mod_path: P) -> crate::Result<Vec<Version>> {
    let mod_path: PathBuf = mod_path.into();
    match execute_wasm_module(&mod_path, "list", &[]) {
        Ok(wasm_mod_results) => {
            log::debug!("results: {:?}", wasm_mod_results);
            Ok(Vec::new())
        },
        Err(wasm_err) => {
            eyre::bail!(
                "Error executing WASM plugin {:?}. Error: {:?}",
                mod_path,
                wasm_err
            )
        },
    }
}

fn execute_wasm_module(
    mod_path: &'_ PathBuf,
    fn_name: &'_ str,
    fn_args: &[&str],
) -> crate::Result<Box<[Value]>> {
    let mod_path: PathBuf = mod_path.into();

    let wasm_mod = std::fs::read(&mod_path)?;

    let store = Store::new(&Universal::new(Cranelift::default()).engine());
    let module = Module::new(&store, &wasm_mod)?;

    let module_file_name = mod_path.file_stem().unwrap_or_default().to_str().unwrap();
    let mut wasi_env = WasiState::new(module_file_name).args(fn_args).finalize()?;

    let import_object = wasi_env.import_object(&module)?;
    let instance = Instance::new(&module, &import_object)?;

    let exported_fn = instance.exports.get_function(fn_name)?;
    let result = exported_fn.call(&[])?;

    Ok(result)
}
