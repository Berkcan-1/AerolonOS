# nc-desktop

Masaustu kabugu: duvar kagidi, masaustu simgeleri, gorev cubugu, baslat menusu ve pencere yoneticisi. Uygulamalari
nc-app sozlesmesi uzerinden calistirir; hicbir uygulamayi adiyla bilmez. Bagimlilik: nc-hal, nc-gfx, nc-font, nc-ui,
nc-app. alloc kullanir.

## Genel API

- DesktopState::new(platform, apps: Vec<Box<dyn App>>)
- install_app(app): calisma aninda uygulama ekler (simge, baslat menusu ve gorev cubugu otomatik guncellenir)
- update(mouse, key, width, height, uptime_ms) -> DesktopAction (None, Shutdown, Reboot)
- take_dirty(), take_wallpaper_dirty(): yeniden cizim gerekli mi? (okununca sifirlanir)
- wallpaper_index()
- draw_wallpaper(canvas, index): duvar kagidini bir tampona cizer (secim degisince bir kez)
- draw(canvas, wallpaper, state): sahneyi cizer (duvar kagidini kopyalar, ustune simge, pencere, gorev cubugu, menu)

## Gorunum ve davranis

- Duvar kagidi renklidir (Aurora, Dusk, Forest, Graphite); Ayarlar uygulamasindan secilir. Gorev cubugu ve baslat menusu
  koyu, pencereler acik renklidir.
- Masaustu simgesine bir kez tiklamak uygulamayi acar (cift tik yoktur). Baslat menusunde uygulamalar, RESTART ve
  SHUT DOWN vardir.
- Pencere: baslik cubugundan surukle, kucult, kapat. Tikladigin pencere one gelir. Gorev cubugu dugmesi: kucultulmusse
  geri getirir, odaktaysa kucultur, degilse one getirir.
- Escape sadece acik baslat menusunu kapatir (eski surumde Escape sistemi kapatiyordu).

## Cizim modeli

Kabuk sadece "kirli" oldugunda yeniden cizer: fare bir ogenin ustune girip cikinca, tiklayinca, pencere surukleyince,
saniye degisince (gorev cubugu sayaci) veya bir uygulama Update::REDRAW dondurunce. Sadece imlec hareket ediyorsa
cekirdek yalnizca imlecin eski ve yeni dikdortgenini gunceller (kernel/src/main.rs). Duvar kagidi ayri bir tamponda
saklanir; her cizimde kopyalanir.

## Olay yonlendirme

update, isaretcinin altindaki hedefi bulur (Target). Kabuk ogeleri (simge, gorev cubugu, menu, pencere cercevesi) kabugun
kendisinde islenir. Bir pencerenin icerigine gelen imlec ve tik, o uygulamanin update'ine Input olarak gider; acik ve
kucultulmemis her uygulama her karede update alir. Klavye tusu sadece odaktaki uygulamaya gider.
