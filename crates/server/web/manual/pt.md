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

**O jeito mais fácil: o assistente.** Entre como CTO ou administrador em *Configurações → 🖥 Servidor (passo a passo) → ✉ E-mail*: escolha o provedor (Gmail, Outlook, Yahoo, Brevo, SendGrid, Mailgun, Amazon SES ou outro), digite usuário e senha, toque em **✉ Testar** (envia um e-mail de teste ao seu endereço) e depois **💾 Salvar e ativar**. Sem editar arquivos nem reiniciar: a senha é guardada criptografada e o servidor a usa na hora. Se algo falhar, o assistente explica o motivo (senha recusada, porta bloqueada, segurança errada…).

> O CrewHelm só **envia** e-mail. Não lê nenhuma caixa, então **POP3 ou IMAP não são necessários**: basta uma conta que envie por SMTP.

**Alternativa por arquivo** (servidores sem tela ou instalações por script). O que for salvo no assistente substitui estas variáveis.

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

Para **owners** (os membros usam o próprio espaço, veja a seção 21). O botão **💻 Terminal** da tarefa abre um console na pasta do projeto. Há dois jeitos de conectar (escolhido por projeto):

- **Pelo agente** (recomendado): rode `tm-agent` com `TM_AGENT_TERMINAL=1` na máquina do projeto. A conexão sai dessa máquina: sem chaves nem portas abertas, funciona atrás de NAT.
- **SSH com chave**: o app gera uma chave ed25519 (**🔑 Gerar chave**) e a instala com **🔐 Instalar chave com senha** (a senha não é guardada). A impressão digital do servidor é fixada na primeira vez e, se mudar, é rejeitada.

Cada sessão deixa um comentário 📝 e uma nota `.tm/terminal-tarea-N.md` na pasta do projeto, que a IA lê ao retomar. No menu, **🖥️ Terminais** mostra até 8 painéis e o estado de cada IA (🙋 precisa de você · ⚙️ trabalhando · ⏳ na fila · ✅ terminou).

