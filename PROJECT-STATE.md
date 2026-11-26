# Estado do projeto

Versão: 0.1.1 candidata, aguardando CI e validação Windows. Repositório: `pastrello/pg_atividade`.

0.1.0: CI Windows e matriz SQL PostgreSQL 13–18 concluídos com sucesso no run 45890967866; usuário confirmou funcionamento e apresentou monitoramento de centenas de processos PG16 no Windows. PR #1 consolidada na main. Isso não certifica carga prolongada, TLS ou ações administrativas.

0.1.1: contraste do tema escuro, destaque por duração 2/5/10 s em consultas ativas de client backend, menu PID/SQL, cópia e exportação SQL individual UTF-8 sem sobrescrever, cancelamento com confirmação de PID e validação de instância; sem TLS apenas cancelamento com confirmação adicional não persistida e auditoria distinta, encerramento continua exigindo TLS. Perfis existentes preservados. Novos testes de fronteiras e transporte.

Pendente: CI desta versão, Cargo.lock reproduzível, auditoria de dependências, validação visual, TLS, credenciais, ações em laboratório e benchmarks. Não classificar como estável.
