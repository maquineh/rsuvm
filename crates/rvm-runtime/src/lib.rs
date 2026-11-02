//! `rvm-runtime` — valores, quotas de execução e capabilities do host.
//!
//! Este crate implementa a regra **deny-by-default** do ADR-0005: não existe
//! caminho no EXP-0 que conceda uma capability. [`HostApi::request`] sempre falha
//! com [`rvm_spec::ErrorCode::CapabilityDenied`] e nenhuma função aqui toca
//! arquivos, rede, ambiente, relógio ou processos do sistema operacional.
//!
//! As quotas de execução são objetos de valor já validados: [`CallDepthLimit`]
//! só existem no intervalo permitido. O teto limita a profundidade convidada,
//! mas não é quota de memória em bytes; a pilha explícita de frames na heap
//! ainda precisa de um orçamento próprio (ver ADR-0012).

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

use core::fmt;

use rvm_spec::{ErrorCode, RvmError, Type};

/// Fuel padrão de uma execução quando o host não especifica outro valor.
pub const DEFAULT_FUEL: u64 = 100_000;

/// Teto padrão de memória modelada para frames (16 MiB).
pub const DEFAULT_MEMORY_LIMIT_BYTES: u64 = 16 * 1024 * 1024;

/// Custo fixo modelado de um frame, em bytes (cabeçalho e estado do frame).
pub const FRAME_BASE_BYTES: u64 = 256;

/// Custo modelado de um slot de valor, em bytes.
pub const SLOT_BYTES: u64 = 24;

/// Custo modelado de um frame com `slots` valores, em bytes.
///
/// É um **modelo determinístico** de quota, não a medição de RSS do processo:
/// `FRAME_BASE_BYTES + slots * SLOT_BYTES`, com saturação.
#[must_use]
pub fn frame_cost(slots: u64) -> u64 {
    slots
        .saturating_mul(SLOT_BYTES)
        .saturating_add(FRAME_BASE_BYTES)
}

/// Custo modelado de um buffer transitório de `args` argumentos, em bytes.
///
/// Durante uma chamada ou um desvio, os argumentos são copiados para um buffer
/// antes de serem gravados nos slots do destino. Esse buffer é memória viva
/// enquanto existe e entra na quota de execução (ADR-0016). O custo é
/// `args * SLOT_BYTES`, com saturação; um buffer vazio não custa nada.
#[must_use]
pub fn arg_buffer_cost(args: u64) -> u64 {
    args.saturating_mul(SLOT_BYTES)
}

/// Limite de memória modelada de frames, em bytes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MemoryLimit(u64);

impl MemoryLimit {
    /// Limite padrão de 16 MiB.
    pub const DEFAULT: Self = Self(DEFAULT_MEMORY_LIMIT_BYTES);

    /// Cria um limite a partir de bytes. `0` é aceito: nenhum frame cabe.
    #[must_use]
    pub const fn new(bytes: u64) -> Self {
        Self(bytes)
    }

    /// Valor em bytes.
    #[must_use]
    pub const fn get(self) -> u64 {
        self.0
    }
}

impl Default for MemoryLimit {
    fn default() -> Self {
        Self::DEFAULT
    }
}

/// Valor de runtime do subset EXP-0.
///
/// Um `Value` sempre conhece seu [`Type`] e nunca carrega ponteiro, referência ou
/// handle do processo hospedeiro.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Value {
    /// Valor do tipo `unit`.
    Unit,
    /// Valor do tipo `bool`.
    Bool(bool),
    /// Valor do tipo `i64`.
    I64(i64),
}

impl Value {
    /// Tipo do valor.
    #[must_use]
    pub const fn ty(self) -> Type {
        match self {
            Value::Unit => Type::Unit,
            Value::Bool(_) => Type::Bool,
            Value::I64(_) => Type::I64,
        }
    }

    /// Conteúdo booleano, se o valor for `bool`.
    #[must_use]
    pub const fn as_bool(self) -> Option<bool> {
        match self {
            Value::Bool(value) => Some(value),
            _ => None,
        }
    }

    /// Conteúdo inteiro, se o valor for `i64`.
    #[must_use]
    pub const fn as_i64(self) -> Option<i64> {
        match self {
            Value::I64(value) => Some(value),
            _ => None,
        }
    }

    /// `true` se o valor pertence ao tipo declarado (sem conversão implícita).
    #[must_use]
    pub const fn matches_type(self, ty: Type) -> bool {
        matches!(
            (self, ty),
            (Value::Unit, Type::Unit) | (Value::Bool(_), Type::Bool) | (Value::I64(_), Type::I64)
        )
    }
}

impl fmt::Display for Value {
    /// Formato estável usado pelo CLI e pelo corpus: `unit`, `bool:true`, `i64:30`.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Value::Unit => f.write_str("unit"),
            Value::Bool(value) => write!(f, "bool:{value}"),
            Value::I64(value) => write!(f, "i64:{value}"),
        }
    }
}

