# nc-setup/src

## lib.rs

draw: once nc_ui::draw_backdrop, sonra sirayla ust bilgi, adim listesi (zaman cizgisi once, sonra kure ve noktalar), kart ve kart icerigi, alt bilgi cizilir.
Kartin icerigi (padding = 3 unit) yukaridan asagi: altyazi, baslik, hap, metin satirlari; en altta durum satiri.
Sag hizali yazilar text_width ile olculerek konumlanir. VERSION, env!("CARGO_PKG_VERSION") ile derleme zamaninda alinir.
