//! `rvm-runtime` — valores, quotas de execução e capabilities do host.
//!
//! Este crate implementa a regra **deny-by-default** do ADR-0005: o conjunto de
//! capabilities nasce vazio e só cresce por construtores explícitos do host
//! ([`Capabilities::grant`]/[`Capabilities::with_grant`], ADR-0036).
//! [`HostApi::request`] devolve [`rvm_spec::ErrorCode::CapabilityDenied`] para
//! qualquer efeito/escopo não coberto, e nenhuma função aqui toca arquivos, rede,
//! ambiente, relógio ou processos do sistema operacional.
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

use rvm_spec::{
    BuiltinHost, ErrorCode, RvmError, StructId, TypeId, TYPE_BOOL, TYPE_F64, TYPE_I64, TYPE_STR,
    TYPE_UNIT,
};

pub use rvm_spec::Effect;

/// Fuel padrão de uma execução quando o host não especifica outro valor.
pub const DEFAULT_FUEL: u64 = 100_000;

/// Teto padrão de memória modelada para frames (16 MiB).
pub const DEFAULT_MEMORY_LIMIT_BYTES: u64 = 16 * 1024 * 1024;

/// Limite padrão, em bytes, de uma `string` produzida por concatenação (ADR-0025).
pub const DEFAULT_MAX_STRING_BYTES: u32 = 4096;

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
/// Cada valor carrega o [`TypeId`] do seu tipo (ADR-0034). `Str` carrega o texto
/// UTF-8 por valor (ADR-0024) e `F64` um ponto flutuante IEEE-754 (ADR-0026). Não
/// implementa `Eq` por causa de `NaN`.
#[derive(Clone, Debug, PartialEq)]
pub enum Value {
    /// Valor do tipo `unit`.
    Unit,
    /// Valor do tipo `bool`.
    Bool(bool),
    /// Valor do tipo `i64`.
    I64(i64),
    /// Valor do tipo `string` (texto UTF-8 imutável).
    Str(String),
    /// Valor do tipo `f64` (IEEE-754).
    F64(f64),
    /// Valor do tipo `array` (ADR-0027/0034).
    Array {
        /// Tipo do array na tabela do módulo.
        ty: TypeId,
        /// Elementos, na ordem.
        items: Vec<Value>,
    },
    /// Valor de struct (ADR-0029/0034).
    Struct {
        /// Tipo da struct na tabela do módulo.
        ty: TypeId,
        /// Struct da qual o valor é instância.
        struct_id: StructId,
        /// Valores dos campos, na ordem de declaração.
        fields: Vec<Value>,
    },
    /// Valor opcional (ADR-0030/0034).
    Option {
        /// Tipo do `option` na tabela do módulo.
        ty: TypeId,
        /// `None` (ausente) ou `Some` com o payload.
        value: Option<Box<Value>>,
    },
    /// Valor de resultado (ADR-0033/0034).
    Result {
        /// Tipo do `result` na tabela do módulo.
        ty: TypeId,
        /// Variante: `Ok` com o payload, ou `Err` com o payload.
        value: std::result::Result<Box<Value>, Box<Value>>,
    },
}

impl Value {
    /// `TypeId` do valor (os escalares usam os IDs canônicos).
    #[must_use]
    pub fn ty(&self) -> TypeId {
        match self {
            Value::Unit => TYPE_UNIT,
            Value::Bool(_) => TYPE_BOOL,
            Value::I64(_) => TYPE_I64,
            Value::Str(_) => TYPE_STR,
            Value::F64(_) => TYPE_F64,
            Value::Array { ty, .. }
            | Value::Struct { ty, .. }
            | Value::Option { ty, .. }
            | Value::Result { ty, .. } => *ty,
        }
    }

    /// Conteúdo booleano, se o valor for `bool`.
    #[must_use]
    pub const fn as_bool(&self) -> Option<bool> {
        match self {
            Value::Bool(value) => Some(*value),
            _ => None,
        }
    }

    /// Conteúdo inteiro, se o valor for `i64`.
    #[must_use]
    pub const fn as_i64(&self) -> Option<i64> {
        match self {
            Value::I64(value) => Some(*value),
            _ => None,
        }
    }

    /// Texto, se o valor for `string`.
    #[must_use]
    pub fn as_str(&self) -> Option<&str> {
        match self {
            Value::Str(value) => Some(value.as_str()),
            _ => None,
        }
    }

    /// Conteúdo `f64`, se o valor for `f64`.
    #[must_use]
    pub const fn as_f64(&self) -> Option<f64> {
        match self {
            Value::F64(value) => Some(*value),
            _ => None,
        }
    }

