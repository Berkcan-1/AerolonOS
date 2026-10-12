# nc-splash

Animasyonlu acilis ekrani. Bagimlilik: nc-assets, nc-gfx, nc-font, nc-math, nc-mem, nc-ui.

## Sahne

Ekran ortasinda bir blok: ustte PNG logo, altinda NetizenCore yazisi, onun altinda nefes alan cizgi, en altta
(istege bagli) alt yazi ve ucu yanip sonen nokta. Alt yazi su an PREPARING SETUP'tir; SplashOptions { subtitle: None }
verilirse sadece logo, yazi ve cizgi gorunur (normal acilis icin).

## Zaman cizelgesi (splash baslangicindan itibaren)

| Evre   | Sure        | Ne olur                                                                          |
|--------|-------------|----------------------------------------------------------------------------------|
| Intro  | 0-1600 ms   | Blok bulanik ve seffaftan netlesir; opaklik 0-1000 ms'de artar; cizgi 400 ms'de buyumeye baslar |
| Idle   | sinirsiz    | Cizgi 3200 ms'de bir nefes alir (genislik yuzde 35-100); 2000 ms'de bir renk gecisi (azure -> aqua) soldan saga gecer; uc nokta 1400 ms'de bir dalgalanir |
| Outro  | 800 ms      | Blok bulaniklasip solar, cizgi kisalip kaybolur (request_finish ile baslar, Idle'a gelmeden baslamaz) |
| Done   | -           | Zemin tamamen geri gelmistir                                                     |

Sureler lib.rs basindaki sabitlerdedir (INTRO_MS, OUTRO_MS, BREATH_PERIOD_MS, SHIMMER_PERIOD_MS, DOT_PERIOD_MS, FADE_IN_MS).

## Nasil calisir

1. new: zemini (canvas'taki mevcut gorunum) sahne bolgesine kopyalar (base). Logo, baslik ve alt yaziyi siyah zeminli
   bir murekkep katmanina (sharp) cizer: renkler tersine cevrilmis yazilir (nc_gfx::invert), siyah = bos. Beyaz siluet
   logo mavi gradyanla boyanir (bkz. assets/logo/README.md); renkli logo kendi renkleriyle cizilir. Ayni katmani 3 yaricapla (4, 10, 22) bulaniklastirip 4 seviye uretir.
2. Intro ve Outro karelerinde bulaniklik konumu (0..768) iki komsu seviye arasinda karistirilir, opaklikla carpilir ve
   zeminle carpilir (multiply, compose_ink). Tum sahne bolgesi yeniden hesaplanir.
3. Idle karelerinde sadece cizgi bandi ve nokta bolgesi zeminden geri yuklenir ve yeniden cizilir. Boylece her kare
   cok az is yapar.
4. frame, degisen dikdortgenleri Update icinde dondurur; cagiran bunlari present_region ile ekrana verir.

## Genel API

- Splash::new(&canvas, &mut arena, SplashOptions) -> Option<Splash> (bellek yetmezse None)
- Splash::frame(&mut canvas, simdiki_ms) -> Update { rects, count }
- Splash::request_finish(), Splash::phase() -> Phase

Canvas, Splash::new cagrilirken zemini icermelidir (nc_ui::draw_backdrop). Katman mantigi acik zemin icin tasarlandi;
koyu zeminde multiply gorunmez, o zaman compose_ink yerine screen tabanli bir karistirma gerekir.

## Gorunumu degistirmek

- Renkler: nc-ui paleti (TEXT_PRIMARY baslik, TEXT_MUTED alt yazi, ACCENT_* logo ve cizgi, AQUA cizgideki renk gecisi).
  lib.rs basindaki SILHOUETTE_FLOOR, logonun siluet sayilma esigidir.
- Boyutlar: compute_layout icinde cap (ekran yuksekliginin 1/15'i), iz orani, bosluklar.
- Bulaniklik gucu: BLUR_RADII.
