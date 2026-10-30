# Contribuindo com o GamepadLab

Obrigado por ajudar! Este guia mostra como o projeto é organizado e o que esperamos de cada mudança.

## Ambiente

```bash
npm install
npm run tauri dev        # app completo
npm run dev              # só a interface, no navegador, com dados simulados
```

Sem controle por perto? Use o **modo demo** (botão na barra lateral).

## Organização

| Pasta | O que tem |
|---|---|
| `crates/input` | modelo de dados, trait `GamepadBackend`, backends (gilrs, HID, demo, scripted), registro com dedupe, serviço de entrada, medição de polling |
| `crates/calibration` | toda a matemática de calibração e a análise de saúde dos botões, sem I/O |
| `crates/profiles` | perfis em JSON, validação, migração |
| `crates/haptics` | padrões de vibração e efeitos de gatilho |
| `src-tauri` | comandos, eventos, histórico, overlay, logs |
| `apps/ui` | interface React |
| `fixtures` | valores de referência compartilhados entre Rust e TypeScript, e os casos de saúde dos botões |

## Regras

- `npm run lint` e `npm run format:check` precisam passar (clippy com `-D warnings`, eslint, prettier, rustfmt).
- Toda mudança na matemática de calibração vem com teste em `crates/calibration/tests/analysis.rs`; na saúde dos
  botões, com um caso em `fixtures/button_health/` (o teste lê todos os arquivos da pasta).
- Se mudar o pipeline de calibração, os padrões de vibração, a saúde dos botões ou o controle demo, regenere os
  fixtures e confira o diff:
  ```bash
  GAMEPADLAB_WRITE_GOLDEN=1 cargo test -p gamepadlab-calibration --test golden
  GAMEPADLAB_WRITE_GOLDEN=1 cargo test -p gamepadlab-input --test demo_reports
  ```
- Novos textos entram em **pt-BR, en e es** (`apps/ui/src/i18n/locales/<idioma>/*.json`); o teste de paridade falha se
  faltar chave ou se os `{marcadores}` forem diferentes.
- Acessibilidade não é opcional: todo controle tem rótulo, funciona por teclado, e nada depende só de som, cor ou tempo.
- Nada de telemetria, analytics ou requisições de rede além do updater.

## Adicionar suporte a um controle

1. VID/PID e família em `crates/input/src/mapping.rs` (com teste).
2. Se o sistema não expõe tudo (gyro, LEDs…), um `Driver` em `crates/input/src/hid/` com parser testado por relatórios sintéticos.
3. Um layout em `apps/ui/src/features/test/layouts.ts` e os nomes dos botões em `apps/ui/src/lib/buttonNames.ts`.
4. Atualize a tabela de recursos no README.

## Commits e PRs

Mensagens no imperativo, uma ideia por commit. No PR, descreva o que mudou, como testou (e em qual sistema/controle)
e cole o **diagnóstico** se for correção de bug.
