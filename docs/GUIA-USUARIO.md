# Guia do usuário — RSUVM (EXP-0)

Este guia cobre o uso do binário `rsuvm` no alpha experimental. Leia
`SECURITY.md` e `docs/COMPATIBILIDADE.md` antes de usar em qualquer ambiente
compartilhado.

> **Escopo:** protótipo single-tenant. Não é sandbox, não isola processo/OS e não
> suporta multi-tenant. Formatos e API podem mudar sem compatibilidade.

## 1. Instalação

### A partir do código

```bash
cargo install --path crates/rvm-cli --locked
rsuvm --version
```

### Build local sem instalar

```bash
cargo build --release --locked -p rvm-cli
./target/release/rsuvm --version
```

### Binários de release

Os artefatos oficiais (quando publicados) ficam em
<https://github.com/maquineh/rsuvm/releases>. Verifique o `SHA256SUMS` antes de
executar. Plataformas testadas: Linux x86_64 e Windows x86_64; macOS não é
distribuído sem teste.

## 2. Início rápido

```bash
# Executa examples/soma.rvm com dois argumentos i64.
rsuvm run examples/soma.rvm --arg i64:10 --arg i64:20
# saída: i64:30

# Verifica sem executar.
rsuvm check examples/soma.rvm

# Formata na forma canônica.
rsuvm fmt examples/soma.rvm

# Codifica para bytecode e executa o binário.
rsuvm encode examples/soma.rvm /tmp/soma.rsvmbc
rsuvm run-bc /tmp/soma.rsvmbc --arg i64:10 --arg i64:20
```

## 3. Subcomandos

| Comando | Função |
|---|---|
| `run <arquivo.rvm>` | parse, verificação e execução do formato textual |
| `run-bc <arquivo.bin>` | decode, verificação e execução do bytecode |
| `check <arquivo.rvm>` | parse + verificação, sem executar |
| `fmt <arquivo.rvm>` | imprime a forma canônica |
| `encode <arquivo.rvm> <saida.bin>` | codifica o módulo em bytecode |
| `version` / `--version` | versão do binário |
| `help` / `--help` | ajuda |

### Opções de `run` e `run-bc`

| Opção | Efeito |
|---|---|
| `--entry <fn>` | escolhe a função de entrada |
| `--arg <tipo:valor>` | argumento; pode repetir |
| `--fuel <n>` | saldo de fuel (padrão 100 000) |
| `--max-depth <n>` | profundidade máxima (1..=256; padrão 64) |
| `--memory <bytes>` | quota de memória modelada (padrão 16 MiB) |
| `--allow-fs-read <dir>` | concede `filesystem_read` sob um diretório |

Tipos de argumento: `i64:10`, `bool:true`, `f64:1.5`, `str:texto`, `unit`.

### Códigos de saída

| Código | Significado |
|---:|---|
| 0 | sucesso |
| 1 | uso incorreto ou IO da ferramenta |
| 2 | erro de módulo (parse/decode/verificação) |
| 3 | trap semântico |
| 4 | limite de recurso |
| 5 | capability negada |

## 4. Leitura de arquivos (`--allow-fs-read`)

Sem a flag, `host_read_file` falha com `capability_denied` (saída 5). Com a flag:

- o diretório precisa ser **absoluto**, existente e um diretório;
- `..` é recusado no escopo e no caminho;
- raiz e alvo são canonicalizados; symlink/junção para fora é negado;
- apenas arquivo regular é lido;
- o teto é **1 MiB por leitura**; o intérprete reserva o pior caso na quota antes
  do IO.

Recomendações operacionais:

- conceda o menor diretório possível, de preferência **somente leitura**;
- não conceda diretório gravável por usuários não confiáveis (TOCTOU);
- não conceda diretório com hard links para arquivos sensíveis;
- não use em multi-tenant; o runtime não é fronteira de isolamento.

Exemplo:

```bash
rsuvm run examples/host_read.rvm \
  --arg "str:$PWD/examples/soma.rvm" \
  --arg i64:1024 \
  --allow-fs-read "$PWD/examples"
```

## 5. Exemplos

| Arquivo | Demonstra | Saída esperada |
|---|---|---|
| `examples/soma.rvm` | parâmetros e soma verificada | `i64:30` |
| `examples/strings.rvm` | `concat_str` | `str:RSUVM` |
| `examples/array.rvm` | array imutável e `array_get` | `i64:20` |
| `examples/struct.rvm` | struct e `field_get` | `i64:7` |
| `examples/option.rvm` | `option` e `coalesce` | `i64:42` |
| `examples/result.rvm` | `result` e `result_is_ok` | `bool:true` |
| `examples/host_read.rvm` | `host_read_file` | depende do arquivo |

Os exemplos sem host podem ser executados sem opções extras. O
`host_read.rvm` exige `--allow-fs-read` e dois argumentos.

## 6. Erros e diagnóstico

O CLI imprime `código: detalhe [localização]` em `stderr`. Os códigos estáveis
estão em `docs/CONTRATOS/ERROS-EXP-0.md`. Mensagens humanas podem mudar; não
dependa do texto, apenas do código/classe.

## 7. Limites

- Fuel: 1 por instrução/terminador despachado; não reseta em chamadas.
- Profundidade: 1..=256; frame de entrada = 1.
- Memória: modelo determinístico (`frame + slots + buffers`), não RSS.
- String: limite de concatenação (`ExecConfig::max_string_bytes`).
- Host: 1 MiB por leitura; sem quota agregada de IO/tempo/taxa.

## 8. Próximos documentos

- `docs/INCORPORACAO.md` — usar a VM como biblioteca.
- `docs/COMPATIBILIDADE.md` — versionamento e formatos.
- `docs/ARQUITETURA.md` — desenho e limites.
- `SECURITY.md` — reporte e escopo de segurança.
