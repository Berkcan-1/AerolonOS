# nc-setup

Kurulum ekrani. Bagimlilik: nc-gfx, nc-font, nc-ui. Su an tek ekran: Welcome. Etkilesim yoktur.

## Ekran duzeni

- Ust: NETIZENCORE (sol, koyu), SETUP (sag, soluk).
- Sol: dikey zaman cizgisi ve dort adim: WELCOME (aktif: azure cam kure, koyu yazi), DISKS, INSTALL, FINISH (pasif: kucuk
  gri nokta, soluk yazi).
- Sag: beyaz cam kart (yumusak golgeli). Icinde STEP 1 OF 4, buyuk WELCOME basligi, altinda azure -> aqua hap, iki satir
  metin (SETUP IS READY. / NEXT: DETECT DISKS.), altta aqua kure + READY ve sagda PLATFORM <ad>.
- Alt sol: NETIZENCORE ve surum (Cargo paket surumu).

Olculer ekran yuksekliginden turetilir (unit = yukseklik/36, en az 10); farkli cozunurluklerde orantili kalir.

## Genel API

- draw(&mut Canvas, &SetupInfo { platform })

## Yeni adim eklemek

Su an adimlar sabit bir listedir (STEPS, NUMBERS). Disk algilama gelince DISKS adimi icin ayri bir fonksiyon yazilmali
ve draw yerine bir ekran yoneticisi (hangi adim gorunuyor) kullanilmalidir. Klavye girdisi gelince secim ve gecis eklenecek.
