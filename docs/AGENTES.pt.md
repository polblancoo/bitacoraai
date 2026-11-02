<p align="center"><a href="https://github.com/polblancoo/Rust_tuiManager/blob/main/docs/AGENTS.md">English</a> · <a href="https://github.com/polblancoo/Rust_tuiManager/blob/main/docs/AGENTES.md">Español</a> · <b>Português</b></p>

# Guia: fazer uma IA trabalhar sozinha com o `tm-agent`

Uma IA só age enquanto algo a acorda. O **`tm-agent`** é esse «algo»: um programa pequeno que roda na **máquina onde está a pasta do projeto**, vigia o seu servidor e, quando você atribui uma tarefa à IA:

1. abre um agente (Claude Code, Gemini CLI, um script seu…) **dentro da pasta de trabalho do projeto**,
2. move a tarefa para **Em andamento** e a comenta,
3. ao terminar a deixa em **Concluído** com um resumo; se falhar, devolve para **A fazer** com o erro.

Tudo aparece ao vivo no quadro. Este guia é para **quem instala** e para **quem administra a equipe**.

```
 Quadro (servidor)  ◄── consulta a cada 20 s ──  tm-agent  ──lança──►  seu agente (claude -p …)
   A fazer → Em andamento → Concluído          (na máquina do projeto)     trabalha na pasta
```

O mesmo guia está **dentro do app**: *Configurações → Agentes de IA → Ver guia*, e ao criar uma IA aparece o comando pronto com o token dela.

