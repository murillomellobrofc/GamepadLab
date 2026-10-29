# Windows

## Como os controles são lidos
- **Windows.Gaming.Input** (via gilrs): controles Xbox com gatilhos separados e vibração dos dois motores, e os
  demais como RawGameController. Isso evita o problema clássico do DirectInput (gatilhos do Xbox num eixo só) e a
  duplicata XInput + DirectInput.
- **HID cru** (hidapi) para DualShock 4 e DualSense: giroscópio, touchpad, bateria, LED e gatilhos adaptativos, sem
  precisar de DS4Windows. O registro de dispositivos esconde a cópia vista pelo sistema.
- XInput direto não é usado: o WGI já cobre os controles Xbox, e o XInput não enxerga controles não-Xbox.

## Conflitos comuns
- **Steam Input** remapeia e pode duplicar ou esconder controles: o app avisa quando o Steam está aberto.
- **DS4Windows / HidHide** podem esconder o controle físico e mostrar um virtual (ViGEm). Desative-os para testar.

## Compilar
Visual Studio Build Tools (C++), Rust MSVC e WebView2 (já incluso no Windows 11). `npm run tauri build` gera o MSI
(WiX é baixado automaticamente) em `target/release/bundle/msi/`, um para pt-BR e outro para en-US.

## SmartScreen e assinatura
Instaladores sem assinatura disparam o aviso do SmartScreen ("Mais informações → Executar assim mesmo"). Para
assinar, configure `bundle.windows.certificateThumbprint` (ou `signCommand`) no `tauri.conf.json` com o seu
certificado de assinatura de código.
