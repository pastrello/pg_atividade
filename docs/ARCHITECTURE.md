# Arquitetura 0.1.0

- `src/main.rs`: egui/eframe, gerenciamento de perfis, sessão selecionada, filtros, tabela virtualizada, confirmação de ações.
- `src/model.rs`: modelo de dados, configuração TOML, Windows Credential Manager e cadeia de bloqueios com proteção contra ciclos.
- `src/collector.rs`: Tokio, conexão persistente por perfil, TLS Rustls, snapshot imutável mais recente, reconexão com backoff, conexão administrativa efêmera e auditoria.
- `sql/`: queries estáticas para identificação e atividade. A consulta exclui seu próprio PID e marca amostra truncada acima de 5.000 registros. Sessões sem visibilidade não têm informação inventada.
- Uma runtime Tokio de duas threads; cada perfil tem uma tarefa independente. Snapshot compartilhado via Arc/Mutex, sem fila ilimitada. UI não espera rede PostgreSQL. Acesso a arquivo local e Credential Manager ainda ocorre na UI e deve ser medido.
- Pausa congela snapshot exibido, não interrompe monitoramento. Seleção usa PID + backend_start. Para cancelamento, query_start também é verificado. Ações não são habilitadas em dados antigos ou congelados.
- Sem armazenamento de consultas em disco; sem telemetria. Compatibilidade e performance precisam de testes.

## 0.2.0

Módulo metrics.rs e consultas versionadas: baseline monotônico por perfil e grupo; coleta adicional no mínimo a cada cinco segundos, limite de 128 PIDs ativos no PG18; dados opcionais, ausência de valores durante baseline/reset; a coleta de atividade permanece independente de falha de SQL de métrica. Não usa SSH.
