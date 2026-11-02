#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing
)]
//! Testes ponta a ponta do binário `rsuvm` (códigos de saída e contrato do CLI
//! em `docs/CONTRATOS/ERROS-EXP-0.md` §5).

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

fn bin() -> Command {
    Command::new(env!("CARGO_BIN_EXE_rsuvm"))
}

fn caminho_exemplo() -> PathBuf {
    caminho_exemplo_nome("soma.rvm")
}

fn caminho_exemplo_nome(nome: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../examples")
        .join(nome)
}

fn caminho_corpus(relativo: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../rvm-conformance/corpus")
        .join(relativo)
}

fn saida(comando: &mut Command) -> Output {
    comando.output().expect("executar binário rsuvm")
}

fn texto(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).to_string()
}

#[test]
fn run_soma_imprime_valor_e_sai_com_zero() {
    let output = saida(
        bin()
            .arg("run")
            .arg(caminho_exemplo())
            .args(["--arg", "i64:10", "--arg", "i64:20"]),
    );
    assert_eq!(
        output.status.code(),
        Some(0),
        "stderr: {}",
        texto(&output.stderr)
    );
    assert_eq!(texto(&output.stdout).trim(), "i64:30");
    assert!(texto(&output.stderr).is_empty());
}

#[test]
fn check_aceita_modulo_valido() {
    let output = saida(bin().arg("check").arg(caminho_exemplo()));
    assert_eq!(
        output.status.code(),
        Some(0),
        "stderr: {}",
        texto(&output.stderr)
    );
    assert!(texto(&output.stdout).contains("ok:"));
}

#[test]
fn fmt_imprime_forma_canonica() {
    let output = saida(bin().arg("fmt").arg(caminho_exemplo()));
    assert_eq!(
        output.status.code(),
        Some(0),
        "stderr: {}",
        texto(&output.stderr)
    );
    let texto_fmt = texto(&output.stdout);
    assert!(
        texto_fmt.contains("module exemplo version 1"),
        "{texto_fmt}"
    );
    assert!(texto_fmt.contains("add_i64_checked"), "{texto_fmt}");
}

#[test]
fn tipo_invalido_sai_com_codigo_de_modulo() {
    let output = saida(
        bin()
            .arg("check")
            .arg(caminho_corpus("invalid/N-001-tipo-de-argumento.rvm")),
    );
    assert_eq!(
        output.status.code(),
        Some(2),
        "stderr: {}",
        texto(&output.stderr)
    );
    assert!(texto(&output.stderr).contains("invalid_type"));
}

#[test]
fn overflow_sai_com_codigo_de_trap() {
    let output = saida(
        bin()
            .arg("run")
            .arg(caminho_corpus("invalid/N-009-overflow.rvm")),
    );
    assert_eq!(
        output.status.code(),
        Some(3),
        "stderr: {}",
        texto(&output.stderr)
    );
    assert!(texto(&output.stderr).contains("integer_overflow"));
}

#[test]
fn fuel_insuficiente_sai_com_codigo_de_recurso() {
    let output = saida(
        bin()
            .arg("run")
            .arg(caminho_exemplo())
            .args(["--arg", "i64:1", "--arg", "i64:2", "--fuel", "1"]),
    );
    assert_eq!(
        output.status.code(),
        Some(4),
        "stderr: {}",
        texto(&output.stderr)
    );
    assert!(texto(&output.stderr).contains("out_of_fuel"));
}

#[test]
fn profundidade_invalida_e_erro_de_uso() {
    for depth in ["0", "999"] {
        let output = saida(bin().arg("run").arg(caminho_exemplo()).args([
            "--arg",
            "i64:1",
            "--arg",
            "i64:2",
            "--max-depth",
            depth,
        ]));
        assert_eq!(output.status.code(), Some(1), "depth {depth}");
        assert!(texto(&output.stderr).contains("max-depth"));
    }
}

#[test]
fn argumento_malformado_e_erro_de_uso() {
    let output = saida(
        bin()
            .arg("run")
            .arg(caminho_exemplo())
            .args(["--arg", "10", "--arg", "20"]),
    );
    assert_eq!(
        output.status.code(),
        Some(1),
        "stderr: {}",
        texto(&output.stderr)
    );
    assert!(texto(&output.stderr).contains("tipo"));
}

#[test]
fn opcao_de_capability_nao_existe() {
    let output = saida(
        bin()
            .arg("run")
            .arg(caminho_exemplo())
            .args(["--allow-filesystem", "true"]),
    );
    assert_eq!(
        output.status.code(),
        Some(1),
        "stderr: {}",
        texto(&output.stderr)
    );
    assert!(texto(&output.stderr).contains("desconhecida"));
}

