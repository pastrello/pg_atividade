# PGActivity-RS 0.2.0 — candidato a teste

Monitor desktop Windows x64 para PostgreSQL, escrito em Rust (egui/eframe, Tokio e tokio-postgres). Independente do DBOSS e sem agente ou extensão obrigatória. O código foi elaborado para PostgreSQL 13–18; A versão 0.1.0 passou na CI Windows e SQL PG13–18 e foi testada pelo usuário; a 0.1.1 também passou na CI e rodou no Windows; a 0.2.0 requer nova validação.

## Compilar no Windows

1. Instale Rust stable x64 MSVC pelo site oficial https://rustup.rs/ e Visual Studio Build Tools com o workload **Desktop development with C++**, Windows SDK e MSVC.
2. Extraia o ZIP. Abra PowerShell no diretório do projeto.
3. Execute:

```powershell
powershell.exe -ExecutionPolicy Bypass -File .\scripts\build.ps1
```

O script gera Cargo.lock se ausente, executa testes unitários e compila em release. Resultado: `dist\pgactivity.exe` e `dist\pgactivity.exe.sha256`. Não requer instalação de Python, Node.js ou libpq. Dependências Rust precisam ser obtidas na primeira compilação. O runtime MSVC e driver gráfico compatível podem ser necessários conforme o Windows. Não há binário pré-compilado neste pacote.

Para obter pelo GitHub Actions, publique o projeto em um repositório novo, habilite Actions e execute **Windows x64 build**. Baixe o artifact `pgactivity-windows-x64` somente após o job passar. O workflow também testa SQL em PostgreSQL 13–18 com contêineres descartáveis. Após primeira compilação, faça commit de Cargo.lock para builds reproduzíveis. O código não é assinado digitalmente.

## Usar

- Abra `pgactivity.exe`, crie um perfil com host, porta, banco, usuário, senha e intervalo (2 s por padrão).
- Pode usar `postgres` ou outro superusuário, sem mudanças no servidor. Também funciona com `pg_monitor`; um login comum poderá ter visibilidade parcial.
- Por padrão exige TLS e valida cadeia e nome do servidor, utilizando CAs do sistema e uma CA PEM adicional opcional. Para certificados internos, use o DNS presente no certificado e informe a CA emissora. Não há botão de ignorar certificado inválido.
- Sem TLS: permitido somente mediante confirmação em laboratório; não grava novas credenciais desse perfil e não permite encerrar sessões. Cancelamento é excepcional: exige autorização adicional a cada operação e deve ser limitado a rede privada/VPN confiável. Evite superusuário sem TLS.
- Selecione Conectar. Adicione outros perfis para monitoramento simultâneo; escolha o servidor na barra lateral. Perfis são armazenados em `%LOCALAPPDATA%\PGActivity-RS\profiles.toml`. Senhas só são persistidas com opção explícita, no Windows Credential Manager, quando o usuário conecta.
- Clique no cabeçalho para ordenar, filtre por PID, SQL, usuário, IP, wait event, etc. Clique no PID para detalhes e cadeia de locks. F5 solicita atualização; Espaço congela/retoma a visualização quando não estiver digitando. A coleta continua enquanto pausado.
- As ações administrativas estão implementadas, mas são experimentais. **Teste em laboratório antes de utilizar em produção.** Exigem habilitação explícita, privilégios suficientes, dados atuais e sessão elegível; digite o PID para confirmar. Sem TLS, apenas cancelamento com confirmação adicional por operação; encerrar exige TLS validado. Cancelar afeta a consulta atual; encerrar afeta a sessão e pode causar rollback. O aplicativo volta ao modo monitor após solicitar a ação. O resultado positivo significa sinal aceito, não confirmação de encerramento.
- Clique com o botão direito no PID ou SQL (ou selecione a linha) para copiar, salvar SQL capturado em arquivo UTF-8 sem sobrescrever ou cancelar. A exportação não executa SQL, não é automática e pode conter dados sensíveis. O destaque 2/5/10 s se aplica apenas a consultas ativas de client backend; verde, amarelo e vermelho, respectivamente.
- Desconectar interrompe o worker. Ações administrativas já enviadas não podem ser desfeitas.

## Métricas e filtro (0.2.0)

- Botão **Queries > 1s**: mostra apenas consultas de clientes em estado active e duração > 1 s; desaparecem após terminar, na próxima amostra. Inclui espera por lock/I/O; não inclui replicação, manutenção ou idle. Combina com filtros existentes.
- PostgreSQL 13+: TPS, commits/rollbacks, cache hit no intervalo, leitura de blocos estimada, temporários e deadlocks (todos os bancos da instância).
- PostgreSQL 14+: WAL gerado/s; 16–17: I/O agregado de relações e temporárias via pg_stat_io (op_bytes); 18+: bytes nativos e I/O por PID ativo, até 128 processos priorizados por query_start, quando houver visibilidade. Escrita inclui extensão; WAL separado, não somar as métricas. I/O por PID mede o backend, não exclusivamente a consulta atual.
- Taxas calculadas entre duas amostras, coleta extra a cada pelo menos 5 s; primeira amostra, resets, regressões, dados não disponíveis e lacunas longas mostram —, não zero inventado. Contadores podem atualizar com atraso. Não representam disco físico, CPU, RAM ou upload/download.
- Nenhuma mudança de configuração, SSH, extensão ou agente.

## Usuário alternativo (opcional)

Como DBA, execute em psql:

```sql
CREATE ROLE pgactivity_monitor LOGIN NOSUPERUSER NOCREATEDB NOCREATEROLE NOREPLICATION;
GRANT pg_monitor TO pgactivity_monitor;
ALTER ROLE pgactivity_monitor SET default_transaction_read_only = on;
ALTER ROLE pgactivity_monitor SET statement_timeout = '5s';
ALTER ROLE pgactivity_monitor SET lock_timeout = '1s';
```

Defina a senha interativamente com `\password pgactivity_monitor`. Conceda CONNECT ao banco se não houver acesso. `pg_monitor` permite leitura de dados sensíveis das sessões. Para operações administrativas, privilégios devem ser concedidos separadamente, se desejado. O servidor sempre decide a autorização final.

## Limites da primeira versão

- Coleta `pg_stat_activity` e `pg_blocking_pids()`; no máximo 5.000 processos por amostra, texto SQL até 8.192 caracteres (também limitado pelo servidor). Não confundir sessão ativa com consumo de CPU. Contadores representam processos presentes na amostra, não `max_connections`.
- Locks consultados apenas para sessões em espera do tipo Lock. PID 0 pode representar transação preparada. A árvore é textual, não um grafo visual.
- Não há métricas de host, top queries, replicação detalhada, histórico, exportação em lote, organização em pastas, ou instalador ainda. Essas funções estão no roadmap.
- Cada perfil conectado usa uma conexão persistente. Timeout de consulta 5 s no servidor / 7 s cliente; reconexão progressiva 2–60 s. Não há coleta em transações longas.
- Não há testes de carga ou validação visual executados. Compatibilidade Windows 10/11 x64 e PostgreSQL 13–18 é alvo, não certificação. PostgreSQL 13/14 são legados fora do suporte upstream nesta data.

Ver [SECURITY](docs/SECURITY.md), [VALIDATION](docs/VALIDATION.md), [PROJECT-STATE](PROJECT-STATE.md) e [ROADMAP](ROADMAP.md).
