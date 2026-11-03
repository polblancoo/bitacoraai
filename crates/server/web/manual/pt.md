# Guia passo a passo: seu primeiro projeto no painel de comando

O Rust TUI Manager é um **painel de comando**: um lugar onde você vê e dirige vários projetos ao mesmo tempo, com pessoas e IAs como recursos. Este guia é para quem vai **dirigir o trabalho de uma equipe de pessoas e IAs**: um CTO, uma líder de operações, uma dona de agência… O projeto pode ser um **app**, uma **planilha de orçamento**, um **processo de onboarding**, uma campanha, uma auditoria: o Rust TUI Manager não entende de apps, entende de **tarefas, responsáveis e andamento**. Em 10 minutos você tem o primeiro montado.

> As capturas de tela são de uma empresa de exemplo («Mi empresa») com três projetos bem diferentes.

**O que você vai fazer**

1. [Instalar o servidor](#1-instalar-o-servidor)
2. [Criar sua conta e sua equipe](#2-criar-sua-conta-e-sua-equipe)
3. [Criar os projetos](#3-criar-os-projetos)
4. [Adicionar pessoas](#4-adicionar-pessoas)
5. [Adicionar IAs](#5-adicionar-ias)
6. [Montar e atribuir tarefas](#6-montar-e-atribuir-tarefas)
7. [Acompanhar o andamento](#7-acompanhar-o-andamento)
8. [Delegar a um líder](#8-delegar-a-um-líder)
9. [Dirigir vários projetos ao mesmo tempo](#9-dirigir-vários-projetos-ao-mesmo-tempo)
10. [Quando uma IA precisa que você faça algo](#10-quando-uma-ia-precisa-que-você-faça-algo)
11. [Idioma, tooltips e este manual](#11-idioma-tooltips-e-este-manual)
12. [Configurar o email do servidor](#12-configurar-o-email-do-servidor)
13. [Permitir comandos pela tarefa](#13-permitir-comandos-pela-tarefa)
14. [Terminal no navegador](#14-terminal-no-navegador)
15. [Consumo e cotas](#15-consumo-e-cotas)
16. [Licença comercial e aviso de instalação](#16-licenca-comercial-e-aviso-de-instalacao)

---

## 1. Instalar o servidor

O servidor guarda tudo (equipes, tarefas, usuários) e serve a web. Instala-se uma vez.

1. Baixe `tm-server-linux-x86_64.tar.gz` da [última versão](https://github.com/polblancoo/crewhelm/releases/latest) (ou `cargo install crewhelm-server`).
2. Descompacte e execute `./tm-server`.
3. Abra `http://localhost:7878` no navegador.

Para que outras pessoas entrem de outras máquinas, coloque o servidor num endereço que elas alcancem (idealmente com HTTPS: veja a [configuração do servidor](https://github.com/polblancoo/crewhelm/blob/main/README.pt.md)). Se for só para testar, `localhost` basta.

**Instalação de uma linha (Linux x86_64, sem sudo):** `curl -fsSL https://raw.githubusercontent.com/polblancoo/crewhelm/main/install.sh | sh`. Baixa o servidor do release, **verifica o SHA256** (se não bater, não instala), deixa em `~/.local/bin` e cria um serviço de usuário que inicia sozinho. Opções: `--port N`, `--no-service`, `--with-agent`, `--with-claude`, `--with-gemini`, `--with-codex`, `--with-opencode` (install that AI's program; `--with-all-ais` all four), `--version X`. Na primeira vez que você entra, um **guia de início** com desenhos leva você por equipe, projeto e IA (repita em Configurações → Preferências → 🧭 Guia de início).

Se preferir terminal ou janela própria, também há um cliente de terminal e um app de desktop na mesma página de downloads.

## 2. Criar sua conta e sua equipe

1. Na tela inicial escolha **Criar conta** (usuário e senha de 8 caracteres ou mais).
2. Na barra lateral, **Equipes → «+»**, coloque o nome da sua organização (por exemplo «Mi empresa»). Você fica como **Dono**.

Uma **equipe** é o grupo de pessoas e IAs que compartilham projetos. Você pode ter várias.

## 3. Criar os projetos

Em **Projetos → «+»** crie um para cada coisa que queira levar adiante. No exemplo: um app, uma planilha e um processo, cada um com sua cor.

![Quadro do projeto «App móvel de reservas»](https://raw.githubusercontent.com/polblancoo/crewhelm/main/docs/screenshots/guia-01-tablero.png)

Cada projeto tem seu **quadro** (A fazer · Em andamento · Concluído). Com **+ Nova tarefa** você carrega o trabalho: título, descrição, prioridade, datas e responsável. Os mesmos dados aparecem também em **calendário**, **Gantt** e **lousa**.

![Outro projeto: uma planilha de orçamento, com responsáveis humanos e de IA](https://raw.githubusercontent.com/polblancoo/crewhelm/main/docs/screenshots/guia-08-otro-proyecto.png)

Cada projeto guarda **onde vive o seu trabalho**: com **⚙ Projeto** (admin ou dono) você define a pasta de trabalho e o repositório, e eles aparecem acima do quadro.

![Editar o projeto: nome, pasta de trabalho e repositório](https://raw.githubusercontent.com/polblancoo/crewhelm/main/docs/screenshots/guia-09-proyecto.png)

> 💡 Dica: não precisa ser software. Uma planilha, um processo de onboarding ou uma auditoria se tocam do mesmo jeito: tarefas com responsável e data.

## 4. Adicionar pessoas

Botão **👥 Equipe** (canto superior direito) → **➕ Adicionar / convidar**.

- **Já tem conta**: digite parte do nome, escolha o papel e **Adicionar**.
- **Ainda não tem conta**: escolha um papel e **Gerar código**. Use **Copiar convite** (texto pronto para WhatsApp ou e-mail com link de download, servidor e código) ou **Enviar por e-mail**.

![Convidar com um código](https://raw.githubusercontent.com/polblancoo/crewhelm/main/docs/screenshots/guia-07-invitar.png)

| Papel | O que pode fazer |
|---|---|
| **Leitor** | Olhar. |
| **Membro** | Criar, editar e comentar tarefas; pegar as livres. |
| **Admin** | Além disso, atribuir tarefas a outros, adicionar pessoas e gerar convites. |
| **Dono** | Tudo, inclusive transferir ou excluir a equipe. |

O painel da equipe mostra cada integrante com seu papel, desde quando está, quem o adicionou e quantas tarefas abertas tem:

![Painel da equipe com pessoas e IAs](https://raw.githubusercontent.com/polblancoo/crewhelm/main/docs/screenshots/guia-06-panel-equipo.png)

## 5. Adicionar IAs

As IAs (Claude Code, Cursor, Windsurf, Zed, Gemini ou outro cliente compatível com [MCP](https://modelcontextprotocol.io)) trabalham **como mais um integrante**. Quantas precisar, de modelos diferentes.

1. Na barra lateral, **🤖 IAs → «+»**.
2. Coloque um nome que diga o que ela é (`claude-backend`, `gemini-dados`) e o papel: **Membro** (cria e edita as suas tarefas) ou **Leitor** (só olha).

   ![Nova IA na equipe](https://raw.githubusercontent.com/polblancoo/crewhelm/main/docs/screenshots/guia-02-nueva-ia.png)

3. Mostra-se **o token uma única vez**, com a configuração MCP pronta para copiar. É a «chave» dessa IA.

   ![Token da IA e configuração MCP](https://raw.githubusercontent.com/polblancoo/crewhelm/main/docs/screenshots/guia-03-token.png)

4. Se perder ou quiser cortar: clique na IA → **Novo token** (o anterior deixa de valer) ou **Revogar**.

   ![Ficha de uma IA](https://raw.githubusercontent.com/polblancoo/crewhelm/main/docs/screenshots/guia-03-ia.png)

**O que uma IA pode e não pode fazer**

- ✅ Criar tarefas e **atribuí-las a pessoas** da equipe, comentar, concluir ou apagar **as que ela criou ou que lhe atribuíram**.
- ❌ Criar ou apagar equipes, convidar pessoas, mudar papéis, ou mexer em tarefas alheias (exceto comentar).
- Tudo o que faz fica **em seu nome** na atividade, e aparece com 🤖 no painel.

> 🚧 **Estado**: as contas de IA e seus tokens já funcionam contra a API do servidor. O binário `tm-mcp` e o *skill* que as conectam direto à sua plataforma de código estão em desenvolvimento (veja o [ROADMAP](https://github.com/polblancoo/crewhelm/blob/main/ROADMAP.md)). Enquanto isso, o **`tm-agent`** já permite que uma IA pegue sozinha as tarefas atribuídas: veja o [guia de agentes](https://github.com/polblancoo/crewhelm/blob/main/docs/AGENTES.pt.md).

## 6. Montar e atribuir tarefas

- **Nova tarefa** em qualquer coluna; abra-a para somar descrição, datas, etiquetas, subtarefas, anexos e dependências.
- **Responsável**: o admin atribui a quem quiser (pessoa ou IA); um membro pega as livres ou solta a sua.
- **Menções** `@usuario` nos comentários avisam essa pessoa.
- Uma IA pode criar uma tarefa e **deixá-la atribuída a uma pessoa**, para lhe pedir algo.

## 7. Acompanhar o andamento

Tudo se atualiza **ao vivo**: não precisa recarregar.

| Vista | Para quê |
|---|---|
| **Quadro** | O que está a fazer, em andamento e concluído. |
| **Gantt** | Cronograma com datas e dependências; as atrasadas são marcadas. |
| **Atividade** | Quem fez o quê e quando, pessoas e IAs juntas. |
| **Calendário** | O que vence a cada dia. |
| **🔔 Caixa de entrada** | Novidades: atribuições, menções, mensagens. Também por e-mail e Telegram. |

![Gantt do projeto, com tarefas de pessoas e de IAs](https://raw.githubusercontent.com/polblancoo/crewhelm/main/docs/screenshots/guia-04-gantt.png)

![Atividade: o histórico do projeto, com as ações das IAs em seu nome](https://raw.githubusercontent.com/polblancoo/crewhelm/main/docs/screenshots/guia-05-actividad.png)

Além disso, o **Google Calendar** (suas tarefas com data aparecem na sua agenda) é conectado em **Configurações**.

## 8. Delegar a um líder

Hoje, para delegar um projeto: adicione a pessoa com o papel **Admin** (pode atribuir tarefas, adicionar gente e gerar convites) e atribua a ela as tarefas principais. Ela monta a sua parte e traz a sua gente.

> 🚧 **Em breve**: **líder por projeto** (com alcance só nesse projeto), **escopo configurável por IA** (em quais projetos mexe, se pode convidar outras IAs) e **relatórios de andamento** com gráficos. Veja o [roteiro](https://github.com/polblancoo/crewhelm/blob/main/ROADMAP.md).

## 9. Dirigir vários projetos ao mesmo tempo

**📊 Portfólio** é o primeiro botão da lista de *Projetos*. Mostra todos os projetos da equipe em uma só tela:

- **Em cima**: projetos, andamento geral, tarefas em andamento, atrasadas e projetos em risco.
- **Um cartão por projeto**: barra de andamento, quem tem tarefas abertas (as IAs com 🤖), última atividade, próximo vencimento e quantas ações as IAs fizeram em 24 h.
- **Estado**: 🟢 em dia · 🔴 com tarefas atrasadas · 🟡 sem movimento há 7 dias · ✅ concluído · ⚪ sem tarefas.
- **Embaixo**: o que as IAs fizeram por último. Um clique no cartão abre o projeto, e tudo se atualiza ao vivo.

![Portfólio com todos os projetos](https://raw.githubusercontent.com/polblancoo/crewhelm/main/docs/screenshots/guia-11-portafolio.png)

**Tarefas numeradas.** Cada tarefa mostra seu número (`#12`) no cartão e no detalhe, para citá-la sem ambiguidade em mensagens e comentários.

**Ordem manual.** Arraste uma tarefa dentro da coluna para mudar a ordem em que é feita; a ordem que você deixa vale mais que a prioridade. Também dá para escolher o responsável ao criá-la.

## 10. Quando uma IA precisa que você faça algo

Uma IA pode esbarrar em algo que não faz sozinha: um comando bloqueado, uma permissão, uma decisão. Então ela **pede a você** e espera a resposta:

1. A IA executa `tm-agent ask "…"`. Na tarefa dela aparece um comentário com 🙋 e é **criada uma tarefa de pedido** («🙋 Pedido de #12: …») atribuída a uma pessoa. O comentário diz qual.
2. Abra a tarefa de pedido: no topo está o painel **🙋 Pedido de uma IA** com **cada comando na sua própria linha e um botão 📋 Copiar**. Cole no seu terminal e execute.
3. **Onde vai a saída?** Na caixa **«Cole a saída aqui»** do mesmo painel, e **📤 Enviar saída à IA**. É publicada como comentário dessa tarefa e **chega sozinha à IA** (também aparece na tarefa original dela). Escrever um comentário comum na tarefa de pedido faz o mesmo.
4. Se o pedido é uma **decisão ou permissão** («posso apagar X?»), use **✅ Autorizar** ou **❌ Rejeitar** (com o motivo na caixa). Para um comando que a IA tem bloqueado, «autorizar» não basta: execute você e envie a saída.
5. A IA lê na próxima consulta (a cada 10 s) e segue. Ao enviar, o pedido passa sozinho para Concluída.

![Pedido de uma IA: tarefa criada, comando para copiar](https://raw.githubusercontent.com/polblancoo/crewhelm/main/docs/screenshots/guia-12-pedido.png)

**Para quem vai?** Para os membros com a permissão **Executor** (janela da equipe → caixa *Executor* ao lado de cada pessoa; só um admin altera, e as IAs não podem ser executoras):

1. quem criou a tarefa, se for executor;
2. senão (por exemplo, outra IA a criou), o executor com menos tarefas abertas;
3. sem executores, um admin da equipe.

**Comandos proibidos.** Na ficha de cada IA, **🚫 Comandos proibidos** (um por linha, ex. `Bash(rm:*)`). A IA não pode executá-los: se precisar, pede a você como acima.

## 11. Idioma, tooltips e este manual

- **Idioma**: *Configurações → Preferências → Idioma* (espanhol, inglês ou português). Por padrão segue o do navegador.
- **Tooltips**: passe o cursor sobre um botão para ver o que faz.
- **Este manual** fica dentro do app: botão **📖 Manual** (embaixo à esquerda, ou *Configurações → Ajuda*). Tem busca e um índice por temas com links.

![O manual dentro do app, com busca](https://raw.githubusercontent.com/polblancoo/crewhelm/main/docs/screenshots/guia-13-manual.png)

## 12. Configurar o email do servidor

Para o app enviar **emails** (códigos de verificação, atribuições, menções, lembretes) o **servidor** precisa de uma conta de email de saída. Quem administra o servidor configura uma só vez; os usuários só verificam o email em *Configurações → Meu email*.

**Qual arquivo editar?** Se você usa o serviço do systemd (`tm-server.service`), crie **`~/.config/tm-server/mail.env`** (permissões `600`, nunca dentro do repositório) com estas quatro variáveis:

```
TM_SMTP_URL=smtps://smtp.gmail.com:465
TM_SMTP_USER=suaconta@gmail.com
TM_SMTP_PASSWORD=a-senha-de-app
TM_SMTP_FROM=Quadro de comando <suaconta@gmail.com>
```

e em `~/.config/systemd/user/tm-server.service` adicione, junto com os outros `EnvironmentFile`:

```
EnvironmentFile=-%h/.config/tm-server/mail.env
```

Depois: `systemctl --user daemon-reload && systemctl --user restart tm-server`. Se você inicia o `tm-server` à mão ou com Docker, defina as mesmas variáveis no ambiente (`export …` ou `-e …`).

**Conforme o seu provedor** mudam `TM_SMTP_URL`, o usuário e a senha (confira os dados na documentação do provedor):

| Provedor | `TM_SMTP_URL` | `TM_SMTP_USER` | `TM_SMTP_PASSWORD` |
|---|---|---|---|
| Gmail | `smtps://smtp.gmail.com:465` | seu endereço completo | senha de app (exige verificação em 2 etapas): https://myaccount.google.com/apppasswords |
| Outlook / Microsoft 365 | `smtp://smtp.office365.com:587?tls=required` | seu endereço completo | sua senha ou senha de app; no Microsoft 365 o administrador deve habilitar o *SMTP autenticado* |
| Yahoo | `smtps://smtp.mail.yahoo.com:465` | seu endereço completo | senha de app |
| Zoho | `smtps://smtp.zoho.com:465` | seu endereço completo | sua senha ou senha de app |
| Brevo | `smtp://smtp-relay.brevo.com:587?tls=required` | seu login da Brevo | chave SMTP |
| SendGrid | `smtps://smtp.sendgrid.net:465` | `apikey` | sua API key |
| Mailgun | `smtps://smtp.mailgun.org:465` | `postmaster@seu-dominio` | senha SMTP do domínio |
| Amazon SES | `smtps://email-smtp.REGIAO.amazonaws.com:465` | usuário SMTP do SES | senha SMTP do SES |
| Servidor próprio (Postfix…) | `smtp://localhost:25` | — | — |

- `smtps://` usa a porta 465 (TLS direto); `smtp://…:587?tls=required` usa STARTTLS.
- Coloque usuário e senha em `TM_SMTP_USER` e `TM_SMTP_PASSWORD` (não dentro da URL) e não precisa escapar caracteres especiais.
- `TM_SMTP_FROM` é o remetente; a maioria dos provedores exige que seja um endereço seu ou de um domínio seu verificado.

**Testar**: *Configurações → Meu email*, digite seu endereço e o código de 6 dígitos deve chegar. Se não chegar, veja o log do servidor (`journalctl --user -u tm-server -n 50`): uma senha rejeitada ou uma porta bloqueada aparece ali.

> Gmail e outros serviços gratuitos limitam os envios (Gmail, uns 500 por dia) e reescrevem o remetente. Serve para começar; com muitos usuários, use um serviço de envio com domínio próprio.

## 13. Permitir comandos pela tarefa

Quando uma IA pede um comando, abra a tarefa de pedido e toque em **✅ Permitir comando…**. A caixa mostra o comando, **explica o que ele faz** (aviso vermelho se for perigoso) e deixa escolher **uma vez** ou **sempre**. Também dá para escrever `/permitir cmd` (uma vez) ou `/permitir siempre cmd` (só owners e admins) num comentário. A IA recebe as permissões em `TM_ALLOW` e retoma a tarefa.

![Permitir comando](https://raw.githubusercontent.com/polblancoo/crewhelm/main/docs/screenshots/guia-16-permitir.png)

## 14. Terminal no navegador

Só para **owners**. O botão **💻 Terminal** da tarefa abre um console na pasta do projeto. Há dois jeitos de conectar (escolhido por projeto):

- **Pelo agente** (recomendado): rode `tm-agent` com `TM_AGENT_TERMINAL=1` na máquina do projeto. A conexão sai dessa máquina: sem chaves nem portas abertas, funciona atrás de NAT.
- **SSH com chave**: o app gera uma chave ed25519 (**🔑 Gerar chave**) e a instala com **🔐 Instalar chave com senha** (a senha não é guardada). A impressão digital do servidor é fixada na primeira vez e, se mudar, é rejeitada.

Cada sessão deixa um comentário 📝 e uma nota `.tm/terminal-tarea-N.md` na pasta do projeto, que a IA lê ao retomar. No menu, **🖥️ Terminais** mostra até 8 painéis e o estado de cada IA (🙋 precisa de você · ⚙️ trabalhando · ⏳ na fila · ✅ terminou).

![Vista Terminais](https://raw.githubusercontent.com/polblancoo/crewhelm/main/docs/screenshots/guia-14-terminales.png)

## 15. Consumo e cotas

**💰 Consumo** mostra tokens, custo e horas por IA e por tarefa, com barras de **cota** da equipe, do projeto e da tarefa. Numa tarefa, **🪙 Tokens a gastar** define um teto e **🧮 Estimar** sugere um valor pelo histórico. A cota é verificada **antes** de cada execução: se faltar, a IA não começa e avisa num comentário. Para cortar em dólares use `TM_AGENT_MAX_USD` no agente (1 USD ou mais). O Portfólio soma o consumo do mês.

![Consumo](https://raw.githubusercontent.com/polblancoo/crewhelm/main/docs/screenshots/guia-15-consumo.png)
![Teto de tokens](https://raw.githubusercontent.com/polblancoo/crewhelm/main/docs/screenshots/guia-17-tokens.png)

## 16. Licença comercial e aviso de instalação

O programa é **AGPL-3.0-only e grátis**. A **licença comercial** é para quem o usa numa empresa e não quer cumprir a AGPL. Em **Configurações → 🏢 Licença** (só a primeira pessoa registrada no servidor):

1. Escolha se o servidor é **pessoal** ou de **empresa**.
2. **Comprar uma licença comercial** → escolha o plano, a rede (**USDT na Solana ou na Ethereum**) e seu e-mail.
3. Pague **exatamente** o valor mostrado (as últimas casas decimais identificam seu pagamento) ao endereço indicado; o QR já o traz. Verifique o **contrato do token** com o link do explorador.
4. O app verifica o pagamento sozinho. Quando confirmado, chega um e-mail com o código `TM-XXXX-XXXX-XXXX` e a licença se ativa sozinha. Se não, cole o código em **Já tem um código?**.

O **aviso** no topo aparece com mais de 3 pessoas ou 2 IAs, ou se você declarar que é de empresa; pode ser dispensado por 7 dias e nunca bloqueia nada.

**Aviso de instalação.** Uma vez por dia o servidor envia um código aleatório, a versão, o sistema operacional e quantas pessoas, IAs, projetos e equipes existem. Nunca nomes, e-mails, tarefas nem conteúdo; o IP não é guardado. Desligue com `TM_TELEMETRY=0` ou pela janela Licença.

### Planos e preços (por ano)

| Plano | Inclui | Preço de tabela | Preço fundador |
|---|---|---|---|
| **Grátis** (AGPL) | até 3 pessoas e 2 IAs, sem aviso | USD 0 | — |
| **Equipe** | até 10 pessoas e 10 IAs | **USD 290** | **USD 290 para sempre** (primeiros 5) |
| **Empresa** | até 50 pessoas e 50 IAs, com suporte | **USD 990** | — |

- **🎖 Preço fundador:** os **primeiros 5 clientes** pagam o preço fundador e o mantêm **para sempre** nas renovações. O app mostra sozinho enquanto restarem vagas.
- **Se você crescer:** se, com uma licença vigente, você passar das pessoas ou IAs do plano, o app avisa e aparece **⬆ Melhorar o plano**: você paga só a **diferença** entre o preço do plano novo e o que já pagou, e o vencimento não muda.
- **Termos:** antes de pagar você lê e aceita os **termos da licença comercial** (no seu idioma). Guardamos a versão que você aceitou e a data. Os termos deixam claro, entre outras coisas, que cada comando ou permissão que você aprova para uma IA é responsabilidade sua e que o software é entregue sem garantias.
- Os preços são confirmados na tela de compra: o que você vê ali é o que é cobrado.


---

**Algo não ficou claro?** Cada opção de **Configurações** tem um «?» com ajuda. E se encontrar um problema, abra uma [issue](https://github.com/polblancoo/crewhelm/issues).

## 17. Novidades da interface (0.7.1)

- **CrewHelm.** O app agora se chama **CrewHelm** (interface, e-mails e manifesto). Programas e serviços mantêm seus nomes por enquanto.
- **Guia de início.** Um assistente de 5 passos com ilustrações: boas-vindas, equipe, projeto, IA (opcional) e tarefa de exemplo. Abre sozinho se você não tem equipes; em um servidor vazio, a tela de entrada oferece direto **Criar conta**.
- **Pedidos de uma IA, mais rápidos.** Os comandos que a IA pede aparecem juntos em uma lista: **📋** copia, **✕** tira os que você não quer e **✅ Permitir os N comandos sempre** adiciona todos à lista da IA, avisa e marca o pedido como Feito. **⚙ Lista da IA** abre a lista de permitidos para apagar ou editar linhas (salva na hora). Um comando pode ter até 1000 caracteres.
- **Tarefa mais cômoda.** **💾 Salvar** e **🗑 Apagar** também ficam no alto, em um cabeçalho fixo, e a barra de baixo fica grudada no fundo. **⤢ Maximizar** deixa os comentários maiores.
- **Terminal da tarefa.** Uma só terminal ocupa a tela toda e, se a máquina tem `herdr`, abre sozinho (desligue com `localStorage autoHerdr = "0"`). `~/pasta` funciona como pasta de trabalho.
- **Novidades ampliáveis.** Amplie com **⤢**; ao tocar numa novidade abre **a sua** tarefa.
- **Portfólio clicável.** Em «O último que as IAs fizeram», uma linha com **💬 ver** abre a tarefa e toda a conversa.
- **Configurações em abas**, com ajuda sob cada linha.
- **Zoom da letra.** Embaixo à direita: **A−**, a porcentagem (clique = 100 %) e **A+**; também **Ctrl + Alt + roda**. Lembrado em cada dispositivo.
- **Confirmações próprias.** Apagar, sair de uma equipe ou desconectar uma conta pergunta em uma janela do app (Esc cancela), não no aviso do navegador.

### Perfil de uso (Configurações → Equipe e projeto)

Escolha para que a equipe usa o CrewHelm e a interface se prepara sozinha: *Desenvolvimento de software* (A fazer / Em andamento / Feito, com terminais e pasta do código), *Conteúdo e redes* (Rascunho / Em produção / Publicado, calendário como visão inicial e sem partes técnicas), *Fábrica de apps* (Backlog / Em construção / Entregue, vários projetos em paralelo), *Escritório e consultorias* (administração, contabilidade, advocacia: Pendente / Em andamento / Fechado, com o calendário de vencimentos) ou *Outros / equipe geral*. «Criar exemplo» monta uma **equipe de exemplo separada** («Ejemplo · …») com projetos e tarefas de amostra: sua equipe real não é tocada, você volta a ela com «Volver a mi equipo» e «Eliminar ejemplo» apaga a equipe de exemplo inteira. Só muda nomes e o que é mostrado: seus dados não mudam e você pode trocar de perfil quando quiser. Também se escolhe no guia de início.

Se a equipe lida com informação confidencial (por exemplo, um escritório de advocacia), ative a **revisão humana** do projeto, não carregue dados que não quer que uma IA veja e lembre que o texto das tarefas chega ao modelo que você usar; para dados muito sensíveis, use um modelo próprio da empresa.

### GitHub por projeto (botão 🐙 GitHub, no topo)

1. Em **⚙ Configurações → Avisos e conexões → GitHub** conecte sua conta (uma vez). O administrador do servidor precisa ter definido `TM_GITHUB_CLIENT_ID`.
2. Em **⚙ Dados do projeto** informe o repositório (`dono/repo` ou a URL). Depois toque em **🐙 GitHub** (no topo, ao lado de ⚙ Projeto): liste os issues abertos e **traga os marcados como tarefas**.
3. Em **Ajustes do GitHub para este projeto** há uma caixa por função; cada uma liga ou desliga sozinha: **verificar a cada 5 minutos** (passa para *Feito* a tarefa cujo issue foi fechado ou cujo Pull Request foi integrado, e mostra o estado do PR), **passar comentários** entre a tarefa e o issue (em repositório público, qualquer pessoa os vê), **publicar como issue** as tarefas novas, **avisar quando uma IA termina** para você abrir o Pull Request, **estado dos testes (CI)** com ✅/❌ ao lado do botão, **registrar os commits** em Atividade e **webhook** (aviso na hora; o servidor precisa ser alcançável pela internet; a caixa mostra a URL e o *secret* para colar no GitHub).
4. Na ficha de uma tarefa: **🐙 Publicar como issue**, **Abrir Pull Request** (da branch `tm/tarea-N` ou outra que você indicar; a decisão é sempre sua) e **Fechar issue**.

Os tokens do GitHub e do Google ficam **criptografados** (AES-256-GCM) no banco; a chave fica em `~/.config/tm-server/credenciales.key` (ou `TM_KEY_FILE`). Guarde-a junto com seus backups: sem ela é preciso reconectar as contas.

---

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

![Guia do tm-agent dentro do app](https://raw.githubusercontent.com/polblancoo/crewhelm/main/docs/screenshots/guia-10-agentes.png)

## O que você precisa

- O servidor rodando e uma conta de **admin ou dono** da equipe.
- Uma máquina com a **pasta do projeto** e com o agente que quer usar instalado e com sessão iniciada (por exemplo `claude`).
- Rust (`cargo`) para instalar o `tm-agent`. Instaladores prontos: veja o [ROADMAP](https://github.com/polblancoo/crewhelm/blob/main/ROADMAP.md).

## Passo a passo

### 1. Criar a IA

Na barra lateral: **🤖 IAs → «+»**. Escolha um nome (`claude-backend`), o **modelo** (`claude:Opus`) e o papel **Membro**. Copie o **token** (`tmk_…`): é mostrado uma única vez. Se perder, clique na IA → **Novo token**.

### 2. Definir a pasta de trabalho

Com **⚙ Projeto** preencha **Pasta de trabalho** (por exemplo `/home/ana/projetos/app`) e o **repositório**. O caminho tem que **existir na máquina onde roda o `tm-agent`**. Se não, o agente avisa com um comentário na tarefa e não a pega (ou use `TM_AGENT_DIR`).

### 3. Instalar o `tm-agent`

Na máquina do projeto:

```bash
cargo install crewhelm-agent
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

Quando a IA pergunta, o servidor também **cria uma tarefa de pedido** para uma pessoa (quem criou a tarefa, se tiver a permissão **Executor**; senão o executor com menos tarefas abertas; sem executores, um admin). O comentário diz qual tarefa e que você pode responder ali mesmo; os comandos do pedido aparecem com um botão **📋 Copiar**. Veja o [guia, capítulo 10](https://github.com/polblancoo/crewhelm/blob/main/docs/GUIA.pt.md#10-quando-uma-ia-precisa-que-você-faça-algo).

**Exemplo pronto para usar.** Em [`examples/`](https://github.com/polblancoo/crewhelm/tree/main/examples) há um `run-claude.sh` com listas de comandos permitidos e proibidos (`whitelist.txt`, `blacklist.txt`) para usar como `TM_AGENT_CMD`.

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
