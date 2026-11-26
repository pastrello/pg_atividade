# Instruções de continuidade

Leia PROJECT-STATE.md, ROADMAP.md, docs/SECURITY.md e docs/VALIDATION.md antes de alterações. Mantenha o aplicativo independente. Não exigir superusuário, agente ou extensões; não introduzir SQL arbitrário, bypass TLS, ações automáticas ou gravação de segredos em log. Priorizar Rust, coleta barata e interface responsiva. Não alegar testes não executados. Após a primeira compilação, versionar Cargo.lock. Testar em PostgreSQL descartável; nunca executar fixtures em produção. Registrar decisões, evidências e pendências em PROJECT-STATE.md e CHANGELOG.md.
