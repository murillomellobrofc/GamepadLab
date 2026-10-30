# Arquitetura

## Fluxo de dados

```
 controles físicos                       thread "gamepadlab-input" (~1,3 kHz)
 ─────────────────                       ──────────────────────────────────────────────
 gilrs (WGI / IOKit / evdev) ──┐
 hidapi: DS4, DualSense, ──────┼──► backends ──► Registry (dedupe) ──► Tracker (mín/máx, toques)
   Switch Pro (opt-in)         │        │                                   │
 DemoBackend (simulado) ───────┘        │                                   ├──► PollMeter (Hz, jitter, P99)
                                        │                                   ├──► Capture (assistente, taxa cheia)
                                        │                                   ├──► Health (saúde dos botões, bordas a taxa cheia)
                                        └──► OutputCommand (vibração, LED, gatilhos) ◄── HapticsEngine (100 Hz)
                                                                            │
 thread "gamepadlab-frames" (60/120 Hz) ◄── take_frame() ◄──────────────────┘
        │ aplica a calibração ativa (CalibratedView)
        ▼
 evento Tauri "gp://frame" ──► UI: barramento `live` (fora do React) ──► canvas/SVG em requestAnimationFrame
 eventos "gp://connection", "gp://controller-updated", "gp://error", "gp://rumble"
```

- A leitura nunca bloqueia a UI: o serviço roda numa thread própria, a UI recebe quadros no máximo a 60 ou 120 Hz
  (configurável) e a taxa real de polling é medida à parte, a cada relatório.
- Cada quadro leva o snapshot mais recente e a lista de intervalos desde o último quadro, então o gráfico de latência
  vê todos os relatórios mesmo com a UI a 60 Hz.
- Capturas do assistente gravam todas as amostras no backend (taxa cheia); a UI só recebe o resultado.
- Toques de botão são contados por borda de subida no backend (`buttonPresses`), então o teste guiado não perde
  toques mais curtos que um quadro.

## Dedupe

Um mesmo controle pode aparecer pelo gilrs e pelo HID cru (ou como XInput + DirectInput no Windows). O `Registry`
agrupa por VID:PID e esconde a cópia do backend de menor prioridade (HID 2 > gilrs 1 > demo 0). Se a cópia de maior
prioridade some, a outra volta a aparecer. Controles virtuais (Steam Input, ViGEm) e o demo nunca são escondidos,
mas recebem uma etiqueta.

## Sony via HID

Ler o relatório de calibração do IMU (feature report 0x05/0x02) faz o DualSense/DS4 Bluetooth trocar para o modo
estendido, que o leitor genérico do sistema não entende. Por isso, para Sony, o driver HID é a fonte principal:
botões, analógicos de 8 bits, gatilhos, gyro/acelerômetro calibrados (mesma fórmula do `hid-playstation` do Linux),
touchpad, bateria, e saída de vibração, LED, LEDs de jogador e gatilhos adaptativos (com CRC32 no Bluetooth).
No macOS o hidapi é aberto em modo compartilhado (`macos-shared-device`) para não "roubar" o controle do sistema.

## Calibração gravada no controle (firmware)

`crates/input/src/firmware/` contém a máquina de estados (`FirmwareSession`) e só depende da trait `FeatureIo`
(relatórios de feature, escrita e leitura). O driver HID implementa `FeatureIo` sobre o hidapi, com o tamanho de cada
relatório lido do descritor HID; os testes usam um dispositivo falso que grava os comandos enviados e emula a memória
SPI do Switch. O demo tem uma simulação com o mesmo contrato.

- Sony: `0x82/0x83` (DualSense) ou `0x90/0x91/0x92` (DS4) para centro e alcance; `0x80/0x81` ou `0xA0` para travar e
  destravar a NVS e reiniciar. Cada resposta é conferida contra o valor esperado da referência. Antes de calibrar, a
  NVS é colocada em modo travado (temporário); "salvar" é destravar + travar, exigido só depois de uma calibração na
  sessão e com `acknowledgeRisk`.
- Switch Pro: subcomandos `0x10` (ler SPI) e `0x11` (escrever SPI) com resposta no relatório `0x21`. A escrita é
  recusada fora de `0x8010–0x8025`; antes de escrever, o bloco atual vira backup (salvo em disco pelo `src-tauri`), e
  depois o bloco é lido de volta e comparado.

## Saúde dos botões

Só leitura: nada é enviado ao controle. O serviço de entrada grava toda borda de botão (apertou/soltou, com o
tempo do relatório) de um controle, como faz com as capturas do assistente, e anota se a fonte é por relatório
(HID e demo, com o intervalo mediano entre relatórios) ou por evento (gilrs, que só entrega mudanças de estado
na taxa de polling do sistema). A análise fica em `crates/calibration/src/button_health.rs`, pura:

- Bordas separadas por menos de `MERGE_GAP_US` (25 ms) formam um aperto físico; ninguém solta e aperta de novo
  tão rápido. Cada soltura extra dentro do aperto é um glitch: até `PRESS_SETTLE_US` do primeiro contato é
  **chatter**, até `RELEASE_SETTLE_US` da soltura final é **quique**, no meio é **soltou sozinho**.
