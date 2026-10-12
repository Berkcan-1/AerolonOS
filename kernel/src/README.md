# kernel/src

## main.rs

kernel_main (no_mangle, extern C) ve panic_handler. Sabitler:

- ARENA_BASE 0x400000, ARENA_SIZE 0x3C00000: bellek havuzu.
- FRAME_MS 16: kare hedefi.
- TRANSITION_MS 600: splash'ten kurulum ekranina gecis suresi.

new_canvas: Arena'dan genislik*yukseklik piksellik tampon ayirip Canvas yapar.
Panik olunca platform yeniden baslatilir ve mesaj log'a yazilir, sonra durulur.

## entry.rs

global_asm! ile _start yazar: kesmeleri kapatir, yon bayragini temizler, 64 KB'lik .bss.stack yiginini kurar,
kernel_main'i cagirir. Donerse durur. Bu dosya sadece mod olarak eklenir; icinde Rust kodu yoktur.

## platform.rs

Ozellik bayragina gore Active tipini secer (QemuPlatform veya GenericPlatform) ve create() ile uretir.
Yanlis ozellik kombinasyonunda compile_error verir.

## prepare.rs

Preparation: kurulum hazirliginin durumu. poll(now) hazir olunca true doner. Su an sadece MIN_DURATION_MS (4500)
kadar bekler. Disk algilama gibi gercek adimlar eklenince poll, adimlar bitmeden ve minimum sure dolmadan true donmemelidir.
Her cagri kisa surmeli (bir kare butcesi icinde); uzun isler parcalara bolunmelidir.
