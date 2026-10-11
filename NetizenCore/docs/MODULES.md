Moduller

nc-hal               bootloader sozlesmesi (BootInfo), Platform arayuzu, log! makrosu
nc-arch              x86_64 port G/C, halt, 16550 seri port
nc-gfx               Canvas (bellekte piksel tamponu), cizim ilkelleri, framebuffer'a aktarim
nc-font              kendi vektor cizgi yazi tipi, piksel bagimsiz (geri cagirma ile cizer)
nc-splash            acilis ekrani, PNG logo hattini (build.rs) icerir
nc-platform-qemu     QEMU'ya ozel: debugcon, isa-debug-exit, Bochs/std VGA dispi, seri port
nc-platform-generic  genel PC: yalnizca seri port
kernel               giris (asm), kernel_main, panik, platform secimi

Ayrinti icin her modulun kendi MODULE.md dosyasina bak.