![Guia do tm-agent dentro do app](https://raw.githubusercontent.com/polblancoo/Rust_tuiManager/main/docs/screenshots/guia-10-agentes.png)

## O que você precisa

- O servidor rodando e uma conta de **admin ou dono** da equipe.
- Uma máquina com a **pasta do projeto** e com o agente que quer usar instalado e com sessão iniciada (por exemplo `claude`).
- Rust (`cargo`) para instalar o `tm-agent`. Instaladores prontos: veja o [ROADMAP](https://github.com/polblancoo/Rust_tuiManager/blob/main/ROADMAP.md).

## Passo a passo

### 1. Criar a IA

Na barra lateral: **🤖 IAs → «+»**. Escolha um nome (`claude-backend`), o **modelo** (`claude:Opus`) e o papel **Membro**. Copie o **token** (`tmk_…`): é mostrado uma única vez. Se perder, clique na IA → **Novo token**.

### 2. Definir a pasta de trabalho

Com **⚙ Projeto** preencha **Pasta de trabalho** (por exemplo `/home/ana/projetos/app`) e o **repositório**. O caminho tem que **existir na máquina onde roda o `tm-agent`**. Se não, o agente avisa com um comentário na tarefa e não a pega (ou use `TM_AGENT_DIR`).

### 3. Instalar o `tm-agent`

Na máquina do projeto:

```bash
cargo install rust-tui-manager-agent
```

### 4. Escolher o comando do agente

`TM_AGENT_CMD` é o que se executa a cada tarefa, **dentro da pasta do projeto**. Recebe o texto da tarefa na variável `TM_TASK_PROMPT`. Exemplos (confira as opções da sua versão do cliente):

| Agente | `TM_AGENT_CMD` |
|---|---|
| Claude Code | `claude -p "$TM_TASK_PROMPT" --permission-mode acceptEdits` |
| Gemini CLI | `gemini -p "$TM_TASK_PROMPT"` |
| Um script próprio | `./meu-script.sh` (lê `$TM_TASK_PROMPT`, `$TM_TASK_ID`, …) |

O prompt é sempre passado por **variável de ambiente**, nunca colado na linha de comando, para que o texto de uma tarefa não possa injetar comandos no shell.

### 5. Executar

```bash
TM_URL=https://tarefas.minhaempresa.com \
TM_TOKEN=tmk_… \
TM_AGENT_CMD='claude -p "$TM_TASK_PROMPT" --permission-mode acceptEdits' \
tm-agent
```

Você verá `[tm-agent] conectado como claude-backend…`. Atribua uma tarefa a essa IA pela web: em alguns segundos passa a **Em andamento**.

### 6. Deixar como serviço

**Linux (systemd, usuário)**, `~/.config/systemd/user/tm-agent.service`:

```ini
[Unit]
Description=tm-agent (IA do painel de comando)
After=network-online.target

[Service]
Environment=TM_URL=https://tarefas.minhaempresa.com
EnvironmentFile=%h/.config/tm-agent/claude-backend.env   # TM_TOKEN=tmk_… (chmod 600)
Environment="TM_AGENT_CMD=claude -p \"$TM_TASK_PROMPT\" --permission-mode acceptEdits"
ExecStart=%h/.cargo/bin/tm-agent
Restart=on-failure
RestartSec=5

[Install]
WantedBy=default.target
```

```bash
systemctl --user daemon-reload && systemctl --user enable --now tm-agent
```

No macOS usa-se um agente `launchd` e no Windows o Agendador de Tarefas, com as mesmas variáveis de ambiente.

## Variáveis

| Variável | Para quê | Padrão |
|---|---|---|
| `TM_TOKEN` | Token da IA (obrigatório) | — |
| `TM_AGENT_CMD` | Comando que lança o agente (obrigatório) | — |
| `TM_URL` | Endereço do servidor | `http://127.0.0.1:7878` |
| `TM_AGENT_POLL` | Segundos entre consultas | `20` |
| `TM_AGENT_TIMEOUT` | Segundos máximos por tarefa (depois é cancelada) | `1800` |
| `TM_AGENT_MAX` | Tarefas em paralelo | `1` |
| `TM_AGENT_DIR` | Pasta de reserva se o projeto não tiver uma (ou não existir aqui) | — |
| `TM_AGENT_ONCE=1` | Processa o pendente e sai (útil para testar) | — |
| `TM_AGENT_TERMINAL=1` | Abre terminais web (PTY) na pasta do projeto, conectando ao servidor | — |
| `TM_AGENT_MAX_USD` | Teto em dólares por execução, lido pelo seu script (ex. `--max-budget-usd`) | — |
| `TM_ASK_TIMEOUT` | Segundos que o `tm-agent ask` espera resposta humana | `900` |

O comando também recebe `TM_TASK_ID`, `TM_TASK_TITLE`, `TM_PROJECT_ID`, `TM_URL` e `TM_TOKEN`, caso o seu agente queira comentar ou criar subtarefas pela API.

**Perguntar a uma pessoa.** Dentro de uma tarefa a IA pode executar `tm-agent ask "pergunta"`: levanta um alarme 🙋 para os donos e admins da equipe (no app e por Telegram) e espera a resposta como comentário.

Quando a IA pergunta, o servidor também **cria uma tarefa de pedido** para uma pessoa (quem criou a tarefa, se tiver a permissão **Executor**; senão o executor com menos tarefas abertas; sem executores, um admin). O comentário diz qual tarefa e que você pode responder ali mesmo; os comandos do pedido aparecem com um botão **📋 Copiar**. Veja o [guia, capítulo 10](https://github.com/polblancoo/Rust_tuiManager/blob/main/docs/GUIA.pt.md#10-quando-uma-ia-precisa-que-você-faça-algo).

**Exemplo pronto para usar.** Em [`examples/`](https://github.com/polblancoo/Rust_tuiManager/tree/main/examples) há um `run-claude.sh` com listas de comandos permitidos e proibidos (`whitelist.txt`, `blacklist.txt`) para usar como `TM_AGENT_CMD`.

## Como se comporta

- Só pega tarefas **atribuídas à sua IA** e em **A fazer**. As de pessoas ou de outras IAs, ignora.
- Se o agente termina bem → **Concluído** + comentário com o resumo. Se falha ou estoura o tempo → volta para **A fazer** + comentário com o erro, e **enquanto o `tm-agent` continuar rodando** não tenta de novo até alguém modificá-la.
- Uma IA só pode modificar as tarefas que criou ou que lhe atribuíram (regra do servidor), mesmo que o agente tente mais.
- Os **sub-agentes** que o seu agente lançar trabalham sob a identidade dessa IA; para o trabalho interno convém usar **subtarefas**.

## Segurança (leia)

- **O texto das tarefas chega ao agente como instruções.** Quem pode atribuir uma tarefa à IA pode, na prática, dar-lhe ordens. Atribua tarefas só de contas de confiança.
- Rode o agente com **permissões limitadas**: um usuário sem privilégios, a pasta do projeto e nada mais. Evite opções do tipo «pular todas as permissões».
- **Comandos proibidos**: na ficha da IA, **🚫 Comandos prohibidos** (um por linha, ex. `Bash(rm:*)`). O `tm-agent` passa a lista ao comando em `TM_DENY` e o script de exemplo a soma ao `--disallowedTools`. Se a IA for bloqueada em algo que precisa, ela avisa com um alarme 🙋 (`tm-agent ask`).
- O **token** é a chave dessa IA: guarde-o num arquivo com permissões `600`, não no repositório. Revoga-se com um clique na web.
- Uma IA **não pode** criar nem apagar equipes, convidar pessoas nem mudar papéis.

## Se algo não funciona

| Sintoma | Causa provável |
|---|---|
| `falta TM_TOKEN` / `falta TM_AGENT_CMD` | Falta definir a variável. |
| `no pude iniciar sesión con TM_TOKEN` | Token revogado ou errado, ou servidor mal escrito. |
| Comentário «a pasta de trabalho … não existe» | O caminho do projeto não existe nessa máquina: corrija em ⚙ Projeto ou use `TM_AGENT_DIR`. |
| A tarefa volta para A fazer com um erro | Veja o comentário: é a saída do agente. Corrija e toque na tarefa de novo. |
| Nada acontece ao atribuir | A tarefa está em **A fazer** e atribuída a *essa* IA? O `tm-agent` está rodando? |

## Comandos habituais e autorização total

- **📋 Comandos habituais** (ficha da IA, só admins): lista o que essa IA **pediu antes** nos pedidos 🙋 (por quantidade de vezes), mais um catálogo de comandos comuns (Git, Rust, Node, Python, arquivos, sistema). Marque e toque em **✅ Adicionar aos permitidos**. Abaixo, uma lista de comandos **perigosos** (`rm -rf`, `sudo`, `git push`, `git reset --hard`…) para somar aos **🚫 proibidos** com um clique.
- **🔒 Autorização total** (ficha da IA): «autorizo tudo, menos isto». A IA pode executar qualquer comando exceto os de **🚫 Comandos proibidos** (os da IA, os gerais e `blacklist.txt`). O proibido sempre vence. Vale a partir da próxima tarefa da IA. Antes de ativar, revise a lista de proibidos.
- Funciona com o script `run-claude.sh` (Claude Code). Se você usa **sua própria cópia** do script, adicione esta linha antes de `exec claude`: `[ "$TM_ALLOW_ALL" = "1" ] && ALLOW="Bash$(printf '\t')$ALLOW"`. O `tm-agent` passa `TM_ALLOW_ALL=1` quando a IA tem autorização total.
- **Arquivos do agente (`whitelist.txt` / `blacklist.txt`).** Valem além das listas do app e não podem ser apagados por ele. A ficha da IA mostra um quadro **📄 Os arquivos do agente também valem** com o conteúdo; **Passar para o app** copia para as listas da IA (🚫 e ✅, onde dá para editar) e o agente deixa de ler os arquivos (`TM_LISTS_FROM_APP=1`). **Voltar a usar os arquivos** desfaz. Se você usa sua própria cópia do `run-claude.sh`, pegue a versão nova em `crates/server/assets/agents/run-claude.sh`.
- **Autorizar sem rodeios.** Em um pedido 🙋 há **um botão principal**: **✅ Autorizar e deixar a IA seguir**. Autoriza para sempre tudo o que a IA pediu (comandos **e ferramentas** como `WebSearch`, `WebFetch` ou as de MCP), avisa a IA e a tarefa volta a rodar sozinha. Para uma IA não perguntar mais: **🔓 perfil confiável** (autorização total com uma lista curta de proibidos: `sudo`, `git push`, `git commit`, `rm -rf`, `git reset --hard`, `git clean`, `systemctl`, `dd`, `mkfs`, `cargo/npm publish`). Também na ficha da IA (**⚡ Perfil confiável**). Atenção: `mcp__servidor__*` permite todas as ferramentas de um servidor MCP; não existe permissão global para todos os servidores.
