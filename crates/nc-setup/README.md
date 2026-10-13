# nc-setup

Kurulum sihirbazi. Bagimlilik: nc-hal, nc-gfx, nc-font, nc-ui. Dort adim: WELCOME, DISKS (kurulum hedefi secimi), INSTALL
(ilerleme cubugu), FINISH (RESTART).

## Genel API

- SetupState::new(&[BlockDeviceInfo]), update(mouse, key, width, height) -> SetupAction (None, Shutdown, Reboot)
- draw(canvas, platform, state, mouse)

## Davranis

- Sol: dikey zaman cizgisi (aktif, tamamlanan, bekleyen adim). Sag: kart (baslik, icerik, alt satirda READY rozeti,
  platform, BACK ve NEXT/RESTART dugmeleri).
- DISKS adiminda algilanan diskler listelenir (HDD veya ROM rozeti, model, boyut); tikla veya yukari/asagi ile secilir.
  Secim su an gosterim amaclidir: kernel yazmayi her zaman kanal 0'daki sabit diske yapar.
- INSTALL adimi ilerleme cubugunu zamanla doldurur; asil yazma islemini kernel/src/main.rs yapar.
- Klavye: Enter/Space ileri, Escape geri, Up/Down disk secimi.
