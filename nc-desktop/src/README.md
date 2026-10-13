# nc-desktop/src

## lib.rs

DesktopState (uygulama slotlari, z sirasi, ayarlar, baslat menusu, hover, surukleme), Target (imlecin altindaki hedef),
update, click, dispatch, draw_wallpaper ve draw. Slot = uygulama + pencere durumu (acik, kucuk, yerlesti, x, y).
open_app ilk acilista pencereyi ortalar ve uygulama indeksine gore (mod 4) kaydirir. apply_settings, uygulamanin
degistirdigi Settings'i alir ve gerekirse duvar kagidini ve sahneyi kirli isaretler.

## layout.rs

Tum yerlesim fonksiyonlari (unit = ekran yuksekligi / 36, en az 10). Gorev cubugu, baslat dugmesi ve menusu, simge
izgarasi (ekran yuksekligine gore sutunlara sarar), pencere cercevesi dugmeleri, icerik alani. update ve draw ayni
fonksiyonlari kullanir; boylece tiklama alani ile cizim hep ayni yerdedir.

## shell.rs

Masaustu simgeleri, gorev cubugu (baslat, acik uygulamalar, calisma suresi) ve baslat menusu cizimi.

## window.rs

Pencere cercevesi: golge, govde, baslik cubugu, uygulama ikonu ve adi, kucult ve kapat dugmeleri.

## theme.rs

Sadece kabuga ozgu renkler. Duvar kagitlari ve vurgu renkleri nc-app/theme.rs icindedir.
