# Changelog

## Não lançado

## 0.2.2

- **Saúde dos botões** (nova aba no teste, e atalho no resultado do teste guiado): aperte cada botão escolhido
  N vezes (padrão 20) no seu ritmo e segure uma vez. O app mede chatter, apertos que não chegam, botão preso ou
  que demora a soltar, quique ao soltar e botão que solta sozinho, dá uma nota por botão (boa, atenção, ruim)
  com os motivos e diz o que fazer: limpar, trocar a tampa ou trocar o switch. Avisa quando o controle é lido
  pelo sistema (gilrs), onde chatter rápido pode passar batido; pelo HID mostra o intervalo entre relatórios.
- Lógica pura em `crates/calibration/src/button_health.rs`, com limiares em constantes e fixtures em
  `fixtures/button_health/` (saudável, chatter, falha de registro, preso, quique e um DualSense com o R1 ruim).
- O serviço de entrada grava as bordas dos botões na taxa cheia, sem enviar nada ao controle; comandos
  `button_health_start/progress/stop/cancel`.
- Resultado no histórico (tipo "Saúde dos botões") e nos relatórios PNG, PDF e JSON.
- O controle demo ganhou um R1 com defeito para mostrar a análise.
- Correção: um clique logo ao abrir o app (antes de a conexão interna terminar de carregar) era perdido; agora
  espera. Era a causa dos testes de ponta a ponta instáveis.
- Correção: a lista de controles agora é atualizada antes do aviso de conexão ou desconexão, então quem reage ao
  aviso já vê a lista nova (fazia um teste do serviço falhar às vezes no Windows).

## 0.2.1

- Versão portátil para Windows (`GamepadLab_x.y.z_x64-portable.zip`): roda sem instalar e, com o `PORTABLE.txt`
  ao lado do `.exe`, guarda dados, logs e configurações na pasta `GamepadLab-data`, sem deixar nada no perfil do
  Windows. Configurações explicam que ela se atualiza baixando o `.zip` novo. O diagnóstico mostra o modo portátil.
- `npm run version:set` também atualiza o `PKGBUILD` do Arch.

## 0.2.0

- Espanhol: interface inteira traduzida, detecção automática pelo idioma do sistema e datas no idioma escolhido.
- Novos instaladores: Windows `.exe` (além do `.msi`, agora também em espanhol), `.rpm` (Fedora/openSUSE),
  Flatpak e pacote do Arch (`gamepadlab-bin`, pronto para o AUR). O `.rpm` também instala a regra udev.
- Atualização pelo app só onde ela funciona: no Linux, instalações por apt, dnf, pacman ou Flatpak são
  atualizadas pelo gerenciador de pacotes, e as Configurações explicam isso.
- Abas do topo cabem inteiras em janelas de largura média em todos os idiomas.
- CI no Windows: os testes de integração ganharam o manifesto de controles comuns que faltava.
- Atualização automática ativada: chave de assinatura gerada, progresso do download, mensagens claras quando não
  há versão publicada ou falta internet, `npm run version:set` e checagem de tag no `release.yml`.
- Correções do primeiro teste com controle real: botão Create do DualSense agora se chama Share, contador do
  teste guiado mostra o botão atual, nomes longos cabem no círculo, anel de cobertura e bússola com tamanho fixo.
- Calibração gravada no controle: DualShock 4 e DualSense (centro e alcance com os comandos de fábrica, modo
  temporário primeiro, gravação permanente com confirmação, descartar reiniciando) e Switch Pro experimental
  (calibração do usuário na memória SPI, com backup e verificação). Travas de USB, clone, firmware, bateria e
  DualSense Edge. Simulação no controle demo.

## 0.1.0 — primeira versão

- Teste completo de controles: detecção e hotplug, até 8 controles, layouts Xbox/DS4/DualSense/Switch Pro/genérico,
  dados crus com mín/máx, teste guiado de botões, ghosting/rollover, vibração com padrões, gatilhos adaptativos e
  LEDs do DualSense, latência/polling, giroscópio, acelerômetro e touchpad.
- Calibração por software: assistente em 5 passos, análises de drift, circularidade, direções, spikes e gatilhos,
  nota de qualidade, sugestão automática, comparação antes/depois, ajustes manuais com prévia ao vivo e perfis.
- Overlay para stream, histórico em SQLite, relatórios PNG/PDF/JSON, temas, alto contraste, PT-BR/EN,
  navegação pelo controle, modo demo, diagnóstico copiável, updater configurável.