#[test]
fn arquivo_inexistente_e_erro_de_io() {
    let output = saida(bin().arg("run").arg("nao-existe-rsuvm.rvm"));
    assert_eq!(output.status.code(), Some(1));
    assert!(texto(&output.stderr).contains("não foi possível ler"));
}

#[test]
fn encode_e_run_bc_preservam_o_resultado() {
    let destino = std::env::temp_dir().join(format!("rsuvm-cli-{}-encode.bin", std::process::id()));
    let encode = saida(bin().arg("encode").arg(caminho_exemplo()).arg(&destino));
    assert_eq!(
        encode.status.code(),
        Some(0),
        "stderr: {}",
        texto(&encode.stderr)
    );
    assert!(destino.exists(), "arquivo binário não foi criado");

    let run = saida(
        bin()
            .arg("run-bc")
            .arg(&destino)
            .args(["--arg", "i64:10", "--arg", "i64:20"]),
    );
    let _ = std::fs::remove_file(&destino);
    assert_eq!(run.status.code(), Some(0), "stderr: {}", texto(&run.stderr));
    assert_eq!(texto(&run.stdout).trim(), "i64:30");
}

#[test]
fn run_bc_truncado_sai_com_codigo_de_modulo() {
    let destino =
        std::env::temp_dir().join(format!("rsuvm-cli-{}-truncado.bin", std::process::id()));
    std::fs::write(&destino, b"RSVMB1\x01\x00").expect("escrever arquivo de teste");
    let output = saida(bin().arg("run-bc").arg(&destino));
    let _ = std::fs::remove_file(&destino);
    assert_eq!(
        output.status.code(),
        Some(2),
        "stderr: {}",
        texto(&output.stderr)
    );
    assert!(texto(&output.stderr).contains("truncated_input"));
}

#[test]
fn examples_verificam() {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples");
    let mut count = 0;
    for entry in std::fs::read_dir(&dir).expect("ler diretório examples") {
        let path = entry.expect("entrada de examples").path();
        if path.extension().and_then(|ext| ext.to_str()) != Some("rvm") {
            continue;
        }
        let output = saida(bin().arg("check").arg(&path));
        assert_eq!(
            output.status.code(),
            Some(0),
            "{}: {}",
            path.display(),
            texto(&output.stderr)
        );
        count += 1;
    }
    assert!(
        count >= 7,
        "esperava ao menos 7 exemplos, encontrou {count}"
    );
}

#[test]
fn examples_sem_host_executam() {
    for (nome, esperado) in [
        ("strings.rvm", "str:RSUVM"),
        ("array.rvm", "i64:20"),
        ("struct.rvm", "i64:7"),
        ("option.rvm", "i64:42"),
        ("result.rvm", "bool:true"),
    ] {
        let output = saida(bin().arg("run").arg(caminho_exemplo_nome(nome)));
        assert_eq!(
            output.status.code(),
            Some(0),
            "{nome}: {}",
            texto(&output.stderr)
        );
        assert_eq!(texto(&output.stdout).trim(), esperado, "{nome}");
    }
}

#[test]
fn version_e_help_saem_com_zero() {
    for flag in ["version", "--help"] {
        let output = saida(bin().arg(flag));
        assert_eq!(output.status.code(), Some(0));
        assert!(!texto(&output.stdout).is_empty());
    }
}

/// Módulo que chama `host_read_file`; a tabela de tipos precisa declarar
/// `result string string` (id 5) para a ABI (ADR-0036).
const MODULO_HOST: &str = "module hostcli version 1\n\
entry le\n\
type 5 = result string string\n\
\n\
fn le(path: string, max: i64) -> result string string {\n\
  entry:\n\
    v0 = host_read_file path, max\n\
    return v0\n\
}\n";

/// Cria um diretório único por teste com o módulo, um arquivo dentro e outro
/// fora de um subdiretório de escopo.
fn cenario_host(nome: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("rsuvm-cli-host-{}-{nome}", std::process::id()));
    std::fs::create_dir_all(dir.join("sub")).expect("criar diretório de teste");
    std::fs::write(dir.join("le.rvm"), MODULO_HOST).expect("escrever módulo de teste");
    std::fs::write(dir.join("dentro.txt"), "conteudo-secreto").expect("escrever arquivo dentro");
    std::fs::write(dir.join("fora.txt"), "conteudo-fora").expect("escrever arquivo fora");
    dir
}

