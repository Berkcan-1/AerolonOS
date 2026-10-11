kernel (nc-kernel)

Gorev: giris noktasi ve orkestrasyon.
Bagimlilik: nc-hal, nc-arch, nc-gfx, nc-splash ve secilen platform (ozellik: qemu veya generic).

Dosyalar
  src/entry.rs     _start (asm): yigini kurar, kernel_main'i cagirir
  src/platform.rs  ozellik bayragina gore aktif platformu secer
  src/main.rs      kernel_main: platform, BootInfo, arka tampon, splash, present; panik isleyicisi
  linker.ld        kernel 0x100000'a baglanir, .text.entry en basta
  build.rs         linker.ld'yi baglayiciya verir

Arka tampon adresi 0x400000 sabittir. Bellek yoneticisi eklenince bu alan ayrilmalidir.
