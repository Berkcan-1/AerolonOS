# nc-font/src

## lib.rs

- Segment, Path: bir harfin dogru parcalari (en fazla 64) ve genisligi.
- Path::line, Path::arc: yay, elips yayini 5 derece katlari uzerinden dogru parcalarina boler (nc-math::sin_milli).
- glyph(karakter): karakter icin Path uretir. Tanimsiz karakter bosluk sayilir (genislik 400).
- draw_path: harfin parcalarini olceklendirir (1/256 piksel), sinirlayici kutuyu hesaplar, kutudaki her piksel merkezinin
  en yakin parcaya uzakligini bulur (integer_sqrt) ve kapsama = yarim kalinlik + 0.5 - uzaklik olarak verir.
- text_width ve draw_text: ilerleme = harf genisligi + tracking.

Bu dosyadaki glyph() fonksiyonu, harf tasarimlarini iceren tek yerdir. Harflerin gorunumu degisecekse sadece orayi duzenle.
