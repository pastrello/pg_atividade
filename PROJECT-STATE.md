# Estado do projeto

Versão 0.2.0 candidata, repositório pastrello/pg_atividade.

0.1.0 e 0.1.1: GitHub Actions concluídos com sucesso (runs 45890967866 e 45895873059); usuário confirmou compilação e execução Windows. PRs #1 e #2 consolidadas. Não implica validação de todas as ações privilegiadas.

0.2.0: apenas coleta SQL nativa, sem SSH, agente, extensão ou mudança no servidor. Botão Queries > 1s exclui idle, replicação e manutenção. Métricas adaptadas ao server_version_num, baseline independente por conexão, proteção contra resets e erro isolado por grupo. PG18 por PID limitado a 128 consultas ativas com visibilidade de stats. Métricas sem dados suficientes exibem —. CI e validação Windows da versão pendentes; Cargo.lock e auditoria de dependências ainda pendentes. Não classificar como estável.