#[test]
fn host_sem_flag_e_capability_denied() {
    let dir = cenario_host("sem-flag");
    let output = saida(bin().arg("run").arg(dir.join("le.rvm")).args([
        "--arg",
        &format!("str:{}", dir.join("dentro.txt").display()),
        "--arg",
        "i64:100",
    ]));
    let _ = std::fs::remove_dir_all(&dir);
    assert_eq!(
        output.status.code(),
        Some(5),
        "stderr: {}",
        texto(&output.stderr)
    );
    assert!(texto(&output.stderr).contains("capability_denied"));
}

#[test]
fn host_com_flag_le_dentro_do_escopo() {
    let dir = cenario_host("dentro");
    let output = saida(
        bin()
            .arg("run")
            .arg(dir.join("le.rvm"))
            .args([
                "--arg",
                &format!("str:{}", dir.join("dentro.txt").display()),
                "--arg",
                "i64:100",
                "--allow-fs-read",
            ])
            .arg(&dir),
    );
    let _ = std::fs::remove_dir_all(&dir);
    assert_eq!(
        output.status.code(),
        Some(0),
        "stderr: {}",
        texto(&output.stderr)
    );
    assert_eq!(texto(&output.stdout).trim(), "ok:str:conteudo-secreto");
}

#[test]
fn host_com_flag_nega_arquivo_fora_do_escopo() {
    let dir = cenario_host("fora");
    let output = saida(
        bin()
            .arg("run")
            .arg(dir.join("le.rvm"))
            .args([
                "--arg",
                &format!("str:{}", dir.join("fora.txt").display()),
                "--arg",
                "i64:100",
                "--allow-fs-read",
            ])
            .arg(dir.join("sub")),
    );
    let _ = std::fs::remove_dir_all(&dir);
    assert_eq!(
        output.status.code(),
        Some(5),
        "stderr: {}",
        texto(&output.stderr)
    );
    assert!(texto(&output.stderr).contains("capability_denied"));
}

#[test]
fn host_com_flag_nega_escapada_com_parent_dir() {
    let dir = cenario_host("parent-dir");
    let escape = dir.join("sub").join("..").join("fora.txt");
    let output = saida(
        bin()
            .arg("run")
            .arg(dir.join("le.rvm"))
            .args([
                "--arg",
                &format!("str:{}", escape.display()),
                "--arg",
                "i64:100",
                "--allow-fs-read",
            ])
            .arg(dir.join("sub")),
    );
    let _ = std::fs::remove_dir_all(&dir);
    assert_eq!(
        output.status.code(),
        Some(5),
        "stderr: {}",
        texto(&output.stderr)
    );
    assert!(texto(&output.stderr).contains("capability_denied"));
}

#[test]
fn host_com_escopo_vazio_e_erro_de_uso() {
    let output = saida(bin().arg("run").arg(caminho_exemplo()).args([
        "--arg",
        "i64:10",
        "--arg",
        "i64:20",
        "--allow-fs-read",
        "",
    ]));
    assert_eq!(
        output.status.code(),
        Some(1),
        "stderr: {}",
        texto(&output.stderr)
    );
    assert!(texto(&output.stderr).contains("allow-fs-read"));
}

#[test]
fn host_com_escopo_relativo_e_erro_de_uso() {
    let output = saida(bin().arg("run").arg(caminho_exemplo()).args([
        "--arg",
        "i64:10",
        "--arg",
        "i64:20",
        "--allow-fs-read",
        "relativo",
    ]));
    assert_eq!(
        output.status.code(),
        Some(1),
        "stderr: {}",
        texto(&output.stderr)
    );
    assert!(texto(&output.stderr).contains("absoluto"));
}

#[test]
fn host_com_parent_dir_no_escopo_e_erro_de_uso() {
    let dir = cenario_host("escopo-parent");
    let output = saida(
        bin()
            .arg("run")
            .arg(dir.join("le.rvm"))
            .args([
                "--arg",
                &format!("str:{}", dir.join("dentro.txt").display()),
                "--arg",
                "i64:100",
                "--allow-fs-read",
            ])
            .arg(dir.join("sub").join("..")),
    );
    let _ = std::fs::remove_dir_all(&dir);
    assert_eq!(
        output.status.code(),
        Some(1),
        "stderr: {}",
        texto(&output.stderr)
    );
    assert!(texto(&output.stderr).contains(".."));
}

#[test]
fn host_com_escopo_inexistente_e_erro_de_uso() {
    let dir = cenario_host("escopo-inexistente");
    let output = saida(
        bin()
            .arg("run")
            .arg(dir.join("le.rvm"))
            .args([
                "--arg",
                &format!("str:{}", dir.join("dentro.txt").display()),
                "--arg",
                "i64:100",
                "--allow-fs-read",
            ])
            .arg(dir.join("nao-existe")),
    );
    let _ = std::fs::remove_dir_all(&dir);
    assert_eq!(
        output.status.code(),
        Some(1),
        "stderr: {}",
        texto(&output.stderr)
    );
    assert!(texto(&output.stderr).contains("canonical"));
}

