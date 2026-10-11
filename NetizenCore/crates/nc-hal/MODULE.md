nc-hal

Gorev: bootloader ile kernel arasindaki sozlesme ve platform arayuzu.
Bagimlilik: yok.

Genel API
  BOOT_INFO_ADDRESS (0x900), BOOT_INFO_MAGIC, BOOT_INFO_VERSION
  FramebufferInfo { address, pitch, width, height, bpp, red/green/blue position ve size }
  BootInfo { magic, version, framebuffer } ve BootInfo::read_from_boot_address()
  trait Platform { name, init, write_log, shutdown }
  log!(&platform, "bicim", ...) makrosu (seri/debug cikisina yazar)

Dikkat: BootInfo alan duzeni bootloader/stage2.asm icindeki fill_boot_info ile birebir ayni olmali.
