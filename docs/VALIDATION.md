# Plano de validação 0.2.0

0.1.0: CI Windows e seis testes SQL passaram; monitoramento Windows PG16 confirmado pelo usuário. 0.1.1: CI concluída e execução confirmada pelo usuário. 0.2.0: validações abaixo a executar.

## Automáticos

- `cargo test --locked --target x86_64-pc-windows-msvc`: doze testes unitários de modelo, perfil, filtro, status e grafo de locks.
- `scripts/test-sql.sh`: SQL de identificação, atividade e teste de lock real; use somente PostgreSQL descartável, com `PGACTIVITY_TEST_DISPOSABLE=1`. CI PostgreSQL 13–18. Verificar manualmente também privilégio parcial e nomes de colunas.
- `cargo build --release --locked --target x86_64-pc-windows-msvc`: compilação do executável; CI gera artifact, não release estável.
- Revisão futura: `cargo audit` / `cargo deny` após geração do lockfile; nenhuma auditoria de dependências executada nesta entrega.

## Manual — Windows e laboratório

1. Abrir executável e cadastrar dois perfis. Conectar com superuser e `pg_monitor`, depois usuário comum; conferir indicação de visibilidade.
2. Certificado válido, CA privada válida e DNS correto devem conectar. CA desconhecida, nome incorreto, certificado expirado e servidor sem TLS devem falhar sem downgrade. Texto claro requer confirmação; somente cancelamento com confirmação de risco adicional por operação; encerramento deve permanecer desabilitado.
3. Verificar perfil/senha após reiniciar, editar endpoint, esquecer senha e remover perfil; TOML sem senha.
4. Executar `SELECT pg_sleep(30)` em outra sessão; PID, duração, wait event, SQL e filtros devem corresponder a psql.
5. Criar lock em duas sessões descartáveis. Conferir bloqueador, bloqueado e cadeias indiretas. Testar idle in transaction.
6. Em servidor de teste TLS, habilitar ações administrativas: cancelar pg_sleep e verificar SQLSTATE 57014; encerrar uma sessão de teste. Testar PID incorreto, sessão encerrada, consulta que mudou, sessão de manutenção, pausa, desconexão e ausência de TLS. Inspecionar log local. Não usar sessões de produção.
7. Desligar uma instância, conferir mensagem de dados desatualizados e reconexão após retorno. As demais conexões devem continuar respondendo.
8. Testar 1/10/20 servidores, 100/1.000/5.000 sessões, monitorar CPU/RAM/tráfego do cliente e medir custo de coleta no banco. Não force 5.000 sessões em produção. Medir execução contínua por 24 horas.
9. Windows 10/11 x64, DPI 100–200%, redimensionamento, tema claro/escuro, seleção durante atualização, pausa e ordenação. Registrar prints e logs sem segredos.

10. Verificar contraste, <2 s neutra, >=2 verde, >=5 amarela, >=10 vermelha, replicação/idle sem destaque. Usar botão direito no PID e SQL, comparar clipboard e arquivo UTF-8 com consulta capturada, verificar que não sobrescreve arquivo existente. Testar cancelamento sem TLS apenas em laboratório privado: sem checkbox não envia; com confirmação e PID correto cancela; troca de instância ou consulta aborta. Verificar auditoria e ausência de encerramento sem TLS.

11. Botão Queries > 1s: execute SELECT pg_sleep(20) em laboratório, confirme inclusão após 1 s na primeira coleta elegível e remoção após fim; idle, walsender e manutenção excluídos. Testar filtros combinados.
12. Validar TPS/commit/rollback, cache hit no intervalo, temporários, WAL e I/O contra duas amostras SQL. Confirmar N/D nas versões sem a view, — na primeira amostra/reset, comportamento em reconexão, e ausência de taxa negativa. PG18: validar I/O por PID em atividade prolongada e eventual atraso de flush. Medir impacto de coleta em 682+ processos. 
   
Critério de promoção: compilação e testes aprovados, validação visual e de segurança, resultados de carga registrados, nenhuma regressão crítica. Até lá, 0.1.0 é candidata para laboratório, não produção.
