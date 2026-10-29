<p align="center"><a href="https://github.com/polblancoo/Rust_tuiManager/blob/main/docs/GUIDE.md">English</a> · <a href="https://github.com/polblancoo/Rust_tuiManager/blob/main/docs/GUIA.md">Español</a> · <b>Português</b></p>

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

---

## 1. Instalar o servidor

O servidor guarda tudo (equipes, tarefas, usuários) e serve a web. Instala-se uma vez.

1. Baixe `tm-server-linux-x86_64.tar.gz` da [última versão](https://github.com/polblancoo/Rust_tuiManager/releases/latest) (ou `cargo install rust-tui-manager-server`).
2. Descompacte e execute `./tm-server`.
3. Abra `http://localhost:7878` no navegador.

Para que outras pessoas entrem de outras máquinas, coloque o servidor num endereço que elas alcancem (idealmente com HTTPS: veja a [configuração do servidor](https://github.com/polblancoo/Rust_tuiManager/blob/main/README.pt.md)). Se for só para testar, `localhost` basta.

Se preferir terminal ou janela própria, também há um cliente de terminal e um app de desktop na mesma página de downloads.

## 2. Criar sua conta e sua equipe

1. Na tela inicial escolha **Criar conta** (usuário e senha de 8 caracteres ou mais).
2. Na barra lateral, **Equipes → «+»**, coloque o nome da sua organização (por exemplo «Mi empresa»). Você fica como **Dono**.

Uma **equipe** é o grupo de pessoas e IAs que compartilham projetos. Você pode ter várias.

## 3. Criar os projetos

Em **Projetos → «+»** crie um para cada coisa que queira levar adiante. No exemplo: um app, uma planilha e um processo, cada um com sua cor.

![Quadro do projeto «App móvel de reservas»](https://raw.githubusercontent.com/polblancoo/Rust_tuiManager/main/docs/screenshots/guia-01-tablero.png)

Cada projeto tem seu **quadro** (A fazer · Em andamento · Concluído). Com **+ Nova tarefa** você carrega o trabalho: título, descrição, prioridade, datas e responsável. Os mesmos dados aparecem também em **calendário**, **Gantt** e **lousa**.

![Outro projeto: uma planilha de orçamento, com responsáveis humanos e de IA](https://raw.githubusercontent.com/polblancoo/Rust_tuiManager/main/docs/screenshots/guia-08-otro-proyecto.png)

Cada projeto guarda **onde vive o seu trabalho**: com **⚙ Projeto** (admin ou dono) você define a pasta de trabalho e o repositório, e eles aparecem acima do quadro.

![Editar o projeto: nome, pasta de trabalho e repositório](https://raw.githubusercontent.com/polblancoo/Rust_tuiManager/main/docs/screenshots/guia-09-proyecto.png)

> 💡 Dica: não precisa ser software. Uma planilha, um processo de onboarding ou uma auditoria se tocam do mesmo jeito: tarefas com responsável e data.

## 4. Adicionar pessoas

Botão **👥 Equipe** (canto superior direito) → **➕ Adicionar / convidar**.

- **Já tem conta**: digite parte do nome, escolha o papel e **Adicionar**.
- **Ainda não tem conta**: escolha um papel e **Gerar código**. Use **Copiar convite** (texto pronto para WhatsApp ou e-mail com link de download, servidor e código) ou **Enviar por e-mail**.

![Convidar com um código](https://raw.githubusercontent.com/polblancoo/Rust_tuiManager/main/docs/screenshots/guia-07-invitar.png)

| Papel | O que pode fazer |
|---|---|
| **Leitor** | Olhar. |
| **Membro** | Criar, editar e comentar tarefas; pegar as livres. |
| **Admin** | Além disso, atribuir tarefas a outros, adicionar pessoas e gerar convites. |
| **Dono** | Tudo, inclusive transferir ou excluir a equipe. |

O painel da equipe mostra cada integrante com seu papel, desde quando está, quem o adicionou e quantas tarefas abertas tem:

![Painel da equipe com pessoas e IAs](https://raw.githubusercontent.com/polblancoo/Rust_tuiManager/main/docs/screenshots/guia-06-panel-equipo.png)

## 5. Adicionar IAs

As IAs (Claude Code, Cursor, Windsurf, Zed, Gemini ou outro cliente compatível com [MCP](https://modelcontextprotocol.io)) trabalham **como mais um integrante**. Quantas precisar, de modelos diferentes.

1. Na barra lateral, **🤖 IAs → «+»**.
2. Coloque um nome que diga o que ela é (`claude-backend`, `gemini-dados`) e o papel: **Membro** (cria e edita as suas tarefas) ou **Leitor** (só olha).

   ![Nova IA na equipe](https://raw.githubusercontent.com/polblancoo/Rust_tuiManager/main/docs/screenshots/guia-02-nueva-ia.png)

3. Mostra-se **o token uma única vez**, com a configuração MCP pronta para copiar. É a «chave» dessa IA.

   ![Token da IA e configuração MCP](https://raw.githubusercontent.com/polblancoo/Rust_tuiManager/main/docs/screenshots/guia-03-token.png)

4. Se perder ou quiser cortar: clique na IA → **Novo token** (o anterior deixa de valer) ou **Revogar**.

   ![Ficha de uma IA](https://raw.githubusercontent.com/polblancoo/Rust_tuiManager/main/docs/screenshots/guia-03-ia.png)

**O que uma IA pode e não pode fazer**

- ✅ Criar tarefas e **atribuí-las a pessoas** da equipe, comentar, concluir ou apagar **as que ela criou ou que lhe atribuíram**.
- ❌ Criar ou apagar equipes, convidar pessoas, mudar papéis, ou mexer em tarefas alheias (exceto comentar).
- Tudo o que faz fica **em seu nome** na atividade, e aparece com 🤖 no painel.

> 🚧 **Estado**: as contas de IA e seus tokens já funcionam contra a API do servidor. O binário `tm-mcp` e o *skill* que as conectam direto à sua plataforma de código estão em desenvolvimento (veja o [ROADMAP](https://github.com/polblancoo/Rust_tuiManager/blob/main/ROADMAP.md)). Enquanto isso, o **`tm-agent`** já permite que uma IA pegue sozinha as tarefas atribuídas: veja o [guia de agentes](https://github.com/polblancoo/Rust_tuiManager/blob/main/docs/AGENTES.pt.md).

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

![Gantt do projeto, com tarefas de pessoas e de IAs](https://raw.githubusercontent.com/polblancoo/Rust_tuiManager/main/docs/screenshots/guia-04-gantt.png)

![Atividade: o histórico do projeto, com as ações das IAs em seu nome](https://raw.githubusercontent.com/polblancoo/Rust_tuiManager/main/docs/screenshots/guia-05-actividad.png)

Além disso, o **Google Calendar** (suas tarefas com data aparecem na sua agenda) é conectado em **Configurações**.

## 8. Delegar a um líder

Hoje, para delegar um projeto: adicione a pessoa com o papel **Admin** (pode atribuir tarefas, adicionar gente e gerar convites) e atribua a ela as tarefas principais. Ela monta a sua parte e traz a sua gente.

> 🚧 **Em breve**: **líder por projeto** (com alcance só nesse projeto), **escopo configurável por IA** (em quais projetos mexe, se pode convidar outras IAs), **relatórios de andamento** e um **painel de portfólio** com todos os seus projetos de uma vez. Veja o [roteiro](https://github.com/polblancoo/Rust_tuiManager/blob/main/ROADMAP.md).

---

**Algo não ficou claro?** Cada opção de **Configurações** tem um «?» com ajuda. E se encontrar um problema, abra uma [issue](https://github.com/polblancoo/Rust_tuiManager/issues).
