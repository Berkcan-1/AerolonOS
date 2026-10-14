# nc-app-browser

NetizenCore tarayici uygulamasi. HTML ayristirici, CSS motoru, yerlesim motoru ve resim cozucu burada yasar. Ag katmani nc-net crate'indedir.

## Dosyalar

- lib.rs: BrowserApp, gezinme, alt kaynak kuyrugu, hata sayfasi, cizim
- html.rs: tokenizer, DOM agaci, metin temizleme, YouTube ve meta yedek kartlari
- css.rs: stylesheet ayristirma, secici eslestirme, var() cozumleme, media sorgusu degerlendirme, SafeClamp
- layout.rs: akis yerlesimi, satir ici yerlesim, tablo satiri, hesaplanan stiller
- image.rs: PNG, JPEG, BMP, SVG cozucu
- url.rs: URL normalize ve cozumleme
- engine.rs: eski motor, derlenmez, referans icin duruyor

## Kararlilik kurallari

- Tum i32 clamp islemleri SafeClamp::sclamp ile yapilir. Alt sinir ust sinirdan buyukse ust sinir doner, panik olmaz.
- Metin ve metin benzeri ozellikler html.rs icinde sanitize_text ile ASCII'ye indirgenir. Turkce harfler en yakin Latin harfine donusur, fontta olmayan semboller atilir.
- display flex ve grid, sayfa CSS'inde blok akisina cevrilir. inline-flex ve inline-grid inline-block olur. Tablo satiri (tr) yatay kalir.
- max-width, min-height ve height blok kutularini etkilemez. Kutu yuksekligi icerikten gelir. Resim ve svg boyutu height ozelligini kullanir.
- background kisayolunda gradient ve url() degeri yok sayilir.
- Media sorgulari 1100 piksellik masaustu genisligine gore degerlendirilir. print, dark, reduce, coarse, portrait ve not iceren sorgular uygulanmaz.
- var() ifadeleri deger icinde herhangi bir yerde cozulur, yedek deger desteklenir.
- Her ana belge yuklenisinde resim listesi temizlenir. Eski sayfanin svg ve resimleri yeni sayfaya karismaz.
- Resim eslestirmesi tam esitlikle yapilir, sonek eslestirmesi yoktur.
- Alt kaynaklar (css, resim) bittikce her biri icin degil, kuyruk bosalinca bir kez yerlesim yeniden kurulur.
- Ana belge basarisiz olursa veya bos donerse hata sayfasi gosterilir, durum Idle'a doner ve dongu olusmaz.
- Gorunum alani altina durum cubugu zemini cizilir, tasan kutular durum cubugunun ustune cikmaz.
- Gorunum alani yuksekligi cizim, tiklama ve kaydirma icin ayni (area.height - unit * 7) degerini kullanir.
- HTML ayristirma siniri 1 MB, yanit siniri 1 MB.