    /// Elementos, se o valor for `array`.
    #[must_use]
    pub fn as_array(&self) -> Option<&[Value]> {
        match self {
            Value::Array { items, .. } => Some(items.as_slice()),
            _ => None,
        }
    }

    /// Campos, se o valor for struct.
    #[must_use]
    pub fn as_struct(&self) -> Option<&[Value]> {
        match self {
            Value::Struct { fields, .. } => Some(fields.as_slice()),
            _ => None,
        }
    }

    /// Payload de `some`, se o valor for `option` com payload; `None` para `none`
    /// e para valores que não são `option`.
    #[must_use]
    pub fn as_option(&self) -> Option<Option<&Value>> {
        match self {
            Value::Option { value, .. } => Some(value.as_deref()),
            _ => None,
        }
    }

    /// Variante, se o valor for `result`.
    #[must_use]
    pub fn as_result(&self) -> Option<std::result::Result<&Value, &Value>> {
        match self {
            Value::Result { value, .. } => Some(match value {
                Ok(inner) => Ok(inner.as_ref()),
                Err(inner) => Err(inner.as_ref()),
            }),
            _ => None,
        }
    }

    /// `true` se o valor pertence ao tipo declarado (sem conversão implícita).
    #[must_use]
    pub fn matches_type(&self, ty: TypeId) -> bool {
        self.ty() == ty
    }

    /// Bytes modelados retidos pelo valor (ADR-0035).
    ///
    /// `unit`/escalares custam 0; `string` custa o tamanho UTF-8; `array` e
    /// `struct` custam `SLOT_BYTES` por elemento/campo mais o custo recursivo dos
    /// filhos; `option`/`result` custam `SLOT_BYTES` quando carregam payload. A
    /// soma satura em [`u64::MAX`] e é determinística.
    #[must_use]
    pub fn retained_bytes(&self) -> u64 {
        match self {
            Value::Unit | Value::Bool(_) | Value::I64(_) | Value::F64(_) => 0,
            Value::Str(text) => u64::try_from(text.len()).unwrap_or(u64::MAX),
            Value::Array { items, .. } => u64::try_from(items.len())
                .unwrap_or(u64::MAX)
                .saturating_mul(SLOT_BYTES)
                .saturating_add(sum_retained(items.iter())),
            Value::Struct { fields, .. } => u64::try_from(fields.len())
                .unwrap_or(u64::MAX)
                .saturating_mul(SLOT_BYTES)
                .saturating_add(sum_retained(fields.iter())),
            Value::Option { value, .. } => value
                .as_deref()
                .map_or(0, |inner| SLOT_BYTES.saturating_add(inner.retained_bytes())),
            Value::Result { value, .. } => match value {
                Ok(inner) | Err(inner) => SLOT_BYTES.saturating_add(inner.retained_bytes()),
            },
        }
    }
}

/// Soma saturada de [`Value::retained_bytes`].
fn sum_retained<'a>(values: impl Iterator<Item = &'a Value>) -> u64 {
    values.fold(0u64, |total, value| {
        total.saturating_add(value.retained_bytes())
    })
}

impl fmt::Display for Value {
    /// Formato estável usado pelo CLI e pelo corpus: `unit`, `bool:true`,
    /// `i64:30`, `str:<texto>`, `f64:<valor>`, `array:[<elementos>]`,
    /// `struct:<id>[<campos>]`.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Value::Unit => f.write_str("unit"),
            Value::Bool(value) => write!(f, "bool:{value}"),
            Value::I64(value) => write!(f, "i64:{value}"),
            Value::Str(value) => write!(f, "str:{value}"),
            Value::F64(value) => write!(f, "f64:{value}"),
            Value::Array { items, .. } => {
                f.write_str("array:[")?;
                for (index, item) in items.iter().enumerate() {
                    if index > 0 {
                        f.write_str(", ")?;
                    }
                    write!(f, "{item}")?;
                }
                f.write_str("]")
            }
            Value::Struct {
                struct_id, fields, ..
            } => {
                write!(f, "struct:{struct_id}[")?;
                for (index, field) in fields.iter().enumerate() {
                    if index > 0 {
                        f.write_str(", ")?;
                    }
                    write!(f, "{field}")?;
                }
                f.write_str("]")
            }
            Value::Option { value, .. } => match value {
                None => f.write_str("none"),
                Some(inner) => write!(f, "some:{inner}"),
            },
            Value::Result { value, .. } => match value {
                Ok(inner) => write!(f, "ok:{inner}"),
                Err(inner) => write!(f, "err:{inner}"),
            },
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
    max_string_bytes: u32,
}

