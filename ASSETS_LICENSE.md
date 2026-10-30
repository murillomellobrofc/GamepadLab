# Licenças das artes

O **código** do GamepadLab é MIT (veja [LICENSE](LICENSE)). Os **desenhos de controle** ficam separados do código, em
[`apps/ui/public/controllers/`](apps/ui/public/controllers/), e têm licenças próprias, descritas aqui.

## Desenhos do Zacksly — CC BY 3.0

| Arquivo usado no app | Pacote original | Arquivo de origem no pacote |
|---|---|---|
| `controllers/ps5/dualsense.svg` (também usado para o DualShock 4) | PS5 Button Icons and Controls | `Controller Images/Outline/Outline White SVG.svg` |
| `controllers/xbox/xbox.svg` | Xbox Series Button Icons and Controls | `Controller Images/Outline/Outline White SVG.svg` |
| `controllers/switch/switch-pro.svg` | Switch Button Icons and Controls | `Controller Images/Pro Controller/Outline/Pro Controller Outline White SVG.svg` |

- **Autor:** Zacksly — <https://zacksly.itch.io>
- **Licença:** Creative Commons Attribution 3.0 Unported (CC BY 3.0) —
  <https://creativecommons.org/licenses/by/3.0/>
- **Crédito:** "PS5 Button Icons and Controls", "Xbox Series Button Icons and Controls" e "Switch Button Icons and
  Controls" por Zacksly, licenciados sob CC BY 3.0 (zacksly.itch.io). **Modificados** pelo GamepadLab.
- Os arquivos originais, sem nenhuma alteração, estão em `controllers/<família>/source/`, com o `LICENSE.txt` do
  autor ao lado.

**O que o GamepadLab mudou** (os arquivos usados no app são gerados por
[`scripts/build-controller-art.mjs`](scripts/build-controller-art.mjs), e cada um repete esta lista no cabeçalho):

- tirou a folha de estilo e as cores embutidas, para o tema do app (escuro, claro, alto contraste) colorir o desenho;
- deu um ID a cada parte que o app acende ou move (`btn-cross`, `stick-left`, `trigger-l2`, `touchpad`…) e agrupou
  os símbolos com os seus botões;
- recortou a área do desenho;
- no Pro Controller, removeu o logotipo e o nome "Nintendo Switch";
- acrescentou partes desenhadas pelo GamepadLab: os medidores dos gatilhos analógicos (L2/R2, LT/RT, ZL/ZR), as áreas
  de cada direção do D-pad no Xbox e no Pro Controller (que no original é uma peça só), os pontos de toque do
  touchpad e os botões extras do DualSense Edge (Fn e traseiros) e do Xbox Elite (P1/P2).

## Desenhos originais do GamepadLab — CC BY 4.0

- `controllers/gamepadlab/generic.svg` (controle genérico) e as partes acrescentadas aos desenhos acima.
- © Murillo Mello e colaboradores do GamepadLab, licença Creative Commons Attribution 4.0 International (CC BY 4.0) —
  <https://creativecommons.org/licenses/by/4.0/>.
- O controle genérico foi desenhado do zero no mesmo estilo de traço; não copia nenhum dos desenhos acima.

## Ícone e logotipo do GamepadLab

O símbolo do app (`assets/gamepadlab-dark.svg` e `assets/gamepadlab-light.svg`, e o que sai deles:
`src-tauri/icons/`, `apps/ui/public/brand/` e `apps/ui/public/favicon.svg`) foi criado por Murillo Mello para o
GamepadLab e identifica o projeto. As versões de 1024 px em `assets/` só acrescentam a margem da grade de ícones do
macOS; o desenho é o mesmo.

## Marcas

PlayStation, DualSense, DualShock, Xbox, Nintendo Switch e os respectivos logotipos são marcas dos seus donos e
aparecem só para identificar o controle. O GamepadLab não tem ligação com a Sony, a Microsoft, a Nintendo nem com o
Zacksly, e não é endossado por eles.

---

### English

GamepadLab's code is MIT. The controller drawings live apart from it in `apps/ui/public/controllers/`.
The DualSense (also used for the DualShock 4), Xbox Series and Switch Pro drawings are based on "PS5 Button Icons and
Controls", "Xbox Series Button Icons and Controls" and "Switch Button Icons and Controls" by Zacksly
(<https://zacksly.itch.io>), licensed under CC BY 3.0 (<https://creativecommons.org/licenses/by/3.0/>), and were
**modified** by GamepadLab as listed above; the untouched originals and the author's licence are in each family's
`source/` folder. The generic controller and the parts GamepadLab added are original work under CC BY 4.0.
Trademarks belong to their owners; GamepadLab is not affiliated with or endorsed by them or by Zacksly.
