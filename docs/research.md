# Pesquisa (Passo 0)

## Site 1 — PG Custom, Calibrador de Analógicos
<https://www.pgcustom.com.br/calibrador/calibrador-controle>

Fork em PT-BR do projeto open-source *DualShock Calibration GUI* (MIT, the_al).
Funciona só com DualShock 4, DualSense, DualSense Edge e VR2, via **WebHID** (Chrome/Edge).

Fluxo e telas:
- Tela inicial com aviso de que o calibrador **não conserta drift mecânico**.
- Modal de boas-vindas (bateria carregada, testar calibração temporária antes da permanente).
- Botão Conectar → abas *Calibração*, *Informações*, *Depuração*.
- **Calibrar centro**: rápido ou em 4 etapas (mover os dois analógicos para cada canto e soltar).
- **Calibrar alcance**: girar os analógicos 2× em cada sentido; botão Concluído liberado após ≤15 s; modo normal e especialista.
- **Ajuste fino**: centro (L1) e circularidade (R1) ajustados pelo D-pad, passo de 1 a 15; meta de erro médio de circularidade entre 7 e 9 % (a Sony sai de fábrica com ~10 %, e jogos esperam isso).
- "Salvar alterações permanentemente" (grava no firmware) ou temporário (perde ao reiniciar).
- Visualização LX/LY/RX/RY em modo normal e zoom 10×, "Verificar circularidade" (software/hardware).
- Restaurar calibração, reiniciar controle, status e lock/unlock da NVS.
- Detecção de clone (DS4), informações do controle (firmware, placa, serial).
- Teste rápido com resumo, histórico de calibrações (local), ~22 idiomas, FAQ.

## Site 2 — hardwaretester.com/gamepad
<https://hardwaretester.com/gamepad>

Usa a **HTML5 Gamepad API** (qualquer navegador, qualquer controle).
- Até 4 slots ("1: None detected"), seleção por clique, mini-barras de atividade por eixo/botão.
- Por controle: nome (+ Vendor/Product), INDEX, CONNECTED, MAPPING, TIMESTAMP, POSE, HAPTIC ACTUATORS, VIBRATION.
- Botões `B0…Bn` com valor analógico de 0 a 1; eixos `AXIS n`.
- SVG de controle Xbox que acende quando o mapping é "standard" (≥16 botões e 4 eixos).
- L STICK / R STICK com ponto e **Test Circularity**: 32 setores (π/16); erro = RMS(1 − r) dos setores.
- Vibração: "1 sec" e "infinite" (`dual-rumble`, magnitude 1).
- Eventos de conexão/desconexão, FAQ, ranking de controles mais usados (coleta estatísticas anônimas).

## Matriz de funções

| Função | PG Custom | hardwaretester | GamepadLab |
|---|:-:|:-:|:-:|
| Qualquer controle | — | ✔ | ✔ (gilrs) |
| Vários controles | — | ✔ (4) | ✔ (8) |
| Botões/eixos crus | parcial | ✔ | ✔ + mín/máx |
| SVG do controle | — | Xbox | Xbox, DS4, DS5, Switch Pro, genérico |
| Vibração | — | ✔ (fixa) | ✔ motores separados, padrões |
| Circularidade | ✔ | ✔ | ✔ + setores, raios, cobertura |
| Calibração de centro/alcance | ✔ (firmware) | — | ✔ (software, perfis) |
| Ajuste fino | ✔ D-pad | — | ✔ deadzone, curvas, gate, anti-deadzone |
| Gatilhos | (planejado) | valor | ✔ análise + ajustes + adaptativos DS5 |
| Info do controle | ✔ | parcial | ✔ VID:PID, conexão, bateria |
| Histórico | ✔ calibrações | — | ✔ SQLite, todas as sessões |
| i18n | ✔ 22 idiomas | — | PT-BR, EN |
| Latência/Hz, gyro, touchpad, teste guiado, ghosting, overlay | — | — | ✔ |

Decisão: o GamepadLab **não grava no firmware** (risco de brickar e não dá pra desfazer, como o próprio PG Custom avisa).
A calibração é aplicada por software (dentro do app e, no futuro, num controle virtual).

## Como cada sistema expõe controles

### Windows
- **XInput**: só controles compatíveis com Xbox, até 4 slots, polling (`XInputGetState`), gatilhos separados (0–255), 2 motores (`XInputSetState`), bateria (`XInputGetBatteryInformation`). Não tem gyro nem gatilhos com vibração.
- **DirectInput / HID**: qualquer HID. Controles Xbox aparecem com **os dois gatilhos num eixo Z só** e sem vibração, e o mesmo controle aparece também no XInput (daí as duplicatas).
- **Windows.Gaming.Input (WGI)**: `Gamepad` (Xbox, gatilhos separados, motores e *impulse triggers*) e `RawGameController` (o resto). É o backend padrão do gilrs no Windows, e já evita a duplicata XInput + DirectInput.
- Steam Input pode criar um controle virtual (VID 28DE) e esconder ou duplicar o real.

### macOS
- **IOKit HID** (`IOHIDManager`): acesso de baixo nível a qualquer HID. É o que gilrs e hidapi usam. Alguns dispositivos disparam a permissão **Input Monitoring** (TCC `ListenEvent`), checável com `IOHIDCheckAccess`. O hidapi abre em modo **exclusivo** por padrão, por isso o app usa `macos-shared-device`.
- **GameController.framework**: `GCController`/`GCExtendedGamepad`, perfis ricos (`GCDualSenseGamepad` com gatilhos adaptativos, `GCMotion`, touchpad, bateria, luz). Só Objective-C/Swift, fica como opção futura.

### Linux
- **evdev** (`/dev/input/event*`): `EV_KEY`, `EV_ABS`, `EV_FF` (vibração), timestamps por evento. É o que o gilrs usa. A permissão costuma vir de `uaccess`/grupo `input`.
- **joydev** (`/dev/input/js*`): API legada, sem vibração.
- **hidraw** (`/dev/hidraw*`): relatórios HID crus (gyro, touchpad, LED e gatilhos do DS4/DS5). Precisa de regra udev. O kernel (`hid-playstation`, `hid-nintendo`) também expõe sensores como evdev separado.
- **uinput** (`/dev/uinput`): criar controle virtual (fase futura), precisa de regra udev.

## Consequências para a arquitetura
1. **gilrs** é a base para todo controle "comum" (Xbox, genéricos, Switch Pro sem sensores).
2. **hidapi** é o driver principal para Sony (DS4/DS5): botões, analógicos crus de 8 bits, gyro, acelerômetro, touchpad, bateria, LED, gatilhos adaptativos e Hz real por relatório. Ler o relatório de calibração pelo Bluetooth troca o controle para o modo estendido, e aí o gilrs para de entender. Por isso o HID **substitui** o gilrs nesse caso e o registro faz *dedupe* por VID:PID + serial.
3. A taxa de polling real é medida pelo intervalo entre relatórios no HID. No gilrs, é medida pelo intervalo entre mudanças de estado (precisa mexer o controle).
