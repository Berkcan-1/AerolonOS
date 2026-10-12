# nc-splash/src

## lib.rs

- Layout / compute_layout: tum konumlar. cap = yukseklik/15; baslik, cizgi, alt yazi ve nokta konumlari ustten asagiya
  siralanir; blok dikeyde ortalanir (biraz yukari kaydirilir). Sahne bolgesi (stage) bloktan her yonde 56 piksel
  tasar ki bulaniklik kirpilmasin.
- Splash alanlari: base (zemin kopyasi), levels (4 murekkep katmani: keskin + 3 bulanik), zaman durumu, evre.
- is_silhouette: logonun gorunen tum piksellerinde R, G, B >= SILHOUETTE_FLOOR (200) ise dogru doner.
- draw_logo_ink: logoyu murekkep katmanina yazar. Siluet ise satira gore ACCENT_LIGHT -> ACCENT_DARK gradyaniyla boyar
  (sadece alfa kullanilir), degilse kendi renkleriyle. Renkler invert edilerek yazilir.
- breathing(t): cizgi genisligi carpani, 90..256 (yuzde 35-100).
- compose / restore: sahne bolgesini veya bir alt dikdortgenini zeminden yeniden uretir.
- draw_line: cizgiyi 3 satir kalinliginda (orta tam, ust ve alt 1/3) cizer; uclara dogru karesel solar; gecis
  konumuna gore rengi ACCENT_LINE'dan AQUA'ya yaklastirir.
- draw_dots: uc nokta, her biri 110 derece faz farkiyla sinus dalgasi.
- frame_intro, frame_idle, frame_outro: evre mantigi. Intro'dan Idle'a gecerken tum sahne bir kez tam netlikte cizilir.
- Update: en fazla iki dikdortgen.
