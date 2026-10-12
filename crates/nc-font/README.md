# nc-font

Kendi vektor cizgi yazi tipi. Bagimlilik: nc-math. Yazi tipi dosyasi veya bitmap yoktur; her harf birkac dogru ve
yaydan olusur ve her boyutta alias'siz cizilir.

## Genel API

- Style { cap_height, stroke_sixteenths, tracking }
- text_width(metin, &Style) -> piksel
- draw_text(metin, x, y, &Style, &mut dyn FnMut(x, y, kapsama 0..=255))

Fonksiyon piksel yazmaz; her pikselin kapsama degerini geri cagirmaya verir. Boylece modul hicbir grafik modulune
bagimli olmaz. nc-ui bunu Canvas'a baglar.

## Desteklenen karakterler

A-Z (kucuk harfler buyuge cevrilir), 0-9, bosluk, nokta, virgul, iki nokta, tire, egik cizgi, unlem.
Turkce harfler (Ç Ğ İ Ö Ş Ü) henuz yok; eklenecekler.

## Birimler

Harfler yukseklik 1000 birimlik bir kutuda tanimlidir (y asagi dogru). Genislik harfe gore degisir (I icin 0, W icin 1060).
cap_height piksel cinsinden buyuk harf yuksekligidir. stroke_sixteenths cizgi kalinligi, piksel'in 16'da biri
cinsindendir (22 = yaklasik 1.4 piksel). tracking harfler arasi ek bosluktur (piksel).

## Yeni harf eklemek

src/lib.rs icindeki glyph() fonksiyonuna bir kol ekle: Path::new(genislik), sonra path.line(x0, y0, x1, y1) ve
path.arc(merkez_x, merkez_y, yaricap_x, yaricap_y, baslangic_derece, bitis_derece, adim). Aci 0 derece saga,
90 derece yukari; adim 5'in kati olmali ve (bitis - baslangic) adima tam bolunmelidir. Bir harf en fazla 64 parca
icerebilir (MAX_SEGMENTS).
