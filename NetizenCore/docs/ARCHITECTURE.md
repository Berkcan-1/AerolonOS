NetizenCore mimarisi

Acilis akisi
  BIOS -> El Torito (no-emulation) -> stage1 (2 KB, CD'den stage2 + kernel'i okur)
  -> stage2 (A20, 64-bit kontrolu, VBE grafik modu, BootInfo, 4 GB sayfalama, long mode)
  -> kernel (_start: asm, kernel_main: Rust) -> platform baslat -> splash cizilir

Bellek haritasi
  0x0600  VBE kontrolcu bilgisi (gecici)
  0x0800  VBE mod bilgisi (gecici)
  0x0900  BootInfo (bootloader -> kernel sozlesmesi)
  0x1000  sayfa tablolari (PML4, PDPT, 4 adet PD) 0x6FFF'e kadar
  0x7C00  stage1 (ve acilis yigini)
  0x8800  stage2
  0x9800  kernel yuklu hali (gecici, 0x100000'a kopyalanir)
  0x100000  kernel (.text, .rodata, .data, .bss ve 64 KB yigin)
  0x400000  arka tampon (en fazla 1920x1200x4 bayt)
  Sayfalama: 0-3 GB onbellekli, 3-4 GB onbelleksiz (framebuffer ve MMIO), 2 MB sayfalar

BootInfo (adres 0x900, little-endian)
  0x00 u32 magic 0x4942434E
  0x04 u32 version 1
  0x08 u64 framebuffer adresi
  0x10 u32 pitch (bayt)
  0x14 u32 genislik
  0x18 u32 yukseklik
  0x1C u8 bpp
  0x1D u8 kirmizi konum, 0x1E kirmizi boyut
  0x1F u8 yesil konum, 0x20 yesil boyut
  0x21 u8 mavi konum, 0x22 mavi boyut
  Bu yapi hem bootloader/stage2.asm hem crates/nc-hal icinde tanimli, ikisi birlikte degisir.

Bagimlilik grafigi
  nc-hal        (bagimlilik yok)
  nc-arch       (bagimlilik yok)
  nc-font       (bagimlilik yok)
  nc-gfx        -> nc-hal
  nc-splash     -> nc-gfx, nc-font
  nc-platform-qemu     -> nc-hal, nc-arch
  nc-platform-generic  -> nc-hal, nc-arch
  kernel (nc-kernel)   -> hepsi, platform ozellik bayragina gore (qemu veya generic)

Profiller
  Profil kernel'in cargo ozelligidir. Ayni anda yalnizca biri secilir, aksi halde derleme hatasi verir.
  Yeni bir platform eklemek: crates altinda nc-platform-<ad> olustur, nc-hal::Platform'u uygula,
  kernel/Cargo.toml'a ozellik ekle, kernel/src/platform.rs'e bir cfg satiri ekle.

Kurallar
  Moduller yalnizca bagimlilik grafigindeki oklara gore birbirini kullanir.
  Donanima ozel kod yalnizca nc-arch ve nc-platform-* icinde olur.
  Ekrana cizen kod Canvas uzerinden cizer, dogrudan framebuffer'a yazmaz (sadece nc-gfx::present yazar).
  Kodlarda yorum satiri yok, aciklamalar bu dosyalarda ve MODULE.md dosyalarinda.

Yapay zeka araclariyla calisirken
  Bir modul uzerinde calisirken sadece su dosyalari ver: o modulun MODULE.md'si, src klasoru, ve
  bagimli oldugu modullerin MODULE.md dosyalari. Tum proje gerekmez.
