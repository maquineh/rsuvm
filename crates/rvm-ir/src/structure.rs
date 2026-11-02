//! Invariantes estruturais da RIR EXP-0.
//!
//! [`check`] valida o que precisa ser verdade **antes** de qualquer varredura de
//! tipos: versão do formato, densidade e unicidade de IDs, nomes, limites,
//! existência de alvos de desvio e de funções chamadas. Tipos de operandos,
//! dominância e compatibilidade de assinatura pertencem ao verificador
//! (`rvm-verify`) — não são checados aqui.

use std::collections::BTreeSet;

use rvm_spec::{ErrorCode, Limits, Location, RvmError, IR_VERSION};

use crate::{BranchTarget, Function, Module, Op, Terminator};

/// Resultado da checagem estrutural: `Ok(())` ou o primeiro diagnóstico.
///
/// A ordem das checagens é fixa para que o corpus observe sempre o mesmo código
/// para o mesmo módulo inválido.
pub fn check(module: &Module, limits: &Limits) -> Result<(), RvmError> {
    if module.version != IR_VERSION {
        return Err(invalid(
            ErrorCode::UnsupportedVersion,
            format!(
                "versão de módulo {} não suportada; esperada {}",
                module.version, IR_VERSION
            ),
            Location::Module,
        ));
    }
    check_name(&module.name, "nome do módulo", &Location::Module, limits)?;

    let function_count = module.functions.len();
    if function_count == 0 {
        return Err(invalid(
            ErrorCode::InvalidModule,
            "módulo não declara funções",
            Location::Module,
        ));
    }
    if function_count > limit(limits.max_functions) {
        return Err(invalid(
            ErrorCode::InvalidModule,
            format!(
                "limite de funções excedido: {function_count} > {}",
                limits.max_functions
            ),
            Location::Module,
        ));
    }
    if !index_in_range(module.entry, function_count) {
        return Err(invalid(
            ErrorCode::InvalidModule,
            format!("entry point {} não existe no módulo", module.entry),
            Location::Module,
        ));
    }
    if let Some(entry_function) = module
        .functions
        .get(usize::try_from(module.entry).unwrap_or(usize::MAX))
    {
        if entry_function.is_external() {
            return Err(invalid(
                ErrorCode::InvalidModule,
                format!(
                    "função de entrada `{}` não pode ser uma declaração externa (sem corpo)",
                    entry_function.name
                ),
                Location::Module,
            ));
        }
    }

    let mut function_names: BTreeSet<&str> = BTreeSet::new();
    for (index, function) in module.functions.iter().enumerate() {
        check_function(function, index, function_count, limits, &mut function_names)?;
    }
    Ok(())
}