/// Limite de profundidade de chamadas já validado.
///
/// O teto [`CallDepthLimit::HARD_MAX`] limita a profundidade convidada. O
/// intérprete usa frames explícitos na heap, então cada chamada convidada não
/// consome um frame da pilha nativa. Este limite não contabiliza a memória total
/// dos frames em bytes; essa quota permanece pendente (ADR-0012).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CallDepthLimit(u16);

impl CallDepthLimit {
    /// Teto absoluto aceito pelo protótipo.
    pub const HARD_MAX: u16 = 256;

    /// Limite padrão de chamadas aninhadas.
    pub const DEFAULT: Self = Self(64);

    /// Cria um limite, devolvendo [`None`] para `0` ou acima de
    /// [`CallDepthLimit::HARD_MAX`].
    #[must_use]
    pub const fn new(limit: u16) -> Option<Self> {
        if limit == 0 || limit > Self::HARD_MAX {
            None
        } else {
            Some(Self(limit))
        }
    }

    /// Valor numérico do limite.
    #[must_use]
    pub const fn get(self) -> u16 {
        self.0
    }
}

impl Default for CallDepthLimit {
    fn default() -> Self {
        Self::DEFAULT
    }
}

/// Configuração de execução: fuel e profundidade.
///
/// Fuel é debitado antes de cada instrução/terminador despachado; chamadas não
/// resetam o saldo.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ExecConfig {
    fuel: u64,
    max_call_depth: CallDepthLimit,
    memory_limit: MemoryLimit,
}

impl ExecConfig {
    /// Cria uma configuração com fuel e profundidade já validada.
    #[must_use]
    pub const fn new(fuel: u64, max_call_depth: CallDepthLimit) -> Self {
        Self {
            fuel,
            max_call_depth,
            memory_limit: MemoryLimit::DEFAULT,
        }
    }

    /// Saldo inicial de fuel.
    #[must_use]
    pub const fn fuel(self) -> u64 {
        self.fuel
    }

    /// Limite de profundidade de chamadas.
    #[must_use]
    pub const fn max_call_depth(self) -> CallDepthLimit {
        self.max_call_depth
    }
    /// Define o limite de memória modelada dos frames.
    #[must_use]
    pub const fn with_memory(mut self, limit: MemoryLimit) -> Self {
        self.memory_limit = limit;
        self
    }

    /// Limite de memória modelada dos frames.
    #[must_use]
    pub const fn memory_limit(self) -> MemoryLimit {
        self.memory_limit
    }
}

impl Default for ExecConfig {
    fn default() -> Self {
        Self {
            fuel: DEFAULT_FUEL,
            max_call_depth: CallDepthLimit::DEFAULT,
            memory_limit: MemoryLimit::DEFAULT,
        }
    }
}

/// Efeito de host que exigiria capability explícita.
///
/// Nenhum destes efeitos é oferecido ao convidado no EXP-0.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Effect {
    /// Leitura de arquivo.
    FilesystemRead,
    /// Escrita de arquivo.
    FilesystemWrite,
    /// Rede (cliente ou servidor).
    Network,
    /// Variáveis de ambiente.
    Environment,
    /// Relógio (data/hora, temporização).
    Clock,
    /// Criação de subprocesso ou sinal.
    Process,
}

impl Effect {
    /// Nome estável do efeito em relatórios e erros.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Effect::FilesystemRead => "filesystem_read",
            Effect::FilesystemWrite => "filesystem_write",
            Effect::Network => "network",
            Effect::Environment => "environment",
            Effect::Clock => "clock",
            Effect::Process => "process",
        }
    }
}

impl fmt::Display for Effect {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.name())
    }
}

/// Conjunto de capabilities concedidas ao convidado.
///
/// No EXP-0 só existe [`Capabilities::none`]; o campo é privado e não há método
/// de concessão. Adicionar um caminho de concessão exige ADR e aprovação
/// explícita do mantenedor (ADR-0005).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Capabilities {
    granted: Vec<Effect>,
}

impl Capabilities {
    /// Conjunto vazio: nenhuma capability concedida.
    #[must_use]
    pub fn none() -> Self {
        Self {
            granted: Vec::new(),
        }
    }

    /// Lista das capabilities concedidas (sempre vazia no EXP-0).
    #[must_use]
    pub fn granted(&self) -> &[Effect] {
        &self.granted
    }

    /// `true` somente se a capability foi concedida explicitamente.
    #[must_use]
    pub fn is_granted(&self, effect: Effect) -> bool {
        self.granted.contains(&effect)
    }
}

/// API de efeitos do host, sem efeitos.
///
/// Toda requisição falha de forma fechada: `capability_denied`. O parâmetro
/// `scope` existe para que o host futuro descreva o alvo pedido (por exemplo
/// `file:/tmp/x`), mas não é interpretado nem registrado em log pelo EXP-0.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HostApi {
    capabilities: Capabilities,
}