- Toques (apertos menores que `HOLD_MIN_US`) são comparados com os pedidos: o que faltar é **falha de registro**.
- **Preso**: toque que demora `SLOW_RELEASE_FACTOR` vezes mais que o típico (e pelo menos `SLOW_RELEASE_US`),
  aperto de `STUCK_PRESS_US` ou mais, ou botão ainda apertado no fim da sessão.
- Cada sintoma vira um motivo com nota (atenção ou ruim, pelos limiares `GLITCH_BAD_*`, `MISSED_BAD_RATIO`,
  `SLOW_RELEASE_BAD_COUNT`) e uma dica: falhas leves de contato pedem limpeza; repetidas, trocar o switch;
  botão agarrando ou apertos perdidos apontam também para a tampa.

Os comandos `button_health_start/progress/stop/cancel` (`src-tauri/src/cmd_health.rs`) ligam a gravação, servem
a contagem ao vivo para a tela e rodam a análise no fim. Os fixtures em `fixtures/button_health/` cobrem cada
sintoma e um DualSense com o R1 ruim; o R1 do controle demo falha do mesmo jeito.

## Desenhos dos controles

Os desenhos são arte, não código: ficam em `apps/ui/public/controllers/<família>/` com licença própria
([ASSETS_LICENSE.md](../ASSETS_LICENSE.md)). DualSense (usado também no DualShock 4), Xbox Series e Switch Pro saem
dos pacotes do Zacksly (CC BY 3.0) por `scripts/build-controller-art.mjs`, que guarda o original intacto em `source/`
e lista cada mudança no cabeçalho do arquivo gerado; o genérico é desenho original. Nenhum desenho traz cor: o tema
pinta pelas classes `gl-body`, `gl-part`, `gl-glyph`, `gl-glyph-fill`, `gl-base`, `gl-seam`, `gl-detail`, `gl-hit`.

O `ControllerDiagram` busca o SVG, limpa (só elementos de desenho e geometria: nada de script, `on*`, `style`,
`foreignObject` ou link externo), troca cada `id` por `data-part` (vários desenhos podem dividir a página) e, a cada
quadro, mexe nas partes pelo ID:

| ID | O que o app faz |
|---|---|
| `btn-*`, `dpad-up/down/left/right`, `touchpad` | `data-on="true"` enquanto apertado (acende) |
| `stick-left`, `stick-right` | desloca pelo valor do analógico, até o raio de `stick-*-base` menos o do próprio círculo; acende no L3/R3 |
| `trigger-l2` (ou `-lt`, `-zl`…) | acende passado o limiar; `trigger-*-fill` (um `rect`) cresce com o valor e `trigger-*-value` mostra a % |
| `touch-0`, `touch-1` | posicionados dentro da área do `touchpad` para cada dedo |

O mapa índice do botão → ID de cada família fica em `apps/ui/src/features/test/layouts.ts`; partes de botões que o
modelo não tem (mute no DS4, Fn e traseiros fora do Edge, paddles fora do Elite) ganham `data-absent` e somem.

## Calibração

Ordem do pipeline (`crates/calibration/src/pipeline.rs`):
troca de eixos → subtrai o centro → escala por direção → inversão → deadzone (interna/externa, radial ou axial,
reescalada para não haver salto) → curva → sensibilidade → anti-deadzone → gate (círculo/octógono/quadrado).
A UI tem uma cópia em TypeScript só para a prévia ao vivo; o fixture `fixtures/calibration_golden.json` garante
que as duas dão o mesmo resultado.

## Perfis

Um arquivo por controle em `<dados do app>/profiles/<vvvv_pppp_serial>.json`, com `schemaVersion`. Migrações em
`crates/profiles/src/migrate.rs`. Exportação usa um envelope `{ format: "gamepadlab-profile", schemaVersion, profile }`.
Gravação atômica (arquivo temporário + rename); arquivo corrompido vira backup e o app recomeça com o padrão.

## Estratégia de testes

| Camada | Como |
|---|---|
| Matemática | dados sintéticos: círculo perfeito, achatado, drift conhecido, ruído, spikes, setor amassado, gate quadrado, limites |
| Saúde dos botões | fixtures JSON em `fixtures/button_health/` (saudável, chatter, falha de registro, preso, quique, R1 ruim) |
| Drivers HID | relatórios montados byte a byte (USB e Bluetooth), CRC, calibração do IMU |
| Serviço | `ScriptedBackend` gera eventos falsos: hotplug, dedupe, Hz, capturas, roteamento de saída, demo |
| IPC | `src-tauri/tests/ipc.rs` chama os comandos reais pelo runtime de mock do Tauri com os mesmos argumentos da UI |
| Contrato JSON | teste no Rust confere os nomes camelCase que os tipos TypeScript usam |
| UI | vitest (i18n, paridade TS/Rust, latência, ghosting, orientação, navegação espacial) |
| Fluxo | Playwright no Chromium com o transporte simulado (mesmo demo, relatórios gerados pelo Rust) |
