# Linux

## Dependências para compilar
```bash
sudo apt install libwebkit2gtk-4.1-dev libappindicator3-dev librsvg2-dev patchelf libudev-dev build-essential
```
(Fedora: `webkit2gtk4.1-devel libappindicator-gtk3-devel librsvg2-devel systemd-devel`.)

## Permissões (udev)
O leitor comum (evdev, via gilrs) costuma funcionar para o usuário logado. Para giroscópio, touchpad, LED, bateria
exata e gatilhos do DualSense, o app precisa do **hidraw**, que pede uma regra udev. O `.deb` já instala
`/usr/lib/udev/rules.d/70-gamepadlab.rules`. Para AppImage ou build manual:

```bash
sudo cp packaging/linux/70-gamepadlab.rules /etc/udev/rules.d/
sudo udevadm control --reload-rules && sudo udevadm trigger
```
Depois desconecte e reconecte o controle. A regra também libera `/dev/uinput` para o futuro controle virtual.

Se nenhum controle aparece: confira se seu usuário lê `/dev/input/event*` (grupo `input` ou `uaccess`) e se o `udev`
está rodando (containers e algumas distros mínimas não têm).

## Wayland e X11
A leitura dos controles vem do kernel (evdev/hidraw), então funciona igual nos dois. O overlay usa uma janela
transparente "sempre no topo": no X11 e no KDE/Wayland funciona; no GNOME/Wayland o compositor pode ignorar o
"sempre no topo" (use o fundo chroma e capture a janela no OBS).

## Pacotes
`npm run tauri build` gera `.deb` e `.AppImage` em `target/release/bundle/`.

## Steam
Com o Steam aberto, o Steam Input pode criar um controle virtual (aparece com a etiqueta "virtual") e esconder o real.
Feche o Steam ou desative o Steam Input para testar o controle de verdade.