fn check_function<'a>(
    function: &'a Function,
    index: usize,
    module_function_count: usize,
    limits: &Limits,
    function_names: &mut BTreeSet<&'a str>,
) -> Result<(), RvmError> {
    let location = Location::Function {
        function: function.name.clone(),
    };

    if !index_in_range(function.id, module_function_count) || function.id as usize != index {
        return Err(invalid(
            ErrorCode::InvalidModule,
            format!(
                "ID de função {} não é denso: esperado {index} na posição {index}",
                function.id
            ),
            location,
        ));
    }
    check_name(&function.name, "nome de função", &location, limits)?;
    if !function_names.insert(function.name.as_str()) {
        return Err(invalid(
            ErrorCode::InvalidModule,
            format!("nome de função duplicado `{}`", function.name),
            location,
        ));
    }
    if function.params.len() > limit(limits.max_params) {
        return Err(invalid(
            ErrorCode::InvalidModule,
            format!(
                "limite de parâmetros excedido na função `{}`: {} > {}",
                function.name,
                function.params.len(),
                limits.max_params
            ),
            location,
        ));
    }

    let mut named_values: BTreeSet<&str> = BTreeSet::new();
    for param in &function.params {
        check_name(&param.name, "nome de parâmetro", &location, limits)?;
        reject_reserved(&param.name, &location)?;
        if !named_values.insert(param.name.as_str()) {
            return Err(invalid(
                ErrorCode::InvalidModule,
                format!("nome de valor duplicado `{}`", param.name),
                location,
            ));
        }
    }

    let block_count = function.blocks.len();
    if block_count == 0 {
        // Declaração externa: sem corpo a validar. O linker (`rvm-link`) precisa
        // resolvê-la e o verificador rejeita módulos que ainda tenham externas
        // (ADR-0018). Parâmetros e nome já foram validados acima.
        return Ok(());
    }
    if block_count > limit(limits.max_blocks_per_function) {
        return Err(invalid(
            ErrorCode::InvalidModule,
            format!(
                "limite de blocos excedido na função `{}`: {block_count} > {}",
                function.name, limits.max_blocks_per_function
            ),
            location,
        ));
    }
    if !index_in_range(function.entry, block_count) {
        return Err(invalid(
            ErrorCode::InvalidModule,
            format!(
                "bloco de entrada {} não existe na função `{}`",
                function.entry, function.name
            ),
            location,
        ));
    }

    // Contagem de valores e unicidade de rótulos/nomes de parâmetro de bloco.
    let mut labels: BTreeSet<&str> = BTreeSet::new();
    let mut total_values: usize = function.params.len();
    for (block_index, block) in function.blocks.iter().enumerate() {
        let block_location = Location::Block {
            function: function.name.clone(),
            block: block.label.clone(),
        };
        if block.id as usize != block_index || !index_in_range(block.id, block_count) {
            return Err(invalid(
                ErrorCode::InvalidModule,
                format!(
                    "ID de bloco {} não é denso: esperado {block_index} na posição {block_index}",
                    block.id
                ),
                block_location,
            ));
        }
        check_name(&block.label, "rótulo de bloco", &block_location, limits)?;
        if !labels.insert(block.label.as_str()) {
            return Err(invalid(
                ErrorCode::InvalidModule,
                format!("rótulo de bloco duplicado `{}`", block.label),
                block_location,
            ));
        }
        if block.params.len() > limit(limits.max_block_params) {
            return Err(invalid(
                ErrorCode::InvalidModule,
                format!(
                    "limite de parâmetros de bloco excedido: {} > {}",
                    block.params.len(),
                    limits.max_block_params
                ),
                block_location,
            ));
        }
        if block.instrs.len() > limit(limits.max_instrs_per_block) {
            return Err(invalid(
                ErrorCode::InvalidModule,
                format!(
                    "limite de instruções por bloco excedido: {} > {}",
                    block.instrs.len(),
                    limits.max_instrs_per_block
                ),
                block_location,
            ));
        }
        for param in &block.params {
            check_name(
                &param.name,
                "nome de parâmetro de bloco",
                &block_location,
                limits,
            )?;
            reject_reserved(&param.name, &block_location)?;
            if !named_values.insert(param.name.as_str()) {
                return Err(invalid(
                    ErrorCode::InvalidModule,
                    format!("nome de valor duplicado `{}`", param.name),
                    block_location,
                ));
            }
        }
        total_values = total_values
            .checked_add(block.params.len())
            .and_then(|total| total.checked_add(block.instrs.len()))
            .ok_or_else(|| {
                invalid(
                    ErrorCode::InvalidModule,
                    "contagem total de valores transbordou",
                    block_location.clone(),
                )
            })?;
    }

    if let Some(entry_block) = function
        .blocks
        .get(usize::try_from(function.entry).unwrap_or(usize::MAX))
    {
        if !entry_block.params.is_empty() {
            return Err(invalid(
                ErrorCode::InvalidModule,
                format!(
                    "bloco de entrada `{}` não pode declarar parâmetros",
                    entry_block.label
                ),
                Location::Block {
                    function: function.name.clone(),
                    block: entry_block.label.clone(),
                },
            ));
        }
    }

    if total_values > limit(limits.max_values_per_function) {
        return Err(invalid(
            ErrorCode::InvalidModule,
            format!(
                "limite de valores por função excedido: {total_values} > {}",
                limits.max_values_per_function
            ),
            location,
        ));
    }

    // Alocação limitada pelo teto acima: `seen` tem no máximo `max_values` bits.
    let mut seen = vec![false; total_values];
    for slot in seen.iter_mut().take(function.params.len()) {
        *slot = true;
    }

    for block in &function.blocks {
        let block_location = Location::Block {
            function: function.name.clone(),
            block: block.label.clone(),
        };
        for param in &block.params {
            mark_value(&mut seen, param.value, &block_location)?;
        }
        for (instr_index, instr) in block.instrs.iter().enumerate() {
            let instr_location = Location::Instruction {
                function: function.name.clone(),
                block: block.label.clone(),
                index: u32::try_from(instr_index).unwrap_or(u32::MAX),
            };
            mark_value(&mut seen, instr.def, &instr_location)?;
            if let Op::ConstStr(text) = &instr.op {
                if text.len() > limit(limits.max_string_bytes) {
                    return Err(invalid(
                        ErrorCode::InvalidModule,
                        format!(
                            "literal string com {} bytes excede o limite de {}",
                            text.len(),
                            limits.max_string_bytes
                        ),
                        instr_location.clone(),
                    ));
                }
            }
            if let Op::ArrayLiteral(items) = &instr.op {
                if items.is_empty() || items.len() > limit(limits.max_array_len) {
                    return Err(invalid(
                        ErrorCode::InvalidModule,
                        format!(
                            "array literal com {} elemento(s); esperado 1..={}",
                            items.len(),
                            limits.max_array_len
                        ),
                        instr_location.clone(),
                    ));
                }
            }
            for operand in instr.op.operand_ids() {
                check_value_ref(&seen, operand, &instr_location)?;
            }
            if let Op::Call { callee, args } = &instr.op {
                if args.len() > limit(limits.max_call_args) {
                    return Err(invalid(
                        ErrorCode::InvalidModule,
                        format!(
                            "limite de argumentos de chamada excedido: {} > {}",
                            args.len(),
                            limits.max_call_args
                        ),
                        instr_location.clone(),
                    ));
                }
                if !index_in_range(*callee, module_function_count) {
                    return Err(invalid(
                        ErrorCode::InvalidModule,
                        format!("função chamada {} não existe no módulo", callee),
                        instr_location,
                    ));
                }
            }
        }
        check_terminator(&block.term, &seen, block_count, limits, &block_location)?;
    }

    Ok(())
}

