nc-platform-qemu

Gorev: QEMU sanal donanimina ozel Platform uygulamasi.
Bagimlilik: nc-hal, nc-arch.

Ne yapar
  write_log: debugcon (0xE9) ve seri port (COM1) uzerinden yazar
  shutdown(kod): isa-debug-exit (0xF4) ile QEMU'yu kapatir
  display_adapter_present(): Bochs/std VGA dispi kimligini dogrular
  set_display_mode(w, h): dispi yazmaclariyla cozunurlugu calisirken degistirir (32 bpp)

QEMU argumanlari: -vga std -serial stdio -debugcon file:build/debugcon.log
                  -device isa-debug-exit,iobase=0xf4,iosize=0x04
Sonraki adaylar: virtio-gpu, virtio-net, virtio-blk, virtio-input, fw_cfg, RTC.