#[test]
fn host_com_escopo_arquivo_e_erro_de_uso() {
    let dir = cenario_host("escopo-arquivo");
    let output = saida(
        bin()
            .arg("run")
            .arg(dir.join("le.rvm"))
            .args([
                "--arg",
                &format!("str:{}", dir.join("dentro.txt").display()),
                "--arg",
                "i64:100",
                "--allow-fs-read",
            ])
            .arg(dir.join("dentro.txt")),
    );
    let _ = std::fs::remove_dir_all(&dir);
    assert_eq!(
        output.status.code(),
        Some(1),
        "stderr: {}",
        texto(&output.stderr)
    );
    assert!(texto(&output.stderr).contains("diretório"));
}

#[test]
fn host_com_memoria_insuficiente_para_reserva_sai_com_codigo_de_recurso() {
    let dir = cenario_host("reserva");
    let output = saida(
        bin()
            .arg("run")
            .arg(dir.join("le.rvm"))
            .args([
                "--arg",
                &format!("str:{}", dir.join("dentro.txt").display()),
                "--arg",
                "i64:1048576",
                "--memory",
                "2000",
                "--allow-fs-read",
            ])
            .arg(&dir),
    );
    let _ = std::fs::remove_dir_all(&dir);
    assert_eq!(
        output.status.code(),
        Some(4),
        "stderr: {}",
        texto(&output.stderr)
    );
    assert!(texto(&output.stderr).contains("out_of_memory"));
}

#[cfg(unix)]
#[test]
fn host_com_flag_nega_symlink_para_fora_do_escopo() {
    use std::os::unix::fs::symlink;

    let dir = cenario_host("symlink");
    let fora =
        std::env::temp_dir().join(format!("rsuvm-cli-fora-{}-symlink.txt", std::process::id()));
    std::fs::write(&fora, "segredo").expect("escrever arquivo fora");
    symlink(&fora, dir.join("link.txt")).expect("criar symlink");
    let output = saida(
        bin()
            .arg("run")
            .arg(dir.join("le.rvm"))
            .args([
                "--arg",
                &format!("str:{}", dir.join("link.txt").display()),
                "--arg",
                "i64:100",
                "--allow-fs-read",
            ])
            .arg(&dir),
    );
    let _ = std::fs::remove_dir_all(&dir);
    let _ = std::fs::remove_file(&fora);
    assert_eq!(
        output.status.code(),
        Some(5),
        "stderr: {}",
        texto(&output.stderr)
    );
    assert!(texto(&output.stderr).contains("capability_denied"));
}

#[cfg(windows)]
#[test]
fn host_com_flag_nega_junction_para_fora_do_escopo() {
    let dir = cenario_host("junction");
    let link = dir.join("sub").join("link");
    let status = std::process::Command::new("cmd")
        .args(["/C", "mklink", "/J"])
        .arg(&link)
        .arg(&dir)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .expect("executar mklink /J");
    assert!(status.success(), "mklink /J falhou");
    let output = saida(
        bin()
            .arg("run")
            .arg(dir.join("le.rvm"))
            .args([
                "--arg",
                &format!("str:{}", link.join("fora.txt").display()),
                "--arg",
                "i64:100",
                "--allow-fs-read",
            ])
            .arg(dir.join("sub")),
    );
    let _ = std::fs::remove_dir(&link);
    let _ = std::fs::remove_dir_all(&dir);
    assert_eq!(
        output.status.code(),
        Some(5),
        "stderr: {}",
        texto(&output.stderr)
    );
    assert!(texto(&output.stderr).contains("capability_denied"));
}

#[test]
fn encode_e_run_bc_preservam_host_read_file() {
    let dir = cenario_host("roundtrip");
    let fonte = dir.join("le.rvm");
    let destino = dir.join("le.bin");
    let encode = saida(bin().arg("encode").arg(&fonte).arg(&destino));
    assert_eq!(
        encode.status.code(),
        Some(0),
        "stderr: {}",
        texto(&encode.stderr)
    );
    let run = saida(
        bin()
            .arg("run-bc")
            .arg(&destino)
            .args([
                "--arg",
                &format!("str:{}", dir.join("dentro.txt").display()),
                "--arg",
                "i64:100",
                "--allow-fs-read",
            ])
            .arg(&dir),
    );
    let _ = std::fs::remove_dir_all(&dir);
    assert_eq!(run.status.code(), Some(0), "stderr: {}", texto(&run.stderr));
    assert_eq!(texto(&run.stdout).trim(), "ok:str:conteudo-secreto");
}
