# nc-ui

Tema ve ortak arayuz cizim yardimcilari. Bagimlilik: nc-gfx, nc-font, nc-math. Splash ve kurulum ekrani ayni gorunumu
buradan alir.

## Genel API

- Zemin renkleri: BACKDROP_TOP, BACKDROP_BOTTOM, BACKDROP_SKY, BACKDROP_AQUA
- Vurgu renkleri: ACCENT_LIGHT, ACCENT_LINE, ACCENT_DARK (azure); AQUA_LIGHT, AQUA, AQUA_DARK (aqua)
- Metin renkleri: TEXT_PRIMARY, TEXT_MUTED, TEXT_DIM
- Panel renkleri: PANEL_FILL, PANEL_BORDER, SHADOW, RAIL, RAIL_DOT
- text_style(cap, iz_orani_yuzde) -> Style (kalinlik captan turetilir)
- draw_backdrop(canvas): beyaz-gri gradyan ustune ince gokyuzu ve aqua pusu
- draw_text_color, draw_text_gradient: metin cizer (tek renk veya dikey gradyan)
- draw_dot: duz, kenari yumusak daire
- draw_orb: cam kure (azure/aqua gradyan ve ustte parlak yansima)
- draw_pill: yatay gradyanli hap (ustte hafif parlaklik)
- draw_panel: yuvarlatilmis dikdortgen panel (dolgu ve 1 piksellik kenar)
- draw_shadow: yuvarlatilmis dikdortgenin yumusak golgesi
- draw_card: golge + beyaz panel + ustte cok hafif gokyuzu parlakligi (cam kart)
- draw_fading_line: uclari silinen ince cizgi

## Gorsel dil

Sade, HarmonyOS/EMUI beyazi: cok acik zemin, beyaz kartlar, genis kose yaricapi, yumusak golge, ince koyu lacivert metin.
Frutiger Aero dokunuslari sadece kucuk vurgularda: azure/aqua cam kureler, gradyanli hap, kartin ustunde hafif parlama.
Fazlasi eklenmez. Yeni ekranlar bu renklerle ve text_style ile cizilmelidir. Metin icin TEXT_PRIMARY (baslik, aktif),
TEXT_MUTED (govde), TEXT_DIM (pasif, ikincil) kullanilir.