impl HostApi {
    /// Constrói a API com um conjunto de capabilities (o MVP não tem como
    /// construir um conjunto não vazio).
    #[must_use]
    pub fn new(capabilities: Capabilities) -> Self {
        Self { capabilities }
    }

    /// Constrói a API sem nenhuma capability (caso do MVP e do CLI).
    #[must_use]
    pub fn standalone() -> Self {
        Self::new(Capabilities::none())
    }

    /// Capabilities deste host.
    #[must_use]
    pub fn capabilities(&self) -> &Capabilities {
        &self.capabilities
    }

    /// Solicita permissão para executar um efeito.
    ///
    /// # Errors
    ///
    /// Sempre devolve [`ErrorCode::CapabilityDenied`]: o MVP não concede
    /// capabilities e o caso `Ok(())` não é alcançável.
    pub fn request(&self, effect: Effect, scope: &str) -> Result<(), RvmError> {
        if self.capabilities.is_granted(effect) {
            // Inalcançável no EXP-0: não existe construtor que popule o conjunto.
            return Err(RvmError::new(
                ErrorCode::UnsupportedFeature,
                format!(
                    "capability `{}` concedida, mas executores de efeito não existem no EXP-0",
                    effect.name()
                ),
            ));
        }
        Err(RvmError::new(
            ErrorCode::CapabilityDenied,
            format!(
                "capability `{}` negada para escopo `{scope}` (deny-by-default)",
                effect.name()
            ),
        ))
    }
}

impl Default for HostApi {
    fn default() -> Self {
        Self::standalone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn valores_conhecem_seu_tipo() {
        assert_eq!(Value::Unit.ty(), Type::Unit);
        assert_eq!(Value::Bool(true).ty(), Type::Bool);
        assert_eq!(Value::I64(-3).ty(), Type::I64);
        assert!(Value::I64(0).matches_type(Type::I64));
        assert!(!Value::I64(0).matches_type(Type::Bool));
        assert_eq!(Value::I64(30).to_string(), "i64:30");
        assert_eq!(Value::Bool(false).to_string(), "bool:false");
        assert_eq!(Value::Unit.to_string(), "unit");
    }

    #[test]
    fn profundidade_zero_ou_acima_do_teto_e_rejeitada() {
        assert!(CallDepthLimit::new(0).is_none());
        assert!(CallDepthLimit::new(CallDepthLimit::HARD_MAX).is_some());
        assert!(CallDepthLimit::new(CallDepthLimit::HARD_MAX + 1).is_none());
        assert_eq!(CallDepthLimit::new(8).map(CallDepthLimit::get), Some(8));
    }

    #[test]
    fn configuracao_padrao_tem_fuel_e_profundidade() {
        let config = ExecConfig::default();
        assert_eq!(config.fuel(), DEFAULT_FUEL);
        assert_eq!(config.max_call_depth().get(), 64);
    }

    #[test]
    fn nenhuma_capability_e_concedida_por_padrao() {
        let host = HostApi::standalone();
        assert!(host.capabilities().granted().is_empty());
        for effect in [
            Effect::FilesystemRead,
            Effect::FilesystemWrite,
            Effect::Network,
            Effect::Environment,
            Effect::Clock,
            Effect::Process,
        ] {
            assert!(!host.capabilities().is_granted(effect));
            let error = host
                .request(effect, "escopo-de-teste")
                .expect_err("capability deve ser negada");
            assert_eq!(error.code(), ErrorCode::CapabilityDenied);
            assert_eq!(error.class(), rvm_spec::ErrorClass::Capability);
        }
    }
    #[test]
    fn custo_de_frame_e_modelado_e_satura() {
        assert_eq!(frame_cost(0), FRAME_BASE_BYTES);
        assert_eq!(frame_cost(1), FRAME_BASE_BYTES + SLOT_BYTES);
        assert_eq!(frame_cost(u64::MAX), u64::MAX);
    }

    #[test]
    fn custo_de_buffer_de_argumentos_e_modelado_e_satura() {
        assert_eq!(arg_buffer_cost(0), 0);
        assert_eq!(arg_buffer_cost(1), SLOT_BYTES);
        assert_eq!(arg_buffer_cost(16), 16 * SLOT_BYTES);
        assert_eq!(arg_buffer_cost(u64::MAX), u64::MAX);
    }

    #[test]
    fn configuracao_carrega_limite_de_memoria() {
        let config = ExecConfig::default();
        assert_eq!(config.memory_limit(), MemoryLimit::DEFAULT);
        assert_eq!(config.memory_limit().get(), DEFAULT_MEMORY_LIMIT_BYTES);
        let pequeno =
            ExecConfig::new(10, CallDepthLimit::DEFAULT).with_memory(MemoryLimit::new(100));
        assert_eq!(pequeno.memory_limit().get(), 100);
        assert_eq!(pequeno.fuel(), 10);
    }
}
