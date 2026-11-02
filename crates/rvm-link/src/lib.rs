//! `rvm-link` — linker estático inter-módulo (EXP-0, ADR-0018).
//!
//! A RIR EXP-0 representa **um** módulo: `call_static` só alcança funções do
//! próprio módulo. A ligação inter-módulo é feita **antes** da verificação, por
//! uma transformação pura que produz um único [`Module`] plano:
//!
//! 1. cada módulo pode declarar **funções externas** (sem corpo, criadas pelo
//!    `import` do formato textual);
//! 2. o linker resolve cada externa para a função concreta de mesmo nome em
//!    outro módulo e reescreve os `call_static` para IDs globais;
//! 3. o resultado não tem externas e é verificado por `rvm-verify` normalmente.
//!
//! A resolução é **estática e por nome único**: se duas funções concretas de
//! módulos diferentes tiverem o mesmo nome, a ligação é ambígua e falha. Não há
//! serialização de argumentos: a chamada inter-módulo vira uma chamada direta por
//! índice (H03). O linker é determinístico e sem efeitos de host.

#![forbid(unsafe_code)]
#![deny(missing_docs)]
#![cfg_attr(
    test,
    allow(
        clippy::unwrap_used,
        clippy::expect_used,
        clippy::panic,
        clippy::indexing_slicing
    )
)]

use std::collections::btree_map::Entry;
use std::collections::{BTreeMap, BTreeSet};

use rvm_ir::{FnId, Function, Module, Op};
use rvm_spec::{
    builtin_types, ErrorCode, Limits, Location, RvmError, BUILTIN_TYPE_COUNT, IR_VERSION,
};

/// Liga `modules` em um único módulo plano, com entrada `entry_function` do
/// módulo `entry_module`.
///
/// # Errors
///
/// Devolve [`ErrorCode::InvalidModule`] quando:
/// - a lista de módulos está vazia;
/// - há nomes de módulo duplicados;
/// - uma função externa não tem provedor (não resolvida) ou tem dois (ambígua);
/// - a função de entrada é externa ou inexistente, ou um `call_static` aponta
///   para ID inexistente.
pub fn link(
    modules: &[Module],
    entry_module: usize,
    entry_function: FnId,
    limits: &Limits,
) -> Result<Module, RvmError> {
    if modules.is_empty() {
        return Err(module_error("nenhum módulo para linkar".to_string()));
    }

    let mut module_names: BTreeSet<&str> = BTreeSet::new();
    for module in modules {
        if !module_names.insert(module.name.as_str()) {
            return Err(module_error(format!(
                "nome de módulo duplicado `{}`",
                module.name
            )));
        }
        if !module.structs.is_empty() || module.types.len() > BUILTIN_TYPE_COUNT {
            return Err(module_error(format!(
                "módulo `{}` declara tipos compostos/structs; ligação com tipos compostos ainda não é suportada (ADR-0034)",
                module.name
            )));
        }
    }

    // ID global de cada função concreta e mapa nome -> ID (`None` = ambíguo).
    let mut local_to_global: Vec<Vec<Option<FnId>>> = Vec::with_capacity(modules.len());
    let mut concrete_by_name: BTreeMap<String, Option<FnId>> = BTreeMap::new();
    let mut global_functions: Vec<Function> = Vec::new();
    for module in modules {
        let mut map: Vec<Option<FnId>> = vec![None; module.functions.len()];
        for (local, function) in module.functions.iter().enumerate() {
            if function.is_external() {
                continue;
            }
            let global = next_id(global_functions.len())?;
            if let Some(slot) = map.get_mut(local) {
                *slot = Some(global);
            }
            match concrete_by_name.entry(function.name.clone()) {
                Entry::Vacant(vacant) => {
                    vacant.insert(Some(global));
                }
                Entry::Occupied(mut occupied) => {
                    // Duas concretas com o mesmo nome: resolução ambígua.
                    occupied.insert(None);
                }
            }
            global_functions.push(clone_with_id(function, global));
        }
        local_to_global.push(map);
    }

    // Resolve cada externa para o ID global do provedor.
    let mut resolved: Vec<Vec<Option<FnId>>> = Vec::with_capacity(modules.len());
    for module in modules {
        let mut map: Vec<Option<FnId>> = vec![None; module.functions.len()];
        for (local, function) in module.functions.iter().enumerate() {
            if !function.is_external() {
                continue;
            }
            let target = match concrete_by_name.get(function.name.as_str()) {
                Some(Some(global)) => *global,
                Some(None) => {
                    return Err(module_error(format!(
                        "função externa `{}` é ambígua: dois módulos a definem",
                        function.name
                    )))
                }
                None => {
                    return Err(module_error(format!(
                        "função externa `{}` não tem provedor entre os módulos",
                        function.name
                    )))
                }
            };
            if let Some(slot) = map.get_mut(local) {
                *slot = Some(target);
            }
        }
        resolved.push(map);
    }

    // Reescreve os `call_static` de cada função concreta e renomeia para
    // garantir unicidade de nome no módulo plano. A ordem de `global_functions`
    // é a mesma da iteração (módulo, função), então o cursor acompanha.
    let mut used_names: BTreeSet<String> = BTreeSet::new();
    let mut cursor: usize = 0;
    for (module_index, module) in modules.iter().enumerate() {
        for function in &module.functions {
            if function.is_external() {
                continue;
            }
            let resolve = |callee: FnId| -> Result<FnId, RvmError> {
                resolve_callee(module_index, callee, module, &local_to_global, &resolved)
            };
            let target = global_functions
                .get_mut(cursor)
                .ok_or_else(|| module_error("contagem de funções divergente".to_string()))?;
            cursor += 1;
            rewrite_calls(target, &resolve)?;
            target.name = unique_name(&module.name, &function.name, &mut used_names, target.id);
        }
    }

    let entry_module_name = modules
        .get(entry_module)
        .ok_or_else(|| module_error(format!("módulo de entrada {entry_module} não existe")))?
        .name
        .clone();
    let entry = local_to_global
        .get(entry_module)
        .and_then(|map| map.get(usize::try_from(entry_function).unwrap_or(usize::MAX)))
        .copied()
        .flatten()
        .ok_or_else(|| {
            module_error(format!(
                "função de entrada {entry_function} do módulo `{entry_module_name}` não é concreta"
            ))
        })?;

    let linked = Module {
        name: format!("linked_{entry_module_name}"),
        version: IR_VERSION,
        entry,
        types: builtin_types(),
        structs: Vec::new(),
        functions: global_functions,
    };
    rvm_ir::structure::check(&linked, limits)?;
    Ok(linked)
}

