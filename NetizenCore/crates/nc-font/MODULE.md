nc-font

Gorev: ince cizgili, olceklenebilir yazi tipi. Piksel yazmaz, kapsama degerini geri cagirmaya verir.
Bagimlilik: yok.

Genel API
  Style { cap_height, stroke_sixteenths, tracking }
  text_width(metin, &Style) -> piksel
  draw_text(metin, x, y, &Style, &mut dyn FnMut(x, y, kapsama 0..=255))

Desteklenen harfler: N E T I Z C O R (kucuk/buyuk fark etmez) ve bosluk.
Yeni harf eklemek: src/lib.rs icindeki glyph() fonksiyonuna bir kol ekle (birim: yukseklik 1000).