fn check_terminator(
    term: &Terminator,
    seen: &[bool],
    block_count: usize,
    limits: &Limits,
    location: &Location,
) -> Result<(), RvmError> {
    match term {
        Terminator::Return(None) => Ok(()),
        Terminator::Return(Some(value)) => check_value_ref(seen, *value, location),
        Terminator::Br(target) => check_target(target, seen, block_count, limits, location),
        Terminator::CondBr {
            cond,
            then_target,
            else_target,
        } => {
            check_value_ref(seen, *cond, location)?;
            check_target(then_target, seen, block_count, limits, location)?;
            check_target(else_target, seen, block_count, limits, location)
        }
    }
}

fn check_target(
    target: &BranchTarget,
    seen: &[bool],
    block_count: usize,
    limits: &Limits,
    location: &Location,
) -> Result<(), RvmError> {
    if !index_in_range(target.target, block_count) {
        return Err(invalid(
            ErrorCode::InvalidControlFlow,
            format!("bloco alvo {} não existe na função", target.target),
            location.clone(),
        ));
    }
    if target.args.len() > limit(limits.max_call_args) {
        return Err(invalid(
            ErrorCode::InvalidModule,
            format!(
                "limite de argumentos de desvio excedido: {} > {}",
                target.args.len(),
                limits.max_call_args
            ),
            location.clone(),
        ));
    }
    for arg in &target.args {
        check_value_ref(seen, *arg, location)?;
    }
    Ok(())
}

