# Evidências de execução — 2026-11-02

Log gerado por `scripts/gerar-evidencias.ps1`, com captura literal de stdout/stderr (UTF-8) no checkout `H:\projetos\rsuvm`, rustc/cargo 1.94.1, Windows. Nenhum resultado abaixo é estimado.

## cargo build --workspace --all-targets

```text
$ cargo.exe build --workspace --all-targets
[stderr]     Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.05s
exit: 0
```

## cargo test --workspace

```text
$ cargo.exe test --workspace

running 4 tests
test tests::codificacao_de_unidade_segue_o_layout_byte_a_byte ... ok
test tests::encode_e_deterministico_em_duas_chamadas ... ok
test tests::round_trip_reproduz_os_fixtures ... ok
test tests::prefixos_truncados_nunca_produzem_modulo ... ok

test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s


running 14 tests
test arquivo_vazio_e_cabecalho_truncado_sao_truncated_input ... ok
test bytes_excedentes_sao_invalid_module ... ok
test const_bool_fora_de_zero_um_e_invalid_number ... ok
test contagens_gigantes_sao_truncated_input_sem_alocar ... ok
test entrada_exatamente_no_limite_e_aceita ... ok
test entrada_maior_que_o_limite_e_input_too_large_antes_do_corpo ... ok
test fixture_minima_decodifica ... ok
test magic_invalido_e_syntax_error ... ok
test modulo_estruturalmente_invalido_e_invalid_module ... ok
test nome_com_byte_invalido_e_invalid_module ... ok
test opcode_desconhecido_e_unknown_opcode ... ok
test tag_de_tipo_desconhecida_e_unknown_type ... ok
test versao_e_flags_invalidas_sao_unsupported_version ... ok
test mutacao_xorshift_seed_2545f4914f6cdd1d_10_modulos_200_mutacoes ... ok

test result: ok. 14 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s


running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s


running 27 tests
test argumento_malformado_e_erro_de_uso ... ok
test arquivo_inexistente_e_erro_de_io ... ok
test check_aceita_modulo_valido ... ok
test fmt_imprime_forma_canonica ... ok
test fuel_insuficiente_sai_com_codigo_de_recurso ... ok
test host_com_escopo_arquivo_e_erro_de_uso ... ok
test host_com_escopo_inexistente_e_erro_de_uso ... ok
test encode_e_run_bc_preservam_host_read_file ... ok
test encode_e_run_bc_preservam_o_resultado ... ok
test host_com_escopo_relativo_e_erro_de_uso ... ok
test host_com_escopo_vazio_e_erro_de_uso ... ok
test host_com_flag_le_dentro_do_escopo ... ok
test host_com_flag_nega_arquivo_fora_do_escopo ... ok
test host_com_flag_nega_escapada_com_parent_dir ... ok
test host_com_memoria_insuficiente_para_reserva_sai_com_codigo_de_recurso ... ok
test host_com_parent_dir_no_escopo_e_erro_de_uso ... ok
test host_sem_flag_e_capability_denied ... ok
test opcao_de_capability_nao_existe ... ok
test overflow_sai_com_codigo_de_trap ... ok
test host_com_flag_nega_junction_para_fora_do_escopo ... ok
test run_bc_truncado_sai_com_codigo_de_modulo ... ok
test run_soma_imprime_valor_e_sai_com_zero ... ok
test tipo_invalido_sai_com_codigo_de_modulo ... ok
test examples_sem_host_executam ... ok
test profundidade_invalida_e_erro_de_uso ... ok
test version_e_help_saem_com_zero ... ok
test examples_verificam ... ok

test result: ok. 27 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.38s


running 3 tests
test memoria_insuficiente_sai_com_codigo_de_recurso ... ok
test memoria_malformada_e_erro_de_uso ... ok
test memoria_suficiente_executa ... ok

test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.03s


running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s


running 3 tests
test corpus_invalido_e_rejeitado_antes_da_execucao ... ok
test ids_do_corpus_sao_unicos_entre_diretorios ... ok
test corpus_valido_executa_e_reproduz ... ok

test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s


running 2 tests
test corpus_invalido_cobre_os_codigos_de_erro_centrais ... ok
test corpus_valido_cobre_todas_as_operacoes_do_subset ... ok

test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s


running 4 tests
test campanha_bytecode_nao_causa_panico ... ok
test campanha_verificador_nao_causa_panico ... ok
test campanha_texto_nao_causa_panico ... ok
test campanha_linker_nao_causa_panico ... ok

test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.44s


running 6 tests
test chamada_inter_modulo_e_linkada_verificada_e_executada ... ok
test externa_sem_provedor_falha_com_invalid_module ... ok
test modulo_linkado_sobrevive_ao_diferencial_de_bytecode ... ok
test nome_de_modulo_duplicado_falha ... ok
test provedor_ambiguo_falha_com_invalid_module ... ok
test verificador_rejeita_modulo_com_externa_nao_linkada ... ok

test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s


running 8 tests
test capabilities_sao_negadas_em_todos_os_efeitos ... ok
test fuel_e_debitado_antes_do_despacho_e_nao_reseta_em_chamada ... ok
test limite_de_entrada_e_aplicado_antes_do_parse ... ok
test unidade_e_comparacao_tem_tipos_estaveis ... ok
test bytecode_truncado_em_qualquer_prefixo_falha_sem_panico ... ok
test reexecucao_repetida_nao_compartilha_estado ... ok
test mutacao_de_bytes_nao_causa_panico ... ok
test mutacao_textual_nao_causa_panico ... ok

test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.25s


running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s


running 14 tests
test argumentos_invalidos_sao_rejeitados_antes_do_fuel ... ok
test comparacao_produz_bool ... ok
test constante_consome_duas_unidades_de_fuel ... ok
test desvio_verdadeiro_segue_apenas_o_bloco_entao ... ok
test entrada_inexistente_e_invalid_module ... ok
test fuel_zero_falha_na_primeira_instrucao ... ok
test execucao_repetida_e_deterministica ... ok
test fuel_zero_falha_no_terminador_de_funcao_sem_instrucoes ... ok
test loop_infinito_esgota_fuel ... ok
test merge_copia_argumentos_para_os_parametros_do_bloco ... ok
test overflow_e_trap_com_localizacao_da_instrucao ... ok
test recursao_excede_profundidade_e_fuel_cresce_por_frame ... ok
test soma_atravessa_chamada_e_soma_fuel ... ok
test unidade_retorna_unit_com_fuel_um ... ok

test result: ok. 14 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s


running 7 tests
test execute_sem_host_nega_qualquer_chamada ... ok
test host_com_aridade_errada_e_invalid_type ... ok
test host_com_capability_e_escopo_correto_executa ... ok
test host_com_escopo_fora_da_concessao_e_negado ... ok
test host_max_bytes_grande_e_limitado_ao_teto_da_abi ... ok
test host_nao_e_chamado_quando_a_reserva_nao_cabe ... ok
test host_sem_capability_e_negado_antes_do_backend ... ok

test result: ok. 7 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s


running 6 tests
test buffer_de_argumentos_de_desvio_conta_na_quota ... ok
test concatenacao_respeita_o_limite_de_string ... ok
test frame_exato_cabe_e_registra_pico ... ok
test memoria_zero_falha_antes_da_primeira_instrucao ... ok
test soma_ocupa_dois_frames_simultaneos_e_buffer_de_chamada ... ok
test um_byte_menos_falha_antes_de_alocar ... ok

test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s


running 28 tests
test structure::tests::aceita_declaracao_externa_sem_corpo ... ok
test structure::tests::aceita_limite_estrutural_exato ... ok
test structure::tests::aceita_modulos_validos ... ok
test structure::tests::construtor_de_parametros_aceita_i64 ... ok
test structure::tests::nomes_conhecidos_sao_identificadores ... ok
test structure::tests::rejeita_alvo_de_desvio_inexistente ... ok
test structure::tests::rejeita_argumentos_de_chamada_acima_do_limite ... ok
test structure::tests::rejeita_argumentos_de_desvio_acima_do_limite ... ok
test structure::tests::rejeita_bloco_de_entrada_com_parametros ... ok
test structure::tests::rejeita_blocos_acima_do_limite ... ok
test structure::tests::rejeita_chamada_para_funcao_inexistente ... ok
test structure::tests::rejeita_entrada_externa ... ok
test structure::tests::rejeita_entry_inexistente ... ok
test structure::tests::rejeita_id_de_bloco_nao_denso ... ok
test structure::tests::rejeita_identificador_invalido ... ok
test structure::tests::rejeita_nome_duplicado_de_funcao ... ok
test structure::tests::rejeita_nome_reservado ... ok
test structure::tests::rejeita_operando_inexistente ... ok
test structure::tests::rejeita_parametros_de_bloco_acima_do_limite ... ok
test structure::tests::rejeita_parametros_de_funcao_acima_do_limite ... ok
test structure::tests::rejeita_string_acima_do_limite ... ok
test structure::tests::rejeita_valor_repetido ... ok
test structure::tests::rejeita_valores_acima_do_limite ... ok
test structure::tests::rejeita_versao_desconhecida ... ok
test structure::tests::respeita_limite_de_funcoes_antes_de_alocar ... ok
test structure::tests::respeita_limite_de_instrucoes_por_bloco ... ok
test tests::ids_novos_sao_densos_e_unicos ... ok
test tests::modulo_de_teste_passa_pela_checagem_estrutural ... ok

test result: ok. 28 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s


running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s


running 13 tests
test tests::bytes_retidos_do_valor_sao_recursivos ... ok
test tests::capability_concedida_cobre_o_escopo_e_subcaminhos ... ok
test tests::capability_nao_casa_prefixo_textual_de_componente ... ok
test tests::capability_nega_caminho_com_parent_dir ... ok
test tests::configuracao_carrega_limite_de_memoria ... ok
test tests::configuracao_padrao_tem_fuel_e_profundidade ... ok
test tests::custo_de_buffer_de_argumentos_e_modelado_e_satura ... ok
test tests::custo_de_frame_e_modelado_e_satura ... ok
test tests::escopo_vazio_concede_qualquer_caminho ... ok
test tests::host_api_request_permite_escopo_concedido ... ok
test tests::nenhuma_capability_e_concedida_por_padrao ... ok
test tests::profundidade_zero_ou_acima_do_teto_e_rejeitada ... ok
test tests::valores_conhecem_seu_tipo ... ok

test result: ok. 13 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s


running 7 tests
test tests::aritmetica_verificada_nao_transborda_em_silencio ... ok
test tests::comparacoes_sao_assinadas ... ok
test tests::diagnostico_exibe_codigo_e_local ... ok
test tests::limites_padrao_sao_ordens_de_grandeza_pequenas ... ok
test tests::nomes_de_tipo_e_operacao_sao_bidirecionais ... ok
test tests::operacoes_f64_sao_bidirecionais_e_ieee ... ok
test tests::tabela_de_erros_e_estavel ... ok

test result: ok. 7 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s


running 13 tests
test tests::aceita_funcao_unit_e_chamada_sem_argumentos ... ok
test tests::alvo_inexistente_e_dominio_de_nomes ... ok
test tests::bloco_sem_terminador_ou_terminador_fora_de_posicao ... ok
test tests::entradas_hostis_nao_entram_em_panico ... ok
test tests::erros_de_decode_tem_codigos_estaveis ... ok
test tests::import_declara_funcao_externa ... ok
test tests::limite_de_entrada_e_respeitado ... ok
test tests::imprime_modulos_construidos_por_api ... ok
test tests::parseia_o_exemplo_minimo ... ok
test tests::round_trip_do_host_read_file ... ok
test tests::round_trip_do_merge ... ok
test tests::round_trip_preserva_estrutura_e_ids ... ok
test tests::string_literal_round_trip ... ok

test result: ok. 13 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s


running 17 tests
test tests::entrada_com_parametros_verifica ... ok
test tests::modulos_de_referencia_verificam ... ok
test tests::propaga_operando_inexistente_como_invalid_module ... ok
test tests::recursao_verifica ... ok
test tests::rejeita_argumento_de_chamada_com_tipo_errado ... ok
test tests::rejeita_aridade_de_argumento_de_bloco ... ok
test tests::rejeita_aridade_de_chamada ... ok
test tests::rejeita_aritmetica_com_operando_nao_i64 ... ok
test tests::rejeita_bloco_inalcancavel ... ok
test tests::rejeita_comparacao_com_operando_nao_i64 ... ok
test tests::rejeita_condicao_de_desvio_i64 ... ok
test tests::rejeita_retorno_com_valor_em_funcao_unit ... ok
test tests::rejeita_retorno_sem_valor_em_funcao_i64 ... ok
test tests::rejeita_tipo_de_argumento_de_bloco ... ok
test tests::rejeita_tipo_de_retorno_divergente ... ok
test tests::rejeita_uso_antes_da_definicao_no_mesmo_bloco ... ok
test tests::rejeita_valor_de_bloco_nao_dominante ... ok

test result: ok. 17 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s


running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s


running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s


running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s


running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s


running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s


running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s


running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s


running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s


running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
[stderr]     Finished `test` profile [unoptimized + debuginfo] target(s) in 0.04s
     Running unittests src\lib.rs (target\debug\deps\rvm_bytecode-d86c05318e01b21a.exe)
     Running tests\hostile.rs (target\debug\deps\hostile-6e56be861074a7c3.exe)
     Running unittests src\main.rs (target\debug\deps\rsuvm-d936d5c9516154d8.exe)
     Running tests\cli.rs (target\debug\deps\cli-b0b6202274c8ef73.exe)
     Running tests\memoria.rs (target\debug\deps\memoria-416cf1c5a9260892.exe)
     Running unittests src\lib.rs (target\debug\deps\rvm_conformance-62e17f42f080b9f7.exe)
     Running tests\conformance.rs (target\debug\deps\conformance-f2b586cd75d7347c.exe)
     Running tests\coverage.rs (target\debug\deps\coverage-3daf6de255832d86.exe)
     Running tests\fuzzing.rs (target\debug\deps\fuzzing-89c8c59f2e82c8c2.exe)
     Running tests\linking.rs (target\debug\deps\linking-216df9109dcd0ec3.exe)
     Running tests\properties.rs (target\debug\deps\properties-e1a5ca2206cf955e.exe)
     Running unittests src\lib.rs (target\debug\deps\rvm_interpreter-d1e3324bddf64a24.exe)
     Running tests\execucao.rs (target\debug\deps\execucao-4ed78bab9b4e807f.exe)
     Running tests\host.rs (target\debug\deps\host-fac9e33df84e256f.exe)
     Running tests\memoria.rs (target\debug\deps\memoria-3bd79b9b0189d345.exe)
     Running unittests src\lib.rs (target\debug\deps\rvm_ir-73bfa9b4790c9779.exe)
     Running unittests src\lib.rs (target\debug\deps\rvm_link-1f0416df8ca3f5ea.exe)
     Running unittests src\lib.rs (target\debug\deps\rvm_runtime-6465cb35290e26f9.exe)
     Running unittests src\lib.rs (target\debug\deps\rvm_spec-e3748bacbcbd4c54.exe)
     Running unittests src\lib.rs (target\debug\deps\rvm_text-0c89754f8783992a.exe)
     Running unittests src\lib.rs (target\debug\deps\rvm_verify-c50989e455e4bf6c.exe)
   Doc-tests rvm_bytecode
   Doc-tests rvm_conformance
   Doc-tests rvm_interpreter
   Doc-tests rvm_ir
   Doc-tests rvm_link
   Doc-tests rvm_runtime
   Doc-tests rvm_spec
   Doc-tests rvm_text
   Doc-tests rvm_verify
exit: 0
```

