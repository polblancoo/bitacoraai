<p align="center"><img src="https://raw.githubusercontent.com/polblancoo/Rust_tuiManager/main/docs/logo/logo-512.png" width="140" alt="Logo do Rust TUI Manager"></p>

<p align="center"><a href="https://github.com/polblancoo/Rust_tuiManager/blob/main/README.md">English</a> · <a href="https://github.com/polblancoo/Rust_tuiManager/blob/main/README.es.md">Español</a> · <b>Português</b></p>

# Rust TUI Manager

## 🎛️ Painel de comando para os seus projetos

Dirija **vários projetos ao mesmo tempo, a partir de um só lugar**, com **pessoas e agentes de IA trabalhando no mesmo quadro**. Um CTO, uma gerente de operações ou uma dona de agência monta os projetos, atribui recursos (humanos e IAs de diferentes modelos), delega cada um a um líder e **vê como tudo vai andando, momento a momento**.

![Painel de comando: projetos, IAs e tarefas da equipe](https://raw.githubusercontent.com/polblancoo/Rust_tuiManager/main/docs/screenshots/guia-01-tablero.png)

**Um projeto pode ser o que você precisar:**

| Exemplo | Como fica |
|---|---|
| 🏭 **Fábrica de apps** | Um app por projeto; as IAs escrevem código e testes, as pessoas revisam e publicam. |
| 📊 **Planilha ou orçamento** | Cada área é uma tarefa; uma IA monta as abas, o financeiro aprova. |
| 🧭 **Processos da empresa** | Onboarding, compras, auditorias, campanhas: etapas com responsável e data. |
| 🔬 **Pesquisa ou conteúdo** | Agentes que coletam e redigem, um editor que decide. |

Cada projeto sabe **onde vive o seu trabalho** (pasta e repositório); o líder por projeto está no [roadmap](ROADMAP.md). Tudo é **self-hosted**: seus dados ficam no seu servidor.

> 👉 **Primeira vez? Siga o [guia passo a passo: seu primeiro projeto](https://github.com/polblancoo/Rust_tuiManager/blob/main/docs/GUIA.pt.md)** (instalar, criar a equipe, adicionar pessoas e IAs, atribuir e acompanhar o andamento, com capturas de tela).

**Como é feito:** gerenciador de tarefas colaborativo, multiusuário e self-hosted, escrito em Rust. Um único servidor e três clientes: uma **TUI** (ratatui), um **app gráfico** (Tauri 2) e uma **web** servida pelo próprio servidor, com atualizações em tempo real para todos os conectados.

| Peça | Crate / caminho | O que é |
|---|---|---|
| Servidor | [`rust-tui-manager-server`](crates/server) (binário `tm-server`) | API REST + WebSocket sobre axum e SQLite |
| Cliente de terminal | [`rust-tui-manager`](crates/tui) | Quadro kanban com ratatui |
| Agente de IA | [`rust-tui-manager-agent`](crates/agent) (binário `tm-agent`) | Vigia o quadro e põe uma IA para trabalhar quando uma tarefa é atribuída a ela |
| Cliente gráfico | [`apps/desktop`](apps/desktop) | App Tauri 2 (a mesma interface da web, em janela própria) |
| Cliente web | [`crates/server/web`](crates/server/web) | Interface gráfica servida pelo `tm-server` em `/`: quadro, calendário, Gantt, lousa, caixa de entrada |
| Modelos compartilhados | [`rust-tui-manager-core`](crates/core) | Tipos e eventos usados por todos |

> A versão 0.1.x do `rust-tui-manager` era um gerenciador local de notas e snippets. Esse código continua em [`legacy/`](legacy/) e nas versões 0.1.x do crate.

![Quadro kanban no cliente gráfico](https://raw.githubusercontent.com/polblancoo/Rust_tuiManager/main/docs/screenshots/desktop-board.png)

<table>
  <tr>
    <td><img src="https://raw.githubusercontent.com/polblancoo/Rust_tuiManager/main/docs/screenshots/desktop-task.png" alt="Detalhe da tarefa com etiquetas, subtarefas e comentários"></td>
    <td><img src="https://raw.githubusercontent.com/polblancoo/Rust_tuiManager/main/docs/screenshots/desktop-calendar.png" alt="Calendário por data limite"></td>
  </tr>
  <tr>
    <td><img src="https://raw.githubusercontent.com/polblancoo/Rust_tuiManager/main/docs/screenshots/desktop-team.png" alt="Gestão da equipe: papéis e convites por código"></td>
    <td><img src="https://raw.githubusercontent.com/polblancoo/Rust_tuiManager/main/docs/screenshots/tui-board.png" alt="Cliente de terminal (ratatui)"></td>
  </tr>
</table>

### Cliente web (também dentro do app de desktop)

O `tm-server` serve a interface em `/`: abra `http://seu-servidor:7878` e pronto.

<table>
  <tr>
    <td><img src="https://raw.githubusercontent.com/polblancoo/Rust_tuiManager/main/docs/screenshots/web-gantt.png" alt="Gantt com dependências entre tarefas"></td>
    <td><img src="https://raw.githubusercontent.com/polblancoo/Rust_tuiManager/main/docs/screenshots/web-canvas.png" alt="Lousa colaborativa com notas, cartões de tarefa e conexões"></td>
  </tr>
  <tr>
    <td><img src="https://raw.githubusercontent.com/polblancoo/Rust_tuiManager/main/docs/screenshots/web-inbox.png" alt="Caixa de novidades ao iniciar a sessão"></td>
    <td><img src="https://raw.githubusercontent.com/polblancoo/Rust_tuiManager/main/docs/screenshots/web-mindmap.png" alt="Mapa mental de uma tarefa"></td>
  </tr>
  <tr>
    <td><img src="https://raw.githubusercontent.com/polblancoo/Rust_tuiManager/main/docs/screenshots/web-settings.png" alt="Menu de configurações: conta, Telegram, equipe e preferências"></td>
    <td><img src="https://raw.githubusercontent.com/polblancoo/Rust_tuiManager/main/docs/screenshots/web-message.png" alt="Mensagem para a equipe (caixa de entrada, e-mail e Telegram)"></td>
  </tr>
  <tr>
    <td><img src="https://raw.githubusercontent.com/polblancoo/Rust_tuiManager/main/docs/screenshots/web-team.png" alt="Equipe: buscar usuários cadastrados, gerar código de convite e membros"></td>
    <td><img src="https://raw.githubusercontent.com/polblancoo/Rust_tuiManager/main/docs/screenshots/web-activity.png" alt="Atividade do projeto"></td>
  </tr>
  <tr>
    <td><img src="https://raw.githubusercontent.com/polblancoo/Rust_tuiManager/main/docs/screenshots/web-invite.png" alt="Código de convite gerado"></td>
  </tr>
</table>

## 🤖 Seus agentes de IA, como integrantes da equipe

Os assistentes de código (Claude Code, Cursor, Windsurf, Zed ou qualquer cliente [MCP](https://modelcontextprotocol.io)) **conectam-se ao mesmo quadro da sua equipe humana**: cada IA tem seu próprio usuário, token e papel, e trabalha como mais um integrante.

- **Adicione quantas IAs quiser**: na barra lateral, seção **🤖 IAs → «+»**. Cria-se o usuário, ele entra na equipe e você vê **o token uma única vez**, com a configuração MCP pronta para copiar. Cada IA leva o seu **modelo** (ex.: `claude:Opus`) e aparece com 🤖 em todas as listas de usuários.
- **Elas se coordenam com as pessoas**: uma IA pode **criar uma tarefa e atribuí-la a um humano** da equipe, comentar, concluir ou apagar as suas próprias tarefas e receber as que você atribuir. Tudo fica registrado em nome dela na atividade.
- **Controle humano**: o token é **trocado ou revogado com um clique**; o papel é leitor ou membro; uma IA **não pode** criar nem apagar equipes, convidar, mudar papéis nem mexer em tarefas alheias (exceto comentar).
- **Um painel de comando**: você atribui trabalho aos seus agentes pela web, pelo app ou pelo terminal, e vê ao vivo o que cada um está fazendo.

> 🚀 **Para que trabalhem sozinhas**: o **`tm-agent`** vigia o quadro e, quando você atribui uma tarefa a uma IA, abre o seu agente (Claude Code, Gemini CLI…) na pasta do projeto, passa a tarefa para *Em andamento*, comenta e a deixa em *Concluído*. O agente também pode acionar um **alarme de ajuda** 🙋 (`tm-agent ask "..."`) e esperar a resposta de uma pessoa. Passo a passo no **[guia de agentes](https://github.com/polblancoo/Rust_tuiManager/blob/main/docs/AGENTES.pt.md)**; dentro do app fica em *Configurações → Agentes de IA*. O binário `tm-mcp` (ferramentas MCP) ainda está em desenvolvimento (veja o [ROADMAP.md](ROADMAP.md)).

## Novidades da 0.6.1

### 📊 Portfólio: todos os seus projetos em uma tela
Andamento, atrasos, recursos, quem trabalha em quê e qual projeto está em risco (🟢 em dia · 🔴 com atrasos · 🟡 sem movimento há 7 dias · ✅ concluído). Um clique no cartão abre o projeto; atualiza ao vivo.

![Portfólio com todos os projetos](https://raw.githubusercontent.com/polblancoo/Rust_tuiManager/main/docs/screenshots/guia-11-portafolio.png)

### 🙋 Quando uma IA precisa de você, ela pede e você responde na mesma mensagem
Se uma IA esbarra em algo que não faz sozinha (um comando bloqueado, uma permissão), executa `tm-agent ask`: o servidor cria uma **tarefa de pedido** para uma pessoa autorizada, com o comando pronto para copiar (**📋 Copiar**). Você executa no seu terminal, cola o resultado como comentário e a IA segue. Para quem vai: quem criou a tarefa, se tiver a nova permissão **Executor**; senão o executor com menos tarefas abertas; senão um admin. Cada IA tem também uma lista editável de **🚫 comandos proibidos**.

![Pedido de uma IA: tarefa criada e comando para copiar](https://raw.githubusercontent.com/polblancoo/Rust_tuiManager/main/docs/screenshots/guia-12-pedido.png)

### 📖 Manual dentro do app, tooltips e três idiomas
Um manual com **busca e índice por temas** (botão **📖 Manual**), um tooltip em cada botão e toda a interface em **espanhol, inglês e português** (Configurações → Preferências → Idioma). O detalhe da tarefa pode ser **maximizado** para ler comentários longos.

![O manual dentro do app, com busca](https://raw.githubusercontent.com/polblancoo/Rust_tuiManager/main/docs/screenshots/guia-13-manual.png)

### Também novo
- Tarefas numeradas (`#12`) e **ordem manual com arrastar e soltar**.
- **Terminal (TUI)** em três idiomas.
- **Email do servidor**: guia para Gmail, Outlook, Brevo, SendGrid, Mailgun, SES ou um servidor próprio.
- CI em Linux, Windows e macOS. Downloads: **Linux disponível**, **Windows e macOS em breve**.
- Um site: **https://polblancoo.github.io/rust_tuiManager/**

## Novidades da 0.5

<table>
  <tr>
    <td><img src="https://raw.githubusercontent.com/polblancoo/Rust_tuiManager/main/docs/screenshots/web-google.png" alt="Configurações com Google Agenda conectado e um «?» de ajuda em cada linha"></td>
    <td><img src="https://raw.githubusercontent.com/polblancoo/Rust_tuiManager/main/docs/screenshots/web-google-help.png" alt="Ajuda do Google Agenda: o que faz e o que configura quem administra o servidor"></td>
  </tr>
  <tr>
    <td><img src="https://raw.githubusercontent.com/polblancoo/Rust_tuiManager/main/docs/screenshots/web-config-help.png" alt="Ajuda de uma linha: «Se você é usuário» e «Se você administra o servidor»"></td>
    <td><img src="https://raw.githubusercontent.com/polblancoo/Rust_tuiManager/main/docs/screenshots/web-invite.png" alt="Código de convite com «Copiar convite» e «Enviar por e-mail»"></td>
  </tr>
</table>

- **Painel da equipe** (botão «👥 Equipe», coluna da direita): membros com o seu papel (editável ali mesmo), desde quando estão, quem os adicionou e se entraram com um código, e quantas tarefas abertas cada um tem; os convites com usos, validade e **quem entrou com cada código**; e os ajustes da equipe (editar nome e descrição, transferir a propriedade, sair ou excluir a equipe com confirmação pelo nome).

<img src="https://raw.githubusercontent.com/polblancoo/Rust_tuiManager/main/docs/screenshots/web-team-panel.png" alt="Painel da equipe: membros, convites e ajustes">

- **Google Agenda** (opcional, em um só sentido): cada usuário conecta a sua conta em *Configurações → Google Agenda* e as suas tarefas com data (atribuídas a ele ou sem responsável, das suas equipes) aparecem como eventos de dia inteiro na agenda principal. Se mudarem, forem concluídas ou apagadas, o evento é atualizado ou removido sozinho (a cada minuto); «Desconectar» limpa o que o app criou. **Quem administra o servidor ativa uma única vez** com `TM_GOOGLE_CLIENT_ID`, `TM_GOOGLE_CLIENT_SECRET` e `TM_GOOGLE_REDIRECT` (cliente OAuth «Aplicativo da Web» do Google Cloud); enquanto o app do Google estiver em modo *Teste*, só entram os Gmail adicionados como usuários de teste.
- **Ajuda em cada parâmetro de Configurações**: um botão «?» por linha explica o que faz para o usuário comum e o que precisa estar configurado se você administra o servidor (SMTP, Telegram, Google, HTTPS…).
- **Convites prontos para compartilhar**: ao gerar um código, *Copiar convite* monta um texto com o link de instalação, o endereço do servidor, os passos e o código, e *Enviar por e-mail* o abre no seu cliente de e-mail.
- **Busca de usuários mais clara**: os resultados aparecem como botões (funciona também no app de desktop) e a busca encontra o texto em qualquer parte do nome.
- **Equipe mais organizada**: em cima, adicionar e convidar; embaixo, a lista de membros.
- A janela de novidades deixou de reaparecer a cada início quando não há nada sem ler.

## Funcionalidades

- **Equipes** com papéis (leitor, membro, admin, dono) e **convites por código** com validade e limite de usos.
- **Projetos e quadro kanban** (a fazer / em andamento / concluído), prioridades, datas limite e atribuição. Cada projeto guarda a sua **pasta de trabalho e o repositório**; admins podem **editá-lo ou excluí-lo** (a exclusão pede que se digite o nome).
- **Etiquetas** por equipe e **subtarefas** com barra de progresso.
- **Comentários** em cada tarefa.
- **Calendário** mensal por data limite, **Gantt** com dependências entre tarefas (arraste as barras para mover datas), **lousa** colaborativa por projeto (notas, cartões de tarefa e conexões) e **mapa mental** de cada tarefa (cliente gráfico).
- **Responsável**: o líder (admin ou dono) atribui a qualquer membro; um membro só pega tarefas livres ou solta a sua.
- **Caixa de novidades**: ao iniciar a sessão você vê o que aconteceu enquanto estava fora (atribuíram a você, comentaram, adicionaram você) e as suas tarefas abertas; chega ao vivo e, se você cadastrar e verificar o seu e-mail, também por e-mail.
- **Cliente web**: o `tm-server` serve a mesma interface gráfica em `/` (abra `http://seu-servidor:7878` no navegador, sem instalar nada; `TM_WEB=0` desativa).
- **Menções**: escreva `@usuario` em um comentário e essa pessoa recebe o aviso (caixa de entrada, e-mail e Telegram).
- **Lembretes de vencimento**: o responsável recebe um aviso quando faltam menos de 24 horas e outro, uma única vez, quando a tarefa vence sem ser concluída.
- **Tarefas recorrentes** (diária, semanal ou mensal): ao concluí-las cria-se a seguinte, com a data deslocada, as etiquetas e as subtarefas pendentes.
- **Anexos** (até 10 por tarefa, 2 MB cada): o servidor sempre os entrega como download. A web mostra miniaturas e pré-visualização de imagens (PNG, JPEG, GIF, WebP) e texto simples, verificando o conteúdo real do arquivo; nunca HTML, SVG nem PDF.
- **Modelos**: duplique um projeto com as suas tarefas, etiquetas e dependências (tudo volta pendente e sem responsável).
- **Exportar** as tarefas de um projeto para CSV ou JSON (o CSV neutraliza fórmulas de planilha).
- **Atividade** por projeto: quem criou, moveu, atribuiu, comentou ou anexou, e quando.
- **Web instalável (PWA)**: pelo navegador do celular ou do computador você pode «instalá-la» como um app.
- **Telegram** (opcional): cada usuário vincula a sua conta ao bot e recebe por lá as novidades; qualquer membro pode enviar **mensagens para a equipe** (caixa de entrada, e-mail e Telegram).
- **Adicionar membros com busca** entre usuários cadastrados, atalhos com prefixo `Ctrl+T` e **ajuda com busca** (`?`) nos dois clientes.
- **Google Agenda** (opcional): as suas tarefas com data aparecem na sua agenda; veja *Novidades da 0.5*.
- **Agentes de IA** com conta, modelo e token próprios, e o `tm-agent` para que trabalhem sozinhos.
- **Tempo real**: uma mudança de qualquer usuário aparece na hora em todos os clientes da equipe.

## Instalação rápida

**Download**: [última versão](https://github.com/polblancoo/Rust_tuiManager/releases/latest) (Linux: `.deb`, `.rpm`, AppImage e `tar.gz`; **Windows e macOS em breve**) · Site: https://polblancoo.github.io/rust_tuiManager/


```sh
cargo install rust-tui-manager-server   # instala o binário `tm-server`
cargo install rust-tui-manager          # cliente de terminal
cargo install rust-tui-manager-agent    # instala o `tm-agent` (opcional, para agentes de IA)
```

Ou a partir deste repositório:

```sh
cargo run -p rust-tui-manager-server                          # servidor em 0.0.0.0:7878
cargo run -p rust-tui-manager -- http://127.0.0.1:7878        # cliente TUI (F2 = criar conta)
cargo run -p tm-desktop                                       # cliente gráfico
```

O cliente gráfico precisa das dependências do Tauri (no Linux: `webkit2gtk-4.1`, `libsoup-3.0`).

## Configuração do servidor

| Variável | Padrão | Descrição |
|---|---|---|
| `TM_ADDR` | `0.0.0.0:7878` | Endereço em que escuta |
| `TM_DATABASE` | `sqlite://tm-server.db` | Banco de dados (criado com permissões `0600`) |
| `TM_JWT_SECRET` | gerado e guardado no banco | Segredo dos tokens; mínimo de 32 caracteres |
| `TM_CORS_ORIGINS` | Tauri e localhost | Origens extras permitidas, separadas por vírgula |
| `TM_TLS_CERT` / `TM_TLS_KEY` | — | Certificado e chave PEM para servir HTTPS diretamente |
| `TM_TRUSTED_PROXY` | — | `1` se roda atrás de um proxy reverso próprio (usa `X-Forwarded-For` para o limite de requisições) |

Para avisos por e-mail (opcionais; sem isso só funciona a caixa de entrada):

| Variável | Descrição |
|---|---|
| `TM_SMTP_URL` | Servidor SMTP, ex.: `smtps://smtp.exemplo.com:465` ou `smtp://127.0.0.1:2525` |
| `TM_SMTP_FROM` | Remetente, ex.: `Tarefas <avisos@exemplo.com>` |
| `TM_SMTP_USER` / `TM_SMTP_PASSWORD` | Credenciais, se o servidor as exigir |

Para o Telegram, crie um bot com o [@BotFather](https://t.me/BotFather) e passe o token ao servidor:

| Variável | Descrição |
|---|---|
| `TM_REMINDER_SECS` | De quantos em quantos segundos os vencimentos são verificados (padrão 300) |
| `TM_TELEGRAM_TOKEN` | Token do bot. O servidor o atende por *long polling*: não precisa de URL pública nem de HTTPS próprio |
| `TM_TELEGRAM_SEND_ONLY` | `1` para só enviar avisos (sem vincular por `/start`, sem polling); permite compartilhar um bot com outro programa |
| `TM_GOOGLE_CLIENT_ID`, `TM_GOOGLE_CLIENT_SECRET`, `TM_GOOGLE_REDIRECT` | Cliente OAuth do Google Cloud que ativa a sincronização com o Google Agenda (redirecionamento: `https://seu-servidor/api/google/callback`) |

Cada usuário vincula em **Configurações → Telegram** (web/desktop) ou com `Ctrl+T g` (TUI): gera um código de uso único (15 min) e o envia ao bot como `/start CÓDIGO`; `/stop` desvincula. Depois de vinculado, o bot também responde: `/tareas` (suas tarefas abertas) e `/hecho N` (conclui a tarefa N se for sua, ou qualquer uma se você for admin). Só se escreve para chats vinculados dessa forma, nunca para ids arbitrários.

Cada usuário cadastra o seu e-mail e o confirma com um código de 6 dígitos; só os endereços verificados recebem avisos (com limite por hora).

Sem TLS próprio, coloque-o atrás de um proxy TLS antes de expô-lo à internet.

Para mantê-lo sempre ligado, rode-o como serviço (por exemplo, uma unit de usuário do systemd com `ExecStart=~/.local/bin/tm-server` e `Restart=on-failure`).

## Segurança

- Senhas com argon2; tokens JWT de 7 dias revogáveis com «encerrar a sessão em todos os dispositivos».
- Bloqueio por conta (5 falhas = 15 minutos) e limitação por IP no login, cadastro e WebSocket.
- Limites de tamanho de corpo, de comprimento de cada campo e de quantidade por recurso em todos os endpoints.
- CORS restrito, cabeçalhos de segurança e CSP estrita no app Tauri.
- O token do WebSocket não viaja na URL: é enviado como primeira mensagem.
- Os códigos de convite são guardados com hash e só são mostrados ao serem criados.
- As contas de IA usam tokens próprios e revogáveis (`tmk_…`, guardados com hash) e não podem administrar equipes, papéis nem convites.

## Papéis

| Papel | Pode |
|---|---|
| Leitor | Ver tudo |
| Membro | Criar e editar tarefas, subtarefas, comentários, etiquetas e projetos |
| Admin | Também adicionar ou remover membros de nível inferior, mudar papéis, convidar e editar ou excluir projetos |
| Dono | Tudo; não pode abandonar a equipe |

Ninguém pode conceder um papel igual ou superior ao seu.

## Estrutura

```
crates/core      modelos e eventos compartilhados
crates/server    servidor axum + SQLite + WebSocket
crates/tui       cliente de terminal
crates/agent     tm-agent: põe uma IA para trabalhar nas tarefas atribuídas
apps/desktop     cliente Tauri (frontend estático em ui/)
legacy/          gerenciador local anterior (0.1.x)
```

## Licença

MIT
