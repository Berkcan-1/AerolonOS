nc-gfx

Gorev: bellekte 0x00RRGGBB piksel tamponu uzerinde cizim ve ekrana aktarim.
Bagimlilik: nc-hal (FramebufferInfo).

Genel API
  rgb(r, g, b), lerp_color(a, b, t 0..=255)
  Canvas::new(&mut [u32], genislik, yukseklik)
  Canvas: width, height, pixels, fill, vertical_gradient, add_glow, fill_rect, blend_pixel, draw_rgba
  present(&Canvas, &FramebufferInfo) (unsafe): framebuffer'a yazar, renk dizilimini donusturur

Not: Canvas tamponunu cagiran saglar (kernel su an 0x400000 adresini kullanir).
