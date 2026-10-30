<p align="center"><a href="https://github.com/polblancoo/Rust_tuiManager/blob/main/crates/agent/README.md">English</a> · <a href="https://github.com/polblancoo/Rust_tuiManager/blob/main/crates/agent/README.es.md">Español</a> · <b>Português</b></p>

# rust-tui-manager-agent (`tm-agent`)

> **0.6.3**: `TM_AGENT_TERMINAL=1` abre terminais web na pasta do projeto; o agente passa `TM_ALLOW` (comandos permitidos pela tarefa), informa tokens e custo reais e respeita as cotas. Licença **AGPL-3.0-only**.
>
> **0.6.2**: quando uma IA pede ajuda (`tm-agent ask`), o servidor cria uma tarefa de pedido para uma pessoa autorizada (permissão **Executor**) e o comando aparece com botão Copiar na web. Site e downloads: https://polblancoo.github.io/rust_tuiManager/

### O que o torna diferente

O que não vimos junto em nenhuma outra ferramenta (confira, e avise se achar uma):

- 🤖 **IAs como integrantes da equipe**, com papel, lista de comandos proibidos e permissões que uma pessoa dá na própria tarefa (`/permitir cargo build`).
- 🙋 **Pedidos de ajuda da IA** que chegam como tarefa para a pessoa certa.
- 💻 **Terminal SSH dentro da tarefa** (com chave), e o que foi feito fica escrito para a IA retomar dali. *(0.6.3)*
- 📊 **Portfólio de vários projetos**, com custo e horas da IA. *(custo e horas: chegam na 0.6.3)*
- 🏠 **Auto-hospedado**: seus dados e suas chaves ficam no seu servidor.

Põe uma IA para trabalhar quando uma tarefa é atribuída a ela no [painel de comando Rust TUI Manager](https://github.com/polblancoo/Rust_tuiManager): vigia o servidor, abre um agente na **pasta de trabalho do projeto**, comenta e move a tarefa para **Em andamento** e **Concluído**.

Guia completo: [docs/AGENTES.pt.md](https://github.com/polblancoo/Rust_tuiManager/blob/main/docs/AGENTES.pt.md).

```bash
TM_URL=https://tasks.example.com \
TM_TOKEN=tmk_… \
TM_AGENT_CMD='claude -p "$TM_TASK_PROMPT" --permission-mode acceptEdits' \
tm-agent
```

## Como funciona

1. Pega as tarefas `todo` atribuídas à sua conta de IA e as move para `doing`, deixando um comentário.
2. Executa o seu comando (`sh -c` / `cmd /C`) na pasta de trabalho do projeto.
3. Se der certo: tarefa para `done` com um resumo. Se falhar: volta para `todo` com o erro (não repete enquanto está rodando).
4. Se a pasta não existir, comenta e não pega a tarefa.

Dentro de uma tarefa a IA pode chamar `tm-agent ask "pergunta"`: levanta um alarme 🙋 (no app e no Telegram) e espera a resposta de uma pessoa.

## Variáveis

| Variável | Significado | Padrão |
|---|---|---|
| `TM_TOKEN` | token da conta de IA (`tmk_…`) | obrigatória |
| `TM_AGENT_CMD` | comando que faz o trabalho | obrigatória |
| `TM_URL` | endereço do servidor | `http://127.0.0.1:7878` |
| `TM_AGENT_POLL` | segundos entre consultas | 20 |
| `TM_AGENT_TIMEOUT` | segundos por tarefa | 1800 |
| `TM_AGENT_MAX` | tarefas em paralelo | 1 |
| `TM_AGENT_DIR` | pasta se o projeto não tiver | — |
| `TM_AGENT_ONCE` | `1` = uma passada e sair | — |
| `TM_AGENT_TERMINAL` | `1` = permite que um owner abra um terminal nesta máquina pela web (a conexão sai daqui: sem chaves nem portas abertas) | desligado |
| `TM_ASK_TIMEOUT` | segundos que `ask` espera resposta | 900 |

O comando recebe `TM_TASK_PROMPT`, `TM_TASK_ID`, `TM_TASK_TITLE`, `TM_PROJECT_ID`, `TM_URL` e `TM_TOKEN`.

## Licença

**AGPL-3.0-only.** Grátis para usar e auto-hospedar. Se você oferecer uma versão modificada como serviço, precisa publicar suas mudanças com a mesma licença. Há licença comercial sem essa obrigação: pergunte nas [issues](https://github.com/polblancoo/Rust_tuiManager/issues). As versões publicadas até a 0.6.2 continuam em MIT.