impl ExecConfig {
    /// Cria uma configuração com fuel e profundidade já validada.
    #[must_use]
    pub const fn new(fuel: u64, max_call_depth: CallDepthLimit) -> Self {
        Self {
            fuel,
            max_call_depth,
            memory_limit: MemoryLimit::DEFAULT,
            max_string_bytes: DEFAULT_MAX_STRING_BYTES,
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

    /// Define o limite, em bytes, de uma `string` produzida por concatenação.
    #[must_use]
    pub const fn with_max_string_bytes(mut self, bytes: u32) -> Self {
        self.max_string_bytes = bytes;
        self
    }

    /// Limite, em bytes, de uma `string` produzida por concatenação.
    #[must_use]
    pub const fn max_string_bytes(self) -> u32 {
        self.max_string_bytes
    }
}

impl Default for ExecConfig {
    fn default() -> Self {
        Self {
            fuel: DEFAULT_FUEL,
            max_call_depth: CallDepthLimit::DEFAULT,
            memory_limit: MemoryLimit::DEFAULT,
            max_string_bytes: DEFAULT_MAX_STRING_BYTES,
        }
    }
}

/// Concessão de uma capability a um escopo (host-side, ADR-0036).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CapabilityGrant {
    effect: Effect,
    scope: String,
}

impl CapabilityGrant {
    /// Cria uma concessão de `effect` para `scope`.
    #[must_use]
    pub fn new(effect: Effect, scope: impl Into<String>) -> Self {
        Self {
            effect,
            scope: scope.into(),
        }
    }

    /// Efeito concedido.
    #[must_use]
    pub const fn effect(&self) -> Effect {
        self.effect
    }

    /// Escopo concedido (vazio = qualquer escopo).
    #[must_use]
    pub fn scope(&self) -> &str {
        &self.scope
    }
}

/// Conjunto de capabilities concedidas ao convidado.
///
/// O convidado **não** tem caminho de concessão: só o host constrói o conjunto
/// (ADR-0005/ADR-0036). `allows` exige o efeito e compara o escopo pedido com o
/// concedido por componentes de caminho (`Path::starts_with`); escopo vazio
/// concede qualquer caminho sem `..`.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Capabilities {
    grants: Vec<CapabilityGrant>,
}

impl Capabilities {
    /// Conjunto vazio: nenhuma capability concedida.
    #[must_use]
    pub fn none() -> Self {
        Self { grants: Vec::new() }
    }

    /// Conjunto com uma concessão explícita (host-side).
    #[must_use]
    pub fn grant(effect: Effect, scope: impl Into<String>) -> Self {
        Self {
            grants: vec![CapabilityGrant::new(effect, scope)],
        }
    }

    /// Adiciona uma concessão explícita.
    #[must_use]
    pub fn with_grant(mut self, effect: Effect, scope: impl Into<String>) -> Self {
        self.grants.push(CapabilityGrant::new(effect, scope));
        self
    }

    /// Lista das concessões.
    #[must_use]
    pub fn granted(&self) -> &[CapabilityGrant] {
        &self.grants
    }

    /// `true` se o efeito foi concedido para algum escopo.
    #[must_use]
    pub fn is_granted(&self, effect: Effect) -> bool {
        self.grants.iter().any(|grant| grant.effect == effect)
    }

    /// `true` se `effect` foi concedido para `scope`.
    ///
    /// Caminhos com componente `..` são negados: sem canonicalização, `..` pode
    /// escapar do escopo por resolução do sistema de arquivos. A comparação usa
    /// componentes (`Path::starts_with`), então `base/sub` não casa
    /// `base/submarine`.
    #[must_use]
    pub fn allows(&self, effect: Effect, scope: &str) -> bool {
        let requested = std::path::Path::new(scope);
        if requested
            .components()
            .any(|component| matches!(component, std::path::Component::ParentDir))
        {
            return false;
        }
        self.grants.iter().any(|grant| {
            grant.effect == effect
                && (grant.scope.is_empty()
                    || requested.starts_with(std::path::Path::new(&grant.scope)))
        })
    }
}

/// Backend de host para chamadas de host (ADR-0036).
///
/// O host executa o efeito e valida o escopo no seu contexto. O intérprete
/// confere a capability e o tipo do valor devolvido; o host não pode alargar o
/// conjunto de capabilities por conta própria.
pub trait Host {
    /// Capabilities concedidas a esta execução.
    fn capabilities(&self) -> &Capabilities;
    /// Executa `function` com `args`, devolvendo um valor do tipo `ret`.
    ///
    /// # Errors
    ///
    /// Cada host devolve os erros do seu efeito; `capability_denied` é o padrão
    /// para negação e `invalid_type` para retorno incompatível.
    fn call(
        &mut self,
        function: BuiltinHost,
        args: &[Value],
        ret: TypeId,
    ) -> Result<Value, RvmError>;
}

