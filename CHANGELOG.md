# Changelog

## Não lançado

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