![Vista Terminais](https://raw.githubusercontent.com/polblancoo/crewhelm/main/docs/screenshots/guia-14-terminales.png)

## 15. Consumo e cotas

**💰 Consumo** mostra tokens, custo e horas por IA e por tarefa, com barras de **cota** da equipe, do projeto e da tarefa. Numa tarefa, **🪙 Tokens a gastar** define um teto e **🧮 Estimar** sugere um valor pelo histórico. A cota é verificada **antes** de cada execução: se faltar, a IA não começa e avisa num comentário. Para cortar em dólares use `TM_AGENT_MAX_USD` no agente (1 USD ou mais). O Portfólio soma o consumo do mês.

![Consumo](https://raw.githubusercontent.com/polblancoo/crewhelm/main/docs/screenshots/guia-15-consumo.png)
![Teto de tokens](https://raw.githubusercontent.com/polblancoo/crewhelm/main/docs/screenshots/guia-17-tokens.png)

## 16. Licença comercial e aviso de instalação

O programa é **AGPL-3.0-only e grátis**. A **licença comercial** é para quem o usa numa empresa e não quer cumprir a AGPL. Em **Configurações → 🖥 Servidor (passo a passo) → Licença** (só a primeira pessoa registrada no servidor):

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

## 18. Comunicação: chat, videochamadas e tradução
O botão **💬 Comunicação** abre um painel com duas abas.

- **💬 Chat:** mensagens para a equipe, um projeto ou uma pessoa, com arquivos. Os links abrem com um clique. Tudo é guardado criptografado e só funciona por conexão segura (HTTPS).
- **📹 Videochamada:** *Entrar na sala* (a sala permanente da equipe), *Iniciar chamada* (cria uma sala nova e avisa na conversa com um botão «Entrar», visível só para quem participa) e *Convidar alguém de fora*.
- **Convidar alguém de fora:** gera um link único com validade (1 hora a 7 dias) para um fornecedor ou cliente. Não precisa de conta e não vê mais nada. Copie ou envie por e-mail (precisa do e-mail do servidor, seção 12). Pode ser cancelado quando quiser.
- **Configurar a videochamada (administrador):** *Configurações → 🔌 Integrações → Administrar*. O assistente oferece três caminhos: Jitsi público, Jitsi próprio na rede interna ou Jitsi próprio aberto à internet, com o bloco de instalação pronto para colar. Com o `meet.jit.si` público a chamada incorporada é cortada aos 5 minutos, então abre em uma aba própria.
- **Tradução do chat (administrador):** o botão **⚙🌐** a ativa. Cada pessoa escreve no seu idioma e lê no seu. Escolha o motor: *LibreTranslate próprio* (grátis, nada sai da sua rede; o botão «Instalar e ativar automaticamente» instala com Docker) ou *uma IA da equipe* (com seu limite diário de gasto). Cada pessoa liga ou desliga com o botão **🌐**. Ao traduzir, o texto vai para o motor escolhido.
- **Minimizar a chamada:** vira uma barra pequena no canto inferior direito com «▲ Voltar à chamada».

## 19. Pastas das tarefas
Cada tarefa tem sua pasta de trabalho no formato `.crewhelm/<usuário>/<tarefa>/`: cada pessoa ou IA guarda ali o seu. O resultado final é publicado em `final/`, compartilhado por toda a tarefa.

- Na tarefa, o bloco **Pasta** permite ver, baixar e enviar arquivos (até 25 MB; executáveis e scripts são recusados).
- **Quem cria a tarefa decide a aprovação** e a publicação em `final/`, numa tarefa avulsa ou num fluxo.
- Quem trabalha remotamente pode enviar seus arquivos dali.

## 20. Papéis do servidor e Processos
Além dos papéis de cada equipe, há três **papéis do servidor**: *Usuário*, *CTO* e *Administrador*. A primeira pessoa registrada é administradora. Um administrador os altera em *Configurações → 🖥 Servidor (passo a passo)* (sempre deve restar ao menos um administrador). O CTO também define ali os limites dos terminais e vê o uso por pessoa (seção 24).

- **🖥 Processos** (em *Configurações → 🖥 Servidor (passo a passo)*; CTO e administrador): RAM, disco, carga, processos do CrewHelm com CPU e RAM, contêineres e uso do app (banco, arquivos, pessoas, IAs, tarefas). Atualiza a cada 5 segundos.
- **Proteção contra saturação:** limite de pedidos por IP por minuto, limite de pedidos simultâneos e tempo máximo por pedido. Um ataque de rede grande se barra fora (firewall, proxy ou CDN). Atrás de um proxy, defina `TM_TRUSTED_PROXY=1`.
- **Atalhos da equipe:** o prefixo na web é **Alt+K** (o navegador fica com Ctrl+T).

## 21. Terminal das pessoas: cópia de trabalho e barra de git
Quando um owner ativa **«Permitir o terminal às pessoas da equipe»** (🖥️ Terminais → ⚙ Conexão), cada membro abre seu terminal em um sandbox:

- **Espaço pessoal** (`~`): é dele, gravável e se mantém entre sessões. Ninguém mais o vê (os owners podem olhá-lo somente leitura com **👁 Ver espaço de…**).
- **Cópia de trabalho** (`~/<projeto>`): uma cópia própria do projeto onde pode criar pastas e arquivos e usar editores. O original fica em `/proyectos/<projeto>`, somente leitura. Ao entrar, a cópia se **atualiza sozinha**.
- **Barra «🧭 Seu trabalho»** sob o terminal, pensada para quem não programa. Cada botão mostra no tooltip o comando de git equivalente: *Atualizar*, *Nova branch*, *Salvar*, *Enviar para revisão*, *Minhas branches*, *Trocar de branch*.
- **♻ Recuperar** (botão na barra do terminal, em *Configurações → Equipe e projeto* e no cabeçalho de *Terminais*): abre uma janela com botões, sem comandos. **Arquivos**: os que você apagou ou alterou sem guardar, os apagados em um salvamento anterior, ou qualquer outro pelo caminho; escolha a versão (por data e autor) e **↩ Trazer esta**. O que havia fica ao lado como `…antes-de-recuperar-HHMMSS`, então nada se perde. **Ramificações**: as suas atuais (**➡ Mudar para esta**), as que você apagou por engano (**♻ Recuperar**) e as que já enviou para revisão e não estão na sua cópia (**⬇ Trazer de novo**). Só olha a *sua* cópia de trabalho e só as suas ramificações; cada recuperação fica na auditoria. Digitado: `cw recuperar`.
- **Comando `cw`** (o mesmo, digitado): `cw estado`, `cw actualizar`, `cw rama`, `cw guardar`, `cw enviar`, `cw ramas`, `cw cambiar`, `cw recuperar`, `cw espacio` (quanto espaço você usa) e `cw ayuda`. Ordem normal: **branch → editar → salvar → enviar**.
- **Enviar para revisão** sobe a branch ao GitHub e abre o Pull Request **sem que a pessoa precise de conta no GitHub**: o servidor faz isso com as credenciais do owner. As branches de cada pessoa começam com o nome dela (`marta/tema`); não dá para forçar nem mexer nas de outros.
- Se o projeto **não tem repositório**, a barra avisa e oferece **🙋 Pedir repositório**, que notifica os owners.

## 22. Repositório do projeto (assistente de git)
Um owner conecta o repositório em **🐙 Repositório** (em 🖥️ Terminais), no aviso **⚠ Sem repositório** do cabeçalho do projeto ou, ao criar um projeto, na janela que abre sozinha. O assistente oferece:

- **⬇ Trazer um repositório do GitHub** para a pasta do projeto.
- **✨ Criar um novo** na sua conta do GitHub (privado por padrão) e enviar o que há na pasta, sem chaves nem pastas pesadas (`.gitignore` incluído).
- **🔗 Conectar** uma pasta que já é um repositório local com outro do GitHub.
- **💻 Só nesta máquina**: `git init` sem GitHub; dá para conectar depois.

Sua conta do GitHub precisa estar conectada (Configurações → Avisos e conexões) só durante a operação. Para enviar mudanças o servidor usa uma **chave de deploy por repositório**; dá para renová-la com 🔑. Cada projeto pode ter seu próprio repositório, local ou do GitHub.

## 23. Integrar: revisar e juntar o trabalho da equipe
O botão **🔀 Integrar** (cabeçalho do projeto) lista os Pull Requests abertos: quem enviou, branch, arquivos, linhas `+/−` e o estado (pronto, conflitos, testes falhando, bloqueado).

- **👁 Ver mudanças**: arquivo por arquivo, com o adicionado em verde e o apagado em vermelho.
- **✅ Integrar** (só owners): junta o trabalho à branch principal (opção «Juntar em uma única mudança»), apaga a branch da pessoa e **atualiza a pasta do projeto**; a pessoa recebe um aviso e roda `cw actualizar`.
- **↩ Devolver** (só owners): fecha o pedido sem juntar e envia à pessoa o motivo.
- Os owners recebem um aviso quando alguém envia seu trabalho. Rascunhos e pedidos com conflitos não são integrados.

## 24. Limites e cotas (Configurações → 🖥 Servidor (passo a passo))
O **CTO** e o **administrador** definem os limites dos terminais dos membros (os owners não têm limite); o agente de cada máquina os aplica aos terminais abertos a partir daí:

- **💾 Disco por pessoa** (5120 MB por padrão): cheio, o terminal não abre; aos 80 % avisa; ao passar de 125 % fecha. Medido a cada 90 segundos (não é um limite rígido).
- **🧠 Memória** (4096 MB), **⚙ CPU** (200 % = dois núcleos) e **🧵 Processos** (1024): aplicados com grupos de controle do Linux (`systemd-run`). Se um programa passar da memória, só ele é encerrado. Sem systemd, o terminal abre sem esses três limites.
- **📊 Ver uso por pessoa**: quanto cada espaço ocupa em cada máquina.
- Com `0` em um campo não há limite. Os valores iniciais também podem ser definidos com `TM_PERSON_QUOTA_MB`, `TM_PERSON_MEM_MB`, `TM_PERSON_CPU_PCT` e `TM_PERSON_TASKS`.

## 25. Convidar e instalar na máquina de outra pessoa
Em **👥 Equipe → Convidar com um código**, **Copiar convite** monta uma mensagem pronta para colar num chat: o endereço do servidor (com o IP da rede local se você entrou por `localhost`), o código e os passos. O mais fácil é **abrir o endereço no navegador**, sem instalar nada.

Para o app de desktop no Linux, sem servidor: `curl -fsSL https://raw.githubusercontent.com/polblancoo/crewhelm/main/install.sh | sh -s -- --client --server http://IP:7878`. Instala só o app (o serviço e as ferramentas de terminal ficam de fora); ao abrir, informe o endereço em «Servidor». Em Configurações cada pessoa vê só o que lhe cabe: GitHub pessoal só para admins (ou se já vinculou), Google só se o servidor o tem, e a lista de comandos das IAs só para o administrador.

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

## 26. Primeiros passos do servidor

Em *Configurações → 🖥 Servidor (passo a passo)* (CTO e administrador) há uma lista do que falta ao servidor e um assistente para cada peça. Todos testam os dados com o provedor antes de salvar, guardam os segredos criptografados e valem na hora, sem editar arquivos nem reiniciar.

**Tudo o que é geral do servidor está nesta aba, em ordem:** 1 ✉ E-mail · 2 🐙 Código e GitHub · 3 ✈ Telegram · 4 📅 Google Calendar · 🔒 HTTPS · Segurança (papéis e verificação em duas etapas) · Regras das IAs (aprovação por risco, comandos e limites dos terminais) · Processos · Licença. **Sua conta** (*Configurações → 👤 Conta* e *🔔 Avisos e conexões*) tem só o que é seu: seu e-mail, Telegram, GitHub, Google, avisos e sessões. O que é de cada equipe está em *👥 Equipe e projeto*.

- **🐙 Código e GitHub — onde o código fica, projeto por projeto.** O administrador escolhe a que se aplica com um combo: **Todos os projetos (padrão)** ou um projeto em particular (que pode *herdar* o padrão ou ter o seu). Cada um é **GitHub permitido** (o projeto decide: só no servidor, ou no servidor e também GitHub para colaborações abertas) ou **Tudo no servidor** (o GitHub é cortado nesse projeto: repositórios, issues, Pull Requests, e as IAs com cápsula não alcançam mais github.com nem gitlab.com; pensado para clínicas, consultórios e escritórios de advocacia). **Se o projeto já estava no GitHub, primeiro tudo (ramificações e etiquetas) é baixado para a pasta do servidor e só depois é cortado**; se não for possível baixar (por exemplo, o agente do projeto está desconectado), nada é cortado e você é avisado. O envio (`git push`) de qualquer terminal também é cortado; ao permitir o GitHub de novo, ele volta. O CTO vê, mas não altera. Enquanto um projeto está em «tudo no servidor» não há Pull Requests do GitHub: o envio para revisão sem GitHub ainda não está disponível.

- **✉ E-mail** (quase imprescindível): sem ele ninguém verifica o e-mail nem recebe avisos por e-mail. Veja a seção 12.
- **✈ Telegram**: crie um bot com o @BotFather (`/newbot`), copie o token e cole. Depois cada pessoa se vincula em *Configurações → Telegram*.
- **🐙 GitHub**: crie um *OAuth App* em github.com → Settings → Developer settings, marque *Enable Device Flow* e cole o *Client ID*. Assim cada pessoa conecta a própria conta.
- **📅 Google Calendar**: habilite a *Google Calendar API* no console do Google, crie um *ID do cliente OAuth* (aplicativo da Web) com o endereço de redirecionamento que o assistente mostra e cole o ID e o segredo. O Google exige HTTPS (ou localhost).
- **🔒 HTTPS**: escolhido ao instalar o servidor; a lista só mostra se está ativo.

O que for salvo no assistente substitui as variáveis de ambiente (`TM_TELEGRAM_TOKEN`, `TM_GITHUB_CLIENT_ID`, `TM_GOOGLE_*`, `TM_SMTP_*`); *Remover* volta ao que houver no ambiente.

## 27. Auditoria e limites de gasto

**Auditoria exportável** (*Configurações → Equipe e projeto → Registro de auditoria*, só administradores e donos): baixa em CSV ou JSON a linha do tempo completa da equipe, com datas opcionais (de / até). Inclui quem (pessoa ou IA) fez o quê e quando, as permissões dadas às IAs, revisões e devoluções, mudanças de limite e de cota, cada execução de IA com modelo, tokens e custo, e a atividade do chat. Não é cortada com o tempo, e cada download fica registrado no mesmo registro. No CSV, as células que começam com `=`, `+`, `-` ou `@` são neutralizadas para que a planilha não as execute.

**Limites de gasto**: uma tarefa pode ter um limite de tokens, e o projeto e a equipe uma cota mensal. Com o limite esgotado, a IA **não inicia** outra execução (corte duro) até que alguém o aumente. Ao chegar a **80 %** e quando se esgota, donos e administradores recebem um aviso na hora (caixa de entrada, Telegram e e-mail) e fica na auditoria como `alerta_presupuesto`. Uma execução em andamento termina; o corte vale antes da próxima.

## 28. Aprovação por risco e mudanças dentro da tarefa

**Aprovação por risco** (*Configurações → 🤖 IAs → Aprovação por risco*, CTO e administrador): em vez de montar listas de comandos, você escolhe por tipo de ação se as IAs fazem **livremente**, precisam que alguém **peça/aprove**, ou é **proibida**. Tipos: apagar (rm, git clean, git reset --hard…), publicar (git push, npm/cargo publish, releases, merge de PRs), implantar (kubectl, terraform, ssh, rsync…), instalar (npm/pip/cargo install, apt, curl, wget…) e administrar o sistema (sudo, chmod, systemctl…). Em *Pedir* a IA para e avisa você; ao aprovar com `/permitir <comando>` só essa regra é habilitada (`rm`, não `rmdir`) para essa tarefa, ou para essa IA se escolher «sempre». O proibido sempre vence. Vale a partir da próxima execução de cada IA e cada mudança fica na auditoria (`politica_riesgo`).

**Ver mudanças dentro da tarefa**: se a tarefa tem um Pull Request, sua janela mostra **👁 Ver mudanças** com os arquivos e linhas que a IA alterou, sem passar por 🔀 Integrar. Integrar e devolver continuam sendo do projeto.

## 29. Pasta de rede: salvar dos seus próprios programas

*Configurações → Equipe e projeto → Pasta de rede → Como conectar* mostra o endereço do projeto (`/dav/<número>/`) e os passos para montá-la como unidade no **Windows** (Mapear unidade de rede), **macOS** (Ir → Conectar ao Servidor) ou **Linux** (Arquivos → Conectar ao servidor). Você entra com seu usuário e senha do CrewHelm e salva do Photoshop, Premiere, Word, DaVinci ou o que usar: o arquivo chega à pasta do projeto no servidor, sem passar pelo navegador e sem o limite de 25 MB (até 8 GB por arquivo).

- **Quem escreve onde**: dentro da pasta com seu nome (`<seu usuário>/…`). A equipe toda lê tudo; `_tareas/<n>/final/` é somente leitura. Leitores não escrevem.
- **Versões**: se você salvar por cima de um arquivo, movê-lo ou apagá-lo, o CrewHelm guarda antes a versão anterior (as últimas 20 por arquivo, em `.crewhelm/.versions/`).
- **Programas executáveis não são enviados** (`.exe`, `.bat`, instaladores, etc.).
- **O Windows exige HTTPS**: use o «Endereço seguro» e instale o certificado próprio do servidor (link na mesma ajuda); com um certificado real (Let's Encrypt ou o da empresa) não é preciso. O endereço HTTP serve no macOS e Linux numa rede confiável.
- Cada envio, movimento ou exclusão fica na auditoria (`carpeta_de_red`).
- Funciona para projetos cuja pasta de trabalho está na máquina do servidor.

## 30. Arquivos da tarefa: pastas, versões, links e o perfil de design

Na janela de uma tarefa, *📂 Pasta da tarefa* (aberta por padrão nos perfis sem partes técnicas) permite:

- **Enviar vários arquivos ou uma pasta inteira**, também **arrastando-os** para a área pontilhada (até 25 MB cada; para os maiores, use a pasta de rede da seção 29). As subpastas são mantidas.
- **Ver sem baixar** (👁) imagens e textos.
- **🕘 Versões**: cada vez que um arquivo é salvo por cima, a anterior é mantida (as últimas 20). Qualquer pessoa da equipe pode baixá-las; só o dono do arquivo pode *Voltar a esta*, e a atual também fica guardada. É o mesmo histórico que a pasta de rede cria.
- **🔗 Adicionar link**: um link para Drive, Figma, Dropbox ou um vídeo na nuvem como entregável, sem enviar nada. Abre com um clique.

**Perfil 🎨 Design e audiovisual** (*Configurações → Equipe e projeto → Perfil de uso*): colunas *Brief / Em produção / Entregue*, sem partes técnicas, com uma campanha de exemplo. **Para quem programa**: trabalhe no seu editor num clone local e envie com `git`/`cw enviar` (seção 21); a pasta de rede (seção 29) também serve para abrir o projeto no editor.

## 31. Verificação em duas etapas (2FA)

*Meu perfil → 🔐 Verificação em duas etapas → Ativar*: você escaneia um QR code com um app autenticador (Google Authenticator, Authy, 1Password, Aegis…), digita o código de 6 dígitos que ele mostra e pronto. A partir daí, ao entrar é pedido o código depois da senha. Mesmo que alguém roube a senha, não consegue entrar.

- **Códigos de recuperação**: ao ativar são mostrados 8, uma só vez. Cada um serve uma vez no lugar do código do app. Guarde-os à parte (gerenciador de senhas, papel).
- **Um código não é usado duas vezes**, e são aceitos o da janela atual e o anterior/seguinte (caso o relógio do celular esteja um pouco errado). Após 5 tentativas falhas a conta fica bloqueada por 15 minutos.
- **Desativar** pede a senha e um código válido (a sessão aberta sozinha não basta).
- **Pasta de rede (seção 29)**: não pode pedir códigos, então com 2FA ativo não aceita sua senha normal. Gere uma **senha de aplicativo** em *Meu perfil → 🔑 Senha para a pasta de rede* e use-a ao conectar (mostrada uma vez; gerar outra anula a anterior).
- **As IAs** entram com seu token, não com senha: o 2FA não as afeta.
- **Se alguém perder o celular e os códigos**: o administrador do servidor vai em *Configurações → 🖥 Servidor (passo a passo)* e toca em *🔓 Remover 2FA* na linha da pessoa; isso também encerra as sessões. Fica registrado no servidor e na auditoria das equipes dela.
- O terminal de texto (`tm`) ainda não pede o código: com 2FA ativo, use a web ou o app de desktop.

**Pedir à equipe (administrador do servidor)**: em *Configurações → 🖥 Servidor (passo a passo) → 🔐 Pedir verificação em duas etapas* há três interruptores **separados**: *Administradores (incluindo você)*, *CTO* e *Demais pessoas*. É um **pedido, não um bloqueio**: quem se aplica e ainda não ativou recebe um aviso (caixa de entrada, Telegram e e-mail), é lembrado ao entrar e no perfil, e você vê quantos têm e quem falta. Ninguém fica fora do app por não ativar. Só o administrador do servidor altera isto; o CTO apenas vê.
