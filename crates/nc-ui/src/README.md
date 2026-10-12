# nc-ui/src

## lib.rs

- text_style: stroke_sixteenths = max(cap*16/20, 22); tracking = cap*yuzde/100.
- draw_backdrop: ust-alt gradyan (BACKDROP_TOP -> BACKDROP_BOTTOM), ustte ortada yaricapi ekran yuksekliginin yuzde 95'i
  olan gokyuzu pusu (BACKDROP_SKY, guc 52) ve sag altta aqua pusu (BACKDROP_AQUA, guc 44). Guclerin kucuk olmasi beyazligi korur.
- rounded_distance: yuvarlatilmis dikdortgen icin isaretli mesafe (1/256 piksel). draw_panel, draw_shadow, draw_pill ve
  draw_card bunu kullanir. Sadece koselerde karekok alinir.
- circle_coverage: daire kenari icin 0..255 kapsama. draw_dot ve draw_orb kullanir.
- draw_dot: yaricap piksel cinsindendir (eski surumde parilti yaricapiydi); strength 0..256 saydamlik.
- draw_orb: govde ust-alt gradyandir; ustte elips seklinde beyaz yansima asagi dogru solar.
- draw_pill: yaricap = yukseklik / 2; ust yarida beyaza dogru yuzde 40'a kadar aydinlatma.
- draw_shadow: area, offset_y kadar asagi kaydirilir; kenardan spread piksel uzakliga kadar karesel solar. strength 0..255.
- draw_card: golge (offset yaricap/2, yayilma yaricap*2.5, guc 26), beyaz panel, PANEL_BORDER kenar ve ust yuzde 36'da
  BACKDROP_SKY ile en fazla yuzde 10 saydamlikta parlama.
- draw_fading_line: orta noktadan uclara dogru karesel solan 1 piksellik cizgi.