fn mark_value(seen: &mut [bool], value: u32, location: &Location) -> Result<(), RvmError> {
    let index = usize::try_from(value).map_err(|_| {
        invalid(
            ErrorCode::InvalidModule,
            format!("ValueId {value} não representável nesta plataforma"),
            location.clone(),
        )
    })?;
    match seen.get_mut(index) {
        None => Err(invalid(
            ErrorCode::InvalidModule,
            format!("ValueId {value} fora da faixa 0..{}", seen.len()),
            location.clone(),
        )),
        Some(slot) if *slot => Err(invalid(
            ErrorCode::InvalidModule,
            format!("ValueId {value} definido mais de uma vez"),
            location.clone(),
        )),
        Some(slot) => {
            *slot = true;
            Ok(())
        }
    }
}

fn check_value_ref(seen: &[bool], value: u32, location: &Location) -> Result<(), RvmError> {
    let index = usize::try_from(value).unwrap_or(usize::MAX);
    if index >= seen.len() {
        return Err(invalid(
            ErrorCode::InvalidModule,
            format!("valor v{value} não definido na função"),
            location.clone(),
        ));
    }
    Ok(())
}

fn check_name(
    name: &str,
    what: &str,
    location: &Location,
    limits: &Limits,
) -> Result<(), RvmError> {
    if name.is_empty() {
        return Err(invalid(
            ErrorCode::InvalidModule,
            format!("{what} vazio"),
            location.clone(),
        ));
    }
    if name.len() > limit(limits.max_name_len) {
        return Err(invalid(
            ErrorCode::InvalidModule,
            format!(
                "{what} `{name}` excede o limite de {} bytes ({} bytes)",
                limits.max_name_len,
                name.len()
            ),
            location.clone(),
        ));
    }
    if !is_identifier(name) {
        return Err(invalid(
            ErrorCode::InvalidModule,
            format!("{what} `{name}` não é um identificador ASCII válido"),
            location.clone(),
        ));
    }
    Ok(())
}

fn reject_reserved(name: &str, location: &Location) -> Result<(), RvmError> {
    if is_reserved_value_name(name) {
        return Err(invalid(
            ErrorCode::InvalidModule,
            format!("nome `{name}` é reservado para a impressão canônica de definições (v<N>)"),
            location.clone(),
        ));
    }
    Ok(())
}

/// `[A-Za-z_][A-Za-z0-9_]*`, sem Unicode: nomes previsíveis e impressão estável.
#[must_use]
pub fn is_identifier(name: &str) -> bool {
    let mut chars = name.chars();
    match chars.next() {
        Some(first) if first.is_ascii_alphabetic() || first == '_' => {}
        _ => return false,
    }
    chars.all(|c| c.is_ascii_alphanumeric() || c == '_')
}

/// `v` seguido apenas de dígitos é reservado para nomes gerados de definições.
#[must_use]
pub fn is_reserved_value_name(name: &str) -> bool {
    name.strip_prefix('v')
        .is_some_and(|rest| !rest.is_empty() && rest.bytes().all(|byte| byte.is_ascii_digit()))
}

fn index_in_range(id: u32, bound: usize) -> bool {
    usize::try_from(id).is_ok_and(|value| value < bound)
}

fn limit(limit: u32) -> usize {
    usize::try_from(limit).unwrap_or(usize::MAX)
}