/// Resolve um `callee` local para o ID global correspondente.
fn resolve_callee(
    module_index: usize,
    callee: FnId,
    module: &Module,
    local_to_global: &[Vec<Option<FnId>>],
    resolved: &[Vec<Option<FnId>>],
) -> Result<FnId, RvmError> {
    let index = usize::try_from(callee).unwrap_or(usize::MAX);
    let Some(function) = module.functions.get(index) else {
        return Err(module_error(format!(
            "chamada para função inexistente {callee} no módulo `{}`",
            module.name
        )));
    };
    let table = if function.is_external() {
        resolved
    } else {
        local_to_global
    };
    table
        .get(module_index)
        .and_then(|map| map.get(index))
        .copied()
        .flatten()
        .ok_or_else(|| {
            module_error(format!(
                "chamada não resolvida para `{}` no módulo `{}`",
                function.name, module.name
            ))
        })
}

fn rewrite_calls(
    function: &mut Function,
    resolve: &impl Fn(FnId) -> Result<FnId, RvmError>,
) -> Result<(), RvmError> {
    for block in &mut function.blocks {
        for instr in &mut block.instrs {
            if let Op::Call { callee, .. } = &mut instr.op {
                *callee = resolve(*callee)?;
            }
        }
    }
    Ok(())
}

fn unique_name(
    module_name: &str,
    function_name: &str,
    used: &mut BTreeSet<String>,
    global: FnId,
) -> String {
    let candidate = format!("{module_name}__{function_name}");
    if used.insert(candidate.clone()) {
        return candidate;
    }
    let fallback = format!("{candidate}__{global}");
    used.insert(fallback.clone());
    fallback
}

fn clone_with_id(function: &Function, id: FnId) -> Function {
    Function {
        id,
        name: function.name.clone(),
        params: function.params.clone(),
        ret: function.ret,
        entry: function.entry,
        blocks: function.blocks.clone(),
    }
}

fn next_id(count: usize) -> Result<FnId, RvmError> {
    FnId::try_from(count).map_err(|_| module_error("funções demais para linkar".to_string()))
}

fn module_error(detail: String) -> RvmError {
    RvmError::new(ErrorCode::InvalidModule, detail).at(Location::Module)
}
