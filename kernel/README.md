# kernel

nc-kernel kutusu: cekirdegin giris noktasi ve orkestrasyonu. Cikti ikilisinin adi kernel'dir; build.sh bunu
objcopy ile duz ikili dosyaya cevirir.

## Dosyalar

| Dosya        | Gorev                                                                        |
|--------------|------------------------------------------------------------------------------|
| Cargo.toml   | bagimliliklar ve qemu/generic ozellikleri                                    |
| build.rs     | linker.ld dosyasini baglayiciya verir                                        |
| linker.ld    | cekirdegi 0x100000'a yerlestirir; .text.entry en basta                       |
| src/         | kaynak kodlari (src/README.md)                                               |

## linker.ld

- ENTRY(_start). Cikti 0x100000'dan baslar.
- .text icinde ilk olarak .text.entry gelir; stage2 dogrudan 0x100000'a atladigi icin _start en basta olmalidir.
- .rodata, .data, .bss sirayla gelir. .bss icindeki 64 KB yigin ikiliye girmez (objcopy -O binary NOBITS'i atar).
- .eh_frame, .comment, .note atilir.

## Ozellikler

    qemu      nc-platform-qemu kullanir
    generic   nc-platform-generic kullanir

Tam olarak biri secilmelidir. build.sh profil adini ozellik olarak verir.

## kernel_main akisi

1. Platformu olustur ve baslat, log yaz.
2. BootInfo'yu 0x900'den oku ve dogrula.
3. Clock::calibrate ile saati kalibre et.
4. Arena'yi 0x400000'da 60 MB olarak kur; ekran, gecis ve kurulum tamponlarini ayir.
5. Zemini ciz ve ekrana ver.
6. Splash::new ile animasyon katmanlarini hazirla.
7. Animasyon dongusu (16 ms kare hedefi). prepare::Preparation::poll true donunce splash.request_finish cagrilir.
8. Splash Done olunca kurulum ekrani cizilir ve 600 ms'lik yumusak gecisle gosterilir.
9. Durur (halt_forever).

## Bellek ve sureler

Arena 64 MB sinirinda biter; 64 MB'tan az RAM'li makinelerde bu varsayim bozulur (E820 yapilinca duzelecek).
Hazirlik suresi su an sabit 4.5 saniyedir (prepare.rs); gercek hazirlik adimlari eklenince bu sure onlara baglanacak.

## Sonraki adimlar

- E820 bellek haritasi, sayfa cercevesi yoneticisi, gercek bir yigin/heap.
- Klavye (QEMU'da PS/2 veya virtio-input) ve kurulum ekraninda etkilesim.
- Disk algilama (prepare.rs icine adim olarak) ve kurulum ekraninin ikinci adimi.
