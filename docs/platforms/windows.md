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

## Versão portátil
O `GamepadLab_x.y.z_x64-portable.zip` traz só o `GamepadLab.exe` e o `PORTABLE.txt`. Enquanto o `PORTABLE.txt`
estiver ao lado do `.exe`, tudo o que o app grava (dados, logs e o perfil do WebView2, onde ficam idioma e tema)
vai para a pasta `GamepadLab-data` ali mesmo, e nada vai para `%APPDATA%` ou `%LOCALAPPDATA%`. Apagar o
`PORTABLE.txt` volta para as pastas normais do usuário. A versão portátil não se atualiza sozinha: baixe o `.zip`
novo e troque o `.exe`. Precisa do WebView2 Runtime (já vem no Windows 10 atualizado e no 11).

## Compilar
Visual Studio Build Tools (C++), Rust MSVC e WebView2 (já incluso no Windows 11). `npm run tauri build` gera o
instalador `.exe` em `target/release/bundle/nsis/` e os MSI (WiX é baixado automaticamente) em
`target/release/bundle/msi/`, um para cada idioma (pt-BR, en-US, es-ES).

## SmartScreen e assinatura
Instaladores sem assinatura disparam o aviso do SmartScreen ("Mais informações → Executar assim mesmo"). Para
assinar, configure `bundle.windows.certificateThumbprint` (ou `signCommand`) no `tauri.conf.json` com o seu
certificado de assinatura de código.
