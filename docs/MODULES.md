# Moduller

| Modul               | Gorev                                                                       |
|---------------------|-----------------------------------------------------------------------------|
| nc-hal              | bootloader sozlesmesi (BootInfo), Platform arayuzu, log! makrosu            |
| nc-arch             | x86_64 port G/C, halt, 16550 seri port, PIT/TSC saati                       |
| nc-math             | tam sayi trigonometri, karekok, yumusatma ve oran fonksiyonlari             |
| nc-mem              | basit bellek havuzu (Arena)                                                 |
| nc-assets           | PNG logo hatti (build.rs) ve logo erisimi                                   |
| nc-gfx              | Canvas, cizim ilkelleri, bulaniklastirma, karistirma, framebuffer aktarimi  |
| nc-font             | kendi vektor cizgi yazi tipi (A-Z, 0-9, noktalama)                          |
| nc-ui               | tema renkleri, zemin, metin, kure, hap, golge, kart yardimcilari            |
| nc-splash           | animasyonlu acilis ekrani                                                   |
| nc-input            | PS/2 klavye ve fare (InputDevice uygulamasi)                                |
| nc-storage          | ATA PIO disk tarama, okuma/yazma, kurulum (MBR, stage2, cekirdek yazimi)    |
| nc-app              | uygulama sozlesmesi (App trait, Context, Input, tema, ikonlar)              |
| nc-app-settings     | Ayarlar uygulamasi (duvar kagidi, vurgu rengi, cozunurluk bilgisi)          |
| nc-app-system       | Sistem bilgisi uygulamasi                                                   |
| nc-app-stopwatch    | Kronometre uygulamasi                                                       |
| nc-apps             | yerlesik uygulamalarin listesi (builtin)                                    |
| nc-desktop          | masaustu kabugu: pencereler, gorev cubugu, baslat menusu                    |
| nc-setup            | kurulum ekrani (Welcome)                                                    |
| nc-platform-qemu    | QEMU'ya ozel Platform uygulamasi                                            |
| nc-platform-generic | genel PC Platform uygulamasi                                                |
| kernel              | giris noktasi, acilis akisi, panik isleyicisi                               |

Her modulun klasorunde ayrintili README.md, src klasorunde dosya dosya aciklama vardir.
