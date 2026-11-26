# Plano de validação 0.1.0

Estado inicial: código entregue sem compilação local; ambiente sem Rust, Windows e PostgreSQL. Os testes abaixo não estão marcados como aprovados.

## Automáticos

- `cargo test --locked --target x86_64-pc-windows-msvc`: cinco testes unitários de modelo, perfil, filtro, status e grafo de locks.
- `scripts/test-sql.sh`: SQL de identificação, atividade e teste de lock real; use somente PostgreSQL descartável, com `PGACTIVITY_TEST_DISPOSABLE=1`. CI PostgreSQL 13–18. Verificar manualmente também privilégio parcial e nomes de colunas.
- `cargo build --release --locked --target x86_64-pc-windows-msvc`: compilação do executável; CI gera artifact, não release estável.
- Revisão futura: `cargo audit` / `cargo deny` após geração do lockfile; nenhuma auditoria de dependências executada nesta entrega.

## Manual — Windows e laboratório

1. Abrir executável e cadastrar dois perfis. Conectar com superuser e `pg_monitor`, depois usuário comum; conferir indicação de visibilidade.
2. Certificado válido, CA privada válida e DNS correto devem conectar. CA desconhecida, nome incorreto, certificado expirado e servidor sem TLS devem falhar sem downgrade. Texto claro requer confirmação e não permite ações.
3. Verificar perfil/senha após reiniciar, editar endpoint, esquecer senha e remover perfil; TOML sem senha.
4. Executar `SELECT pg_sleep(30)` em outra sessão; PID, duração, wait event, SQL e filtros devem corresponder a psql.
5. Criar lock em duas sessões descartáveis. Conferir bloqueador, bloqueado e cadeias indiretas. Testar idle in transaction.
6. Em servidor de teste TLS, habilitar ações administrativas: cancelar pg_sleep e verificar SQLSTATE 57014; encerrar uma sessão de teste. Testar PID incorreto, sessão encerrada, consulta que mudou, sessão de manutenção, pausa, desconexão e ausência de TLS. Inspecionar log local. Não usar sessões de produção.
7. Desligar uma instância, conferir mensagem de dados desatualizados e reconexão após retorno. As demais conexões devem continuar respondendo.
8. Testar 1/10/20 servidores, 100/1.000/5.000 sessões, monitorar CPU/RAM/tráfego do cliente e medir custo de coleta no banco. Não force 5.000 sessões em produção. Medir execução contínua por 24 horas.
9. Windows 10/11 x64, DPI 100–200%, redimensionamento, tema claro/escuro, seleção durante atualização, pausa e ordenação. Registrar prints e logs sem segredos.

Critério de promoção: compilação e testes aprovados, validação visual e de segurança, resultados de carga registrados, nenhuma regressão crítica. Até lá, 0.1.0 é candidata para laboratório, não produção.