## cargo clippy --workspace --all-targets -- -D warnings

```text
$ cargo.exe clippy --workspace --all-targets -- -D warnings
[stderr]     Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.12s
exit: 0
```

## cargo fmt --all -- --check (sem saída = formatado)

```text
$ cargo.exe fmt --all -- --check
exit: 0
```

Soma de testes verdes nos alvos: **176** (doc-tests inclusos, 0 neles).

## Corpus de conformidade

- casos válidos: **62** (C-001..C-062)
- casos inválidos: **42** (N-001..N-011, N-013..N-043; N-012 vive no decoder binário)

## CLI (`target/debug/rsuvm.exe`)

```text
$ rsuvm run examples/soma.rvm --arg i64:10 --arg i64:20
stdout: i64:30
exit: 0

$ rsuvm check examples/soma.rvm
stdout: ok: módulo `exemplo`, entrada `soma`
exit: 0

$ rsuvm run crates/rvm-conformance/corpus/invalid/N-001-tipo-de-argumento.rvm
stderr: rsuvm: invalid_type: valor v0 tem tipo 1, esperado 2 [função `main`, bloco `entry`, instrução 1]
exit: 2

$ rsuvm run crates/rvm-conformance/corpus/invalid/N-009-overflow.rvm
stderr: rsuvm: integer_overflow: `add_i64_checked` transbordou com os operandos 9223372036854775807 e 1 [função `main`, bloco `entry`, instrução 2]
exit: 3

$ rsuvm run crates/rvm-conformance/corpus/invalid/N-010-loop-sem-fim.rvm --fuel 1000
stderr: rsuvm: out_of_fuel: fuel esgotado antes do despacho (saldo inicial 1000) [função `main`, bloco `loop`]
exit: 4

$ rsuvm run crates/rvm-conformance/corpus/invalid/N-011-recursao-profunda.rvm --max-depth 8
stderr: rsuvm: call_depth_exceeded: profundidade 9 excede o limite de 8 chamada(s) aninhada(s) [função `main`, bloco `entry`, instrução 0]
exit: 4

```

## Bytecode experimental: encode + run-bc

```text
$ rsuvm encode examples/soma.rvm ARQUIVO_TEMP.bin
111 bytes em C:\Users\rapin\AppData\Local\Temp\rsuvm-evidencia-22792.bin
exit: 0

$ rsuvm run-bc ARQUIVO_TEMP.bin --arg i64:7 --arg i64:35
i64:42
exit: 0
```