/// Host que nega toda chamada de host; usado pelo executor sem backend.
#[derive(Clone, Debug, Default)]
pub struct NoHost {
    capabilities: Capabilities,
}

impl NoHost {
    /// Host sem capabilities.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }
}

impl Host for NoHost {
    fn capabilities(&self) -> &Capabilities {
        &self.capabilities
    }

    fn call(
        &mut self,
        function: BuiltinHost,
        _args: &[Value],
        _ret: TypeId,
    ) -> Result<Value, RvmError> {
        Err(RvmError::new(
            ErrorCode::CapabilityDenied,
            format!(
                "capability `{}` negada: nenhum host de efeitos conectado (ADR-0036)",
                function.effect().name()
            ),
        ))
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
    /// Constrói a API com um conjunto de capabilities.
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

    /// Solicita permissão para executar um efeito em `scope`.
    ///
    /// # Errors
    ///
    /// Devolve [`ErrorCode::CapabilityDenied`] quando o efeito não foi concedido
    /// ou o escopo não está coberto por uma concessão.
    pub fn request(&self, effect: Effect, scope: &str) -> Result<(), RvmError> {
        if self.capabilities.allows(effect, scope) {
            return Ok(());
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
        assert_eq!(Value::Unit.ty(), TYPE_UNIT);
        assert_eq!(Value::Bool(true).ty(), TYPE_BOOL);
        assert_eq!(Value::I64(-3).ty(), TYPE_I64);
        assert!(Value::I64(0).matches_type(TYPE_I64));
        assert!(!Value::I64(0).matches_type(TYPE_BOOL));
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
    fn bytes_retidos_do_valor_sao_recursivos() {
        assert_eq!(Value::Unit.retained_bytes(), 0);
        assert_eq!(Value::I64(7).retained_bytes(), 0);
        assert_eq!(Value::Str("abc".to_string()).retained_bytes(), 3);
        let array = Value::Array {
            ty: 5,
            items: vec![Value::I64(1), Value::Str("ab".to_string())],
        };
        assert_eq!(array.retained_bytes(), 2 * SLOT_BYTES + 2);
        let option = Value::Option {
            ty: 6,
            value: Some(Box::new(Value::Str("ab".to_string()))),
        };
        assert_eq!(option.retained_bytes(), SLOT_BYTES + 2);
        let none = Value::Option { ty: 6, value: None };
        assert_eq!(none.retained_bytes(), 0);
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
    fn capability_concedida_cobre_o_escopo_e_subcaminhos() {
        let capabilities = Capabilities::grant(Effect::FilesystemRead, "base");
        assert!(capabilities.allows(Effect::FilesystemRead, "base"));
        assert!(capabilities.allows(Effect::FilesystemRead, "base/sub/arquivo.txt"));
        assert!(!capabilities.allows(Effect::FilesystemRead, "outro"));
        assert!(!capabilities.allows(Effect::FilesystemWrite, "base"));
        assert!(capabilities.is_granted(Effect::FilesystemRead));
        assert_eq!(capabilities.granted().len(), 1);
    }

    #[test]
    fn capability_nao_casa_prefixo_textual_de_componente() {
        let capabilities = Capabilities::grant(Effect::FilesystemRead, "base/sub");
        assert!(capabilities.allows(Effect::FilesystemRead, "base/sub/arquivo"));
        assert!(!capabilities.allows(Effect::FilesystemRead, "base/submarine"));
    }

    #[test]
    fn capability_nega_caminho_com_parent_dir() {
        let capabilities = Capabilities::grant(Effect::FilesystemRead, "base");
        assert!(!capabilities.allows(Effect::FilesystemRead, "base/../fora"));
        assert!(!capabilities.allows(Effect::FilesystemRead, ".."));
    }

    #[test]
    fn escopo_vazio_concede_qualquer_caminho() {
        let capabilities = Capabilities::none().with_grant(Effect::FilesystemRead, "");
        assert!(capabilities.allows(Effect::FilesystemRead, "qualquer/caminho"));
        assert!(!capabilities.allows(Effect::FilesystemWrite, "qualquer/caminho"));
    }

    #[test]
    fn host_api_request_permite_escopo_concedido() {
        let host = HostApi::new(Capabilities::grant(Effect::FilesystemRead, "base"));
        assert!(host.request(Effect::FilesystemRead, "base/arquivo").is_ok());
        let error = host
            .request(Effect::FilesystemRead, "outro/arquivo")
            .expect_err("fora do escopo deve negar");
        assert_eq!(error.code(), ErrorCode::CapabilityDenied);
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
