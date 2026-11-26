# Estado do projeto

Versão: 0.1.0, código candidato a laboratório. Data: 2026-11-26.

Decisões: desktop Rust independente, Windows x64, sem agente/extensão; alvo PG13–18; egui/eframe; superuser suportado, pg_monitor alternativo; leitura por padrão; ações administrativas separadas com confirmação e auditoria; TLS com validação e sem bypass de certificado; monitoramento simultâneo via Tokio.

Entregue: fontes, SQL, documentação, cinco testes unitários, script PowerShell e CI para Windows e SQL em seis versões.

Pendente: Cargo.lock (gerar no primeiro build), compilação real, artifact Windows, verificação de dependências, validação visual, teste TLS, teste credenciais Windows, teste ações em laboratório, matriz real de compatibilidade e benchmarks. Nenhum teste Rust ou PostgreSQL foi executado no ambiente de autoria. Não publicar como estável.

GitHub: nome sugerido `pastrello/pg-activity-rs`; consulta retornou 404, não houve criação nem envio de arquivos. Próximo passo: criar repo vazio e enviar fontes, ou compilar localmente.

Verificações locais concluídas: parsing do manifesto TOML e do YAML de CI, inspeção estática dos fontes, `bash -n` do teste SQL e teste da trava de segurança que recusa execução sem autorização para banco descartável. Nenhuma dessas verificações substitui compilação ou testes de integração.
