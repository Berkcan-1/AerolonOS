NetizenCore

Profiller
  qemu     QEMU icin: 1280x720 tercih edilir, seri port + debugcon + isa-debug-exit kullanilir
  generic  Genel BIOS/VBE: 1024x768, sadece seri port, VirtualBox ve cogu PC icin

Gereksinimler (Debian/Ubuntu)
  sudo apt install nasm xorriso binutils qemu-system-x86 zip
  rustup kurulu olmali (build.sh hedefi kendisi ekler)

Komutlar
  bash build.sh            qemu ISO uretir: build/netizencore-qemu.iso
  bash build.sh generic    generic ISO uretir: build/netizencore-generic.iso
  bash build.sh all        ikisini birden uretir
  bash build.sh run        qemu ISO uretir ve QEMU'da acar
  bash build.sh vbox       generic ISO uretir ve VirtualBox'ta acar
  bash build.sh clean      build klasorunu siler

Logo
  assets/logo klasorune bir PNG koy (ilk PNG alfabetik sirayla secilir), tekrar derle.
  256 pikselden buyukse otomatik kucultulur. Yoksa logosuz acilir.
  Baska klasor icin: NC_LOGO_DIR=/yol bash build.sh

QEMU hata ayiklama
  Seri cikti terminalde gorunur, debugcon cikti build/debugcon.log dosyasina yazilir.
  Cekirdek panik olursa mesaj ikisine de yazilir.

Dokumanlar
  docs/ARCHITECTURE.md   acilis akisi, bellek haritasi, bagimlilik grafigi
  docs/MODULES.md        her modulun gorevi ve genel API'si
  crates/*/MODULE.md     tek bir modulu yapay zekaya verirken kullanilacak ozet
