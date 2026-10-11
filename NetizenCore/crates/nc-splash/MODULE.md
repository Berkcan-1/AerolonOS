nc-splash

Gorev: acilis ekranini cizer: koyu zemin, altin parilti, PNG logo, altin NetizenCore yazisi, ince cizgi.
Bagimlilik: nc-gfx, nc-font. Derleme zamani: png (build.rs).

Genel API
  draw(&mut Canvas)

Logo hatti (build.rs)
  NC_LOGO_DIR (varsayilan assets/logo) icindeki ilk PNG okunur, 256 pikseli asiyorsa kutu filtresiyle
  kucultulur, "NCLG" + genislik + yukseklik + RGBA bicimiyle OUT_DIR/logo.raw olarak uretilir ve
  include_bytes! ile gomulur. PNG yoksa logosuz cizilir ve derleme uyari verir.
