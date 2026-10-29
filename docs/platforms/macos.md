# macOS

## Permissões
- **Monitoramento de Entrada** (Privacidade e Segurança): controles comuns funcionam sem ela, mas alguns aparelhos que
  também se apresentam como teclado só aparecem depois de liberar o GamepadLab. O app avisa no topo e tem um botão que
  abre o painel certo.
- Bluetooth: pareie em Ajustes do Sistema → Bluetooth (DualSense e DS4: PS + Share; Xbox: botão de
  pareamento; Switch Pro: Sync).

## Apple Silicon e Intel
```bash
rustup target add x86_64-apple-darwin aarch64-apple-darwin
npm run tauri build -- --target aarch64-apple-darwin
npm run tauri build -- --target x86_64-apple-darwin
# ou um binário universal:
rustup target add x86_64-apple-darwin && npm run tauri build -- --target universal-apple-darwin
```

## Primeira abertura (sem assinatura)
Builds locais não são assinadas: clique com o botão direito no app → Abrir, ou
`xattr -dr com.apple.quarantine /Applications/GamepadLab.app`.

## Assinatura e notarização
1. Certificado **Developer ID Application** exportado como `.p12`.
2. Variáveis (localmente ou como segredos do GitHub, usados pelo `release.yml`):
   - `APPLE_CERTIFICATE` (o `.p12` em base64), `APPLE_CERTIFICATE_PASSWORD`
   - `APPLE_SIGNING_IDENTITY` (ex.: `Developer ID Application: Seu Nome (TEAMID)`)
   - `APPLE_ID`, `APPLE_PASSWORD` (senha de app), `APPLE_TEAM_ID`
3. `npm run tauri build` assina e o Tauri envia para notarização quando as variáveis existem.

O app não usa sandbox nem entitlements especiais. O overlay transparente usa `macOSPrivateApi`, o que impede a
distribuição pela Mac App Store (a distribuição direta com Developer ID funciona normalmente).

## Detalhes técnicos
- Leitura: gilrs sobre IOKit HID; DualShock/DualSense pelo hidapi em modo compartilhado.
- Controles Xbox 360 com fio não têm driver nativo no macOS.
- O botão Home/PS pode ser capturado pelo sistema (Game Center). Para ligar a navegação pelo controle, segure
  Select + Start.
