# Changelog

## 0.2.0 — candidata

- Botão Queries > 1s: somente client backend ativo com duração estritamente superior a 1 s; filtro combinado com os demais.
- Métricas SQL de instância: TPS, commits, rollbacks, cache hit por intervalo, temporários e deadlocks; WAL PG14+, I/O agregado PG16+, I/O por PID PG18+ (até 128 ativos). Sem SSH/agent/extensão.
- Taxas por tempo monotônico real, baselines por chave/reset, descarte de resets, regressões, mudanças de membros e lacunas longas; dados ausentes exibidos como —.
- Testes unitários de taxas e filtro; matriz CI SQL por versão.

## 0.1.1 — candidata

- Tema escuro com maior contraste; destaque verde >=2 s, amarelo >=5 s, vermelho >=10 s para consultas ativas de client backend.
- Menu de contexto no PID e SQL, cópia e salvamento SQL UTF-8 sem sobrescrever.
- Cancelamento sem TLS exige confirmação adicional por ação e confirmação do PID; encerramento continua exigindo TLS. Auditoria distingue transporte e revalidação de instância desde a seleção.
- Testes de limiares e transporte.

## 0.1.0 — 2026-11-26 (candidata, não validada)

- Interface egui, perfis locais, tema claro/escuro, gerenciamento simultâneo de conexões, TLS com CA adicional.
- Coleta de sessões, identificação de privilégios, filtro, ordenação, detalhes, pausa e cadeia textual de locks.
- Modo administrativo experimental com cancelamento/encerramento, confirmação de PID, revalidação e auditoria local.
- Scripts de build e CI; testes unitários e SQL de laboratório preparados, ainda não executados.