fn invalid(code: ErrorCode, detail: impl Into<String>, location: Location) -> RvmError {
    RvmError::new(code, detail).at(location)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing;
    use crate::{BlockParam, Op, Param, Terminator, ValueId};
    use rvm_spec::Type;

    fn soma() -> Module {
        testing::soma()
    }

    #[test]
    fn aceita_modulos_validos() {
        let limits = Limits::default();
        for module in [
            testing::soma(),
            testing::constante(42),
            testing::desvio_verdadeiro(),
            testing::com_merge(),
            testing::loop_infinito(),
            testing::unidade(),
            testing::overflow(),
        ] {
            let name = module.name.clone();
            assert!(
                check(&module, &limits).is_ok(),
                "módulo de teste `{name}` deveria ser estruturalmente válido"
            );
        }
    }

    #[test]
    fn rejeita_versao_desconhecida() {
        let mut module = soma();
        module.version = 2;
        assert_eq!(
            check(&module, &Limits::default()).unwrap_err().code(),
            ErrorCode::UnsupportedVersion
        );
    }

    #[test]
    fn rejeita_entry_inexistente() {
        let mut module = soma();
        module.entry = 9;
        assert_eq!(
            check(&module, &Limits::default()).unwrap_err().code(),
            ErrorCode::InvalidModule
        );
    }

    #[test]
    fn rejeita_nome_duplicado_de_funcao() {
        let mut module = soma();
        if let Some(function) = module.functions.get_mut(1) {
            function.name = "main".to_string();
        }
        let error = check(&module, &Limits::default()).unwrap_err();
        assert_eq!(error.code(), ErrorCode::InvalidModule);
        assert!(error.detail().contains("duplicado"));
    }

    #[test]
    fn rejeita_id_de_bloco_nao_denso() {
        let mut module = soma();
        if let Some(function) = module.functions.get_mut(1) {
            if let Some(block) = function.blocks.get_mut(0) {
                block.id = 7;
            }
        }
        assert_eq!(
            check(&module, &Limits::default()).unwrap_err().code(),
            ErrorCode::InvalidModule
        );
    }

    #[test]
    fn rejeita_alvo_de_desvio_inexistente() {
        let mut module = soma();
        if let Some(function) = module.functions.get_mut(0) {
            if let Some(block) = function.blocks.get_mut(0) {
                block.term = Terminator::Br(BranchTarget {
                    target: 42,
                    args: Vec::new(),
                });
            }
        }
        assert_eq!(
            check(&module, &Limits::default()).unwrap_err().code(),
            ErrorCode::InvalidControlFlow
        );
    }

    #[test]
    fn rejeita_chamada_para_funcao_inexistente() {
        let mut module = soma();
        if let Some(function) = module.functions.get_mut(0) {
            if let Some(block) = function.blocks.get_mut(0) {
                if let Some(instr) = block.instrs.get_mut(0) {
                    instr.op = Op::Call {
                        callee: 99,
                        args: Vec::new(),
                    };
                }
            }
        }
        assert_eq!(
            check(&module, &Limits::default()).unwrap_err().code(),
            ErrorCode::InvalidModule
        );
    }

    #[test]
    fn rejeita_valor_repetido() {
        let mut module = testing::soma();
        if let Some(function) = module.functions.get_mut(0) {
            if let Some(block) = function.blocks.get_mut(0) {
                if let Some(instr) = block.instrs.get_mut(1) {
                    instr.def = 0;
                }
            }
        }
        let error = check(&module, &Limits::default()).unwrap_err();
        assert_eq!(error.code(), ErrorCode::InvalidModule);
        assert!(error.detail().contains("mais de uma vez"));
    }

    #[test]
    fn rejeita_operando_inexistente() {
        let mut module = testing::soma();
        if let Some(function) = module.functions.get_mut(1) {
            if let Some(block) = function.blocks.get_mut(0) {
                if let Some(instr) = block.instrs.get_mut(0) {
                    instr.op = Op::Bin {
                        op: rvm_spec::BinOp::AddI64Checked,
                        lhs: 0,
                        rhs: 9,
                    };
                }
            }
        }
        let error = check(&module, &Limits::default()).unwrap_err();
        assert_eq!(error.code(), ErrorCode::InvalidModule);
        assert!(error.detail().contains("não definido"));
    }

    #[test]
    fn rejeita_bloco_de_entrada_com_parametros() {
        let mut module = testing::com_merge();
        if let Some(function) = module.functions.get_mut(0) {
            if let Some(block) = function.blocks.get_mut(0) {
                block.params.push(BlockParam {
                    value: 10,
                    name: "entrada".to_string(),
                    ty: Type::I64,
                });
            }
        }
        let error = check(&module, &Limits::default()).unwrap_err();
        assert_eq!(error.code(), ErrorCode::InvalidModule);
        assert!(error.detail().contains("entrada"));
    }

    #[test]
    fn rejeita_nome_reservado() {
        let mut module = soma();
        if let Some(function) = module.functions.get_mut(1) {
            if let Some(param) = function.params.get_mut(0) {
                param.name = "v0".to_string();
            }
        }
        let error = check(&module, &Limits::default()).unwrap_err();
        assert_eq!(error.code(), ErrorCode::InvalidModule);
        assert!(error.detail().contains("reservado"));
    }

    #[test]
    fn rejeita_identificador_invalido() {
        let mut module = soma();
        module.name = "com espaço".to_string();
        assert_eq!(
            check(&module, &Limits::default()).unwrap_err().code(),
            ErrorCode::InvalidModule
        );
    }

    #[test]
    fn respeita_limite_de_funcoes_antes_de_alocar() {
        let module = soma();
        let limits = Limits {
            max_functions: 1,
            ..Limits::default()
        };
        let error = check(&module, &limits).unwrap_err();
        assert_eq!(error.code(), ErrorCode::InvalidModule);
        assert!(error.detail().contains("limite de funções"));
    }

    #[test]
    fn respeita_limite_de_instrucoes_por_bloco() {
        let module = soma();
        let limits = Limits {
            max_instrs_per_block: 1,
            ..Limits::default()
        };
        let error = check(&module, &limits).unwrap_err();
        assert_eq!(error.code(), ErrorCode::InvalidModule);
        assert!(error.detail().contains("instruções"));
    }

    #[test]
    fn nomes_conhecidos_sao_identificadores() {
        assert!(is_identifier("soma"));
        assert!(is_identifier("_x1"));
        assert!(!is_identifier("1x"));
        assert!(!is_identifier("com espaço"));
        assert!(!is_identifier("ação"));
        assert!(is_reserved_value_name("v0"));
        assert!(is_reserved_value_name("v42"));
        assert!(!is_reserved_value_name("v"));
        assert!(!is_reserved_value_name("valor"));
    }

    #[test]
    fn construtor_de_parametros_aceita_i64() {
        let module = Module {
            name: "m".to_string(),
            version: IR_VERSION,
            entry: 0,
            functions: vec![Function {
                id: 0,
                name: "main".to_string(),
                params: vec![Param {
                    name: "a".to_string(),
                    ty: Type::I64,
                }],
                ret: Type::I64,
                entry: 0,
                blocks: vec![crate::Block {
                    id: 0,
                    label: "entry".to_string(),
                    params: Vec::new(),
                    instrs: Vec::new(),
                    term: Terminator::Return(Some(0)),
                }],
            }],
        };
        assert!(check(&module, &Limits::default()).is_ok());
        // v0 é o parâmetro; `return v0` é válido estruturalmente.
        let value: ValueId = 0;
        assert_eq!(value, 0);
    }

    #[test]
    fn aceita_limite_estrutural_exato() {
        // Fronteira inclusiva: `soma` tem 2 funções, 1 bloco por função, 2
        // parâmetros em `soma`, 3 instruções no bloco de `main`, 3 valores por
        // função e 2 argumentos de chamada. Todos os limites exatos passam.
        let module = soma();
        let limits = Limits {
            max_functions: 2,
            max_blocks_per_function: 1,
            max_params: 2,
            max_block_params: 0,
            max_instrs_per_block: 3,
            max_values_per_function: 3,
            max_call_args: 2,
            ..Limits::default()
        };
        assert!(
            check(&module, &limits).is_ok(),
            "módulo no limite exato deveria ser aceito"
        );
    }

    #[test]
    fn rejeita_parametros_de_funcao_acima_do_limite() {
        let module = soma();
        let limits = Limits {
            max_params: 1,
            ..Limits::default()
        };
        let error = check(&module, &limits).unwrap_err();
        assert_eq!(error.code(), ErrorCode::InvalidModule);
        assert!(error.detail().contains("parâmetros"));
    }

    #[test]
    fn rejeita_parametros_de_bloco_acima_do_limite() {
        let module = testing::com_merge();
        let limits = Limits {
            max_block_params: 0,
            ..Limits::default()
        };
        let error = check(&module, &limits).unwrap_err();
        assert_eq!(error.code(), ErrorCode::InvalidModule);
        assert!(error.detail().contains("parâmetros de bloco"));
    }

    #[test]
    fn rejeita_argumentos_de_chamada_acima_do_limite() {
        let module = soma();
        let limits = Limits {
            max_call_args: 1,
            ..Limits::default()
        };
        let error = check(&module, &limits).unwrap_err();
        assert_eq!(error.code(), ErrorCode::InvalidModule);
        assert!(error.detail().contains("argumentos de chamada"));
    }

    #[test]
    fn rejeita_argumentos_de_desvio_acima_do_limite() {
        let module = testing::com_merge();
        let limits = Limits {
            max_call_args: 0,
            ..Limits::default()
        };
        let error = check(&module, &limits).unwrap_err();
        assert_eq!(error.code(), ErrorCode::InvalidModule);
        assert!(error.detail().contains("argumentos de desvio"));
    }

    #[test]
    fn rejeita_blocos_acima_do_limite() {
        let module = testing::com_merge();
        let limits = Limits {
            max_blocks_per_function: 1,
            ..Limits::default()
        };
        let error = check(&module, &limits).unwrap_err();
        assert_eq!(error.code(), ErrorCode::InvalidModule);
        assert!(error.detail().contains("blocos"));
    }

    #[test]
    fn rejeita_valores_acima_do_limite() {
        let module = soma();
        let limits = Limits {
            max_values_per_function: 2,
            ..Limits::default()
        };
        let error = check(&module, &limits).unwrap_err();
        assert_eq!(error.code(), ErrorCode::InvalidModule);
        assert!(error.detail().contains("valores"));
    }

    #[test]
    fn aceita_declaracao_externa_sem_corpo() {
        let mut module = soma();
        module.functions.push(Function {
            id: 2,
            name: "externa".to_string(),
            params: Vec::new(),
            ret: Type::Unit,
            entry: 0,
            blocks: Vec::new(),
        });
        assert!(
            check(&module, &Limits::default()).is_ok(),
            "declaração externa deveria ser estruturalmente aceita"
        );
    }

    #[test]
    fn rejeita_entrada_externa() {
        let mut module = soma();
        // `main` (função 0) vira externa e a entrada aponta para ela.
        if let Some(function) = module.functions.get_mut(0) {
            function.blocks.clear();
        }
        let error = check(&module, &Limits::default()).unwrap_err();
        assert_eq!(error.code(), ErrorCode::InvalidModule);
        assert!(error.detail().contains("externa"));
    }

    #[test]
    fn rejeita_string_acima_do_limite() {
        let mut module = testing::constante(0);
        module.functions[0].ret = Type::Str;
        module.functions[0].blocks[0].instrs[0].op = Op::ConstStr("x".repeat(10));
        let limits = Limits {
            max_string_bytes: 5,
            ..Limits::default()
        };
        let error = check(&module, &limits).unwrap_err();
        assert_eq!(error.code(), ErrorCode::InvalidModule);
        assert!(error.detail().contains("string"));
    }
}
