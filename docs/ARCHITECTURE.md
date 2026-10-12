# NetizenCore mimarisi

## Acilis akisi

    BIOS -> El Torito (no-emulation) -> stage1 -> stage2 -> cekirdek (_start asm, kernel_main Rust)

kernel_main sirasi:

1. Platformu baslat (seri port, debugcon vb.), BootInfo'yu oku.
2. PIT ve TSC ile saati kalibre et (nc-arch::time::Clock).
3. 0x400000'daki bellek havuzundan (Arena) ekran, gecis ve kurulum tamponlarini ayir.
4. Zemini (backdrop) ciz ve ekrana ver; sonra Splash'i hazirla (katmanlar ve bulanik seviyeler).
5. Animasyon dongusu: Preparation hazir olana kadar Intro ve Idle; hazir olunca Outro. Her karede sadece degisen
   dikdortgenler framebuffer'a yazilir.
6. Kurulum ekranini ciz ve ekrana yumusak gecisle getir. Orada dur.

## Bellek haritasi

    0x0600   VBE kontrolcu bilgisi (gecici)
    0x0800   VBE mod bilgisi (gecici)
    0x0900   BootInfo
    0x1000   sayfa tablolari (0x6FFF'e kadar)
    0x7C00   stage1
    0x8800   stage2
    0x9800   cekirdek yuklu hali (gecici)
    0x100000 cekirdek (.text .rodata .data .bss, 64 KB yigin)
    0x400000 bellek havuzu (Arena), 60 MB, 0x4000000'a kadar

Bellek havuzu su an su sekilde dolar (1280x720'de yaklasik 25 MB):

    ekran tamponu      3.7 MB   (u32 piksel, 0x00RRGGBB)
    gecis tamponu      3.7 MB
    kurulum tamponu    3.7 MB
    splash katmanlari  yaklasik 14 MB (zemin kopyasi + 4 bulaniklik seviyesi + gecici)

Sayfalama: 0-3 GB onbellekli, 3-4 GB onbelleksiz, 2 MB sayfalar. Cekirdek en az 64 MB RAM bekler
(E820 okunana kadar). QEMU ve VirtualBox icin 256 MB verilir.

## Bagimlilik grafigi

    nc-hal                 (yok)
    nc-arch                (yok)
    nc-math                (yok)
    nc-mem                 (yok)
    nc-assets              (yok; build.rs: png)
    nc-gfx                 -> nc-hal
    nc-font                -> nc-math
    nc-ui                  -> nc-gfx, nc-font, nc-math
    nc-splash              -> nc-assets, nc-gfx, nc-font, nc-math, nc-mem, nc-ui
    nc-setup               -> nc-gfx, nc-font, nc-ui
    nc-platform-qemu       -> nc-hal, nc-arch
    nc-platform-generic    -> nc-hal, nc-arch
    kernel (nc-kernel)     -> hepsi (platform ozellik bayragina gore biri)

## Profiller

Profil, kernel kutusunun cargo ozelligidir: qemu veya generic. Ikisi birden ya da hicbiri secilirse derleme hatasi
verir (kernel/src/platform.rs). Yeni platform eklemek icin:

1. crates/nc-platform-<ad> olustur, nc-hal::Platform'u uygula.
2. Kok Cargo.toml uye listesine ekle.
3. kernel/Cargo.toml'a ozellik ve istege bagli bagimlilik ekle.
4. kernel/src/platform.rs'e bir cfg satiri ekle.
5. build.sh icindeki configure_profile'a cozunurluk ayarini ekle.

## Kurallar

- Moduller sadece bagimlilik grafigindeki oklara gore birbirini kullanir.
- Donanima ozel kod sadece nc-arch ve nc-platform-* icinde olur.
- Ekrana cizen kod bir Canvas'a cizer. Framebuffer'a sadece nc-gfx::present ve present_region yazar.
- Ekran tamponlari nc-mem::Arena'dan alinir; modullerin kendi sabit adresleri olmaz.
- Kodlarda yorum satiri yok. Aciklamalar README.md dosyalarinda.
- Yeni bir crate eklenince kok Cargo.toml'daki uye listesine elle eklenir (glob kullanilmaz).

## Yapay zeka araclariyla calisirken

Bir modul uzerinde calisirken: o modulun README.md'si ve src klasoru, bagimli oldugu modullerin README.md dosyalari.
