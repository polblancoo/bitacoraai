<p align="center"><a href="https://github.com/polblancoo/bitacoraai/blob/main/crates/tui/README.md">English</a> · <a href="https://github.com/polblancoo/bitacoraai/blob/main/crates/tui/README.es.md">Español</a> · <b>Português</b></p>

# bitacora

> **0.6.3**: licença **AGPL-3.0-only**; o servidor ganha terminal no navegador, consumo e cotas.
>
> **0.6.2**: o cliente de terminal agora está em **espanhol, inglês e português** (o idioma vem de `client.json` ou de `$LANG`). Site e downloads: https://polblancoo.github.io/rust_tuiManager/

> 🎛️ **Painel de comando para os seus projetos**: dirija vários projetos ao mesmo tempo —uma fábrica de apps, uma planilha, um onboarding, o que for— com **pessoas e IAs no mesmo quadro**. Cada projeto sabe onde vive o seu trabalho (pasta e repositório). Self-hosted.

### O que o torna diferente

O que não vimos junto em nenhuma outra ferramenta (confira, e avise se achar uma):

- 🤖 **IAs como integrantes da equipe**, com papel, lista de comandos proibidos e permissões que uma pessoa dá na própria tarefa (`/permitir cargo build`).
- 🙋 **Pedidos de ajuda da IA** que chegam como tarefa para a pessoa certa.
- 💻 **Terminal SSH dentro da tarefa** (com chave), e o que foi feito fica escrito para a IA retomar dali. *(0.6.3)*
- 📊 **Portfólio de vários projetos**, com custo e horas da IA. *(custo e horas: chegam na 0.6.3)*
- 🏠 **Auto-hospedado**: seus dados e suas chaves ficam no seu servidor.

Cliente de terminal (ratatui) do gerenciador de tarefas colaborativo [BitacoraAI](https://github.com/polblancoo/bitacoraai). Precisa de um servidor [`bitacora-server`](https://crates.io/crates/bitacora-server) em execução.

> **Versões 0.2 em diante**: este crate deixou de ser um gerenciador local de notas e snippets e passou a ser um cliente de rede. O código anterior está na pasta `legacy/` do repositório e nas versões 0.1.x.

![Quadro no terminal](https://raw.githubusercontent.com/polblancoo/bitacoraai/main/docs/screenshots/tui-board.png)

![Detalhe de uma tarefa: subtarefas, etiquetas e comentários](https://raw.githubusercontent.com/polblancoo/bitacoraai/main/docs/screenshots/tui-detail.png)

> 👉 **[Guia passo a passo: seu primeiro projeto](https://github.com/polblancoo/bitacoraai/blob/main/docs/GUIA.pt.md)**, com capturas de tela.

## 🤖 Agentes de IA como integrantes da equipe

Os assistentes de código (Claude Code, Cursor, Windsurf, Zed ou qualquer cliente MCP) podem **entrar numa equipe humana**: cada IA tem seu usuário, token, papel e modelo (ex.: `claude:Opus`), cria e conclui tarefas, **atribui-as a pessoas** e recebe as que você atribuir. São adicionadas pela barra lateral do app web ou de desktop (**🤖 IAs → «+»**); o token é mostrado uma vez e é trocado ou revogado com um clique. Uma IA não pode administrar equipes, convidar nem mexer em tarefas alheias. Detalhes no [README principal](https://github.com/polblancoo/bitacoraai/blob/main/README.pt.md); com o **`tm-agent`** uma IA pega sozinha as tarefas que você atribuir (veja o [guia de agentes](https://github.com/polblancoo/bitacoraai/blob/main/docs/AGENTES.pt.md)); o binário `tm-mcp` está em desenvolvimento. Neste cliente, as contas de IA aparecem como membros.

## Uso

```sh
cargo install bitacora
bitacora http://127.0.0.1:7878
```

Na tela de acesso, `F2` alterna entre entrar e criar conta. A sessão é guardada em `~/.config/rust-tui-manager/client.json` (permissões `0600`) e o servidor é lembrado para a próxima vez.

## Atalhos

**Quadro**

| Tecla | Ação |
|---|---|
| `h` `l` / `←` `→` | Mudar de coluna |
| `j` `k` / `↑` `↓` | Mover a seleção |
| `Enter` | Abrir detalhe (subtarefas, etiquetas, comentários) |
| `n` | Nova tarefa |
| `e` | Editar o título |
| `d` | Apagar a tarefa |
| `H` / `L` | Mover a tarefa de estado |
| `+` / `-` | Subir ou baixar a prioridade |
| `a` | Pegar a tarefa ou soltar a minha |
| `A` | Atribuir a um membro, com sugestões (admin ou dono; `-` remove) |
| `N` | Caixa de entrada: novidades e minhas tarefas (abre sozinha ao iniciar a sessão) |
| `Tab` / `Shift+Tab` | Projeto seguinte ou anterior |
| `[` / `]` | Equipe anterior ou seguinte |
| `P` / `T` | Novo projeto / nova equipe |
| `m` | Adicionar membro com busca: `↑↓` escolher, `Tab` completa, `Ctrl+R` muda o papel |
| `Ctrl+T` + letra | Prefixo de equipe: `m` membro · `r` papel · `i` convidar · `j` entrar · `t` equipe · `e` meu e-mail · `v` verificar e-mail · `s` mensagem à equipe · `g` vincular Telegram · `n` caixa de entrada |
| `R` | Mudar o papel de um membro: `usuario papel` |
| `I` | Gerar um código de convite (papel de membro, 7 dias, 10 usos) |
| `J` | Entrar numa equipe com um código |
| `X` | Encerrar a sessão em todos os dispositivos |
| `C` | Menu de configurações: e-mail, Telegram, caixa de entrada, mensagens e equipe, sessão |
| `?` | Ajuda com busca de atalhos |
| `q` | Sair |

**Detalhe de uma tarefa**

| Tecla | Ação |
|---|---|
| `s` | Nova subtarefa |
| `j` `k` | Escolher subtarefa |
| `Espaço` / `x` | Marcar ou desmarcar a subtarefa |
| `d` | Apagar a subtarefa |
| `l` | Pôr ou tirar uma etiqueta (é criada se não existir) |
| `c` | Comentar |
| `Esc` | Voltar |

O cabeçalho marca os papéis: `★` dono, `⚙` admin, `👁` leitor. O ponto verde no canto indica conexão ao vivo.

Mais informações, segurança e configuração do servidor no [README do projeto](https://github.com/polblancoo/bitacoraai/blob/main/README.pt.md).

## Licença

**AGPL-3.0-only.** Grátis para usar e auto-hospedar. Se você oferecer uma versão modificada como serviço, precisa publicar suas mudanças com a mesma licença. Há licença comercial sem essa obrigação: pergunte nas [issues](https://github.com/polblancoo/bitacoraai/issues). As versões publicadas até a 0.6.2 continuam em MIT.
