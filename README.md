# miniterm

Birden çok AI CLI oturumunu (Claude Code, Gemini CLI) yan yana çalıştırmak için
çok-workspace terminal uygulaması. Windows, macOS ve Linux.

## Ne yapar

- Sol tarafta ağaç görünümünde workspace'ler; klasörlerle 5 seviyeye kadar gruplama,
  sürükle-bırak ile taşıma
- Her workspace bir dizin + bir AI aracı + 1-6 terminalden oluşan bir ızgara
- Seçilen AI aracının komutu workspace'teki her terminalde otomatik çalışır
- Gizlenen workspace'lerin süreçleri yaşamaya devam eder; çıktı Rust tarafında
  256 KB'lık bir ring buffer'da tutulur
- Ayarlardan AI araçları, dizin kısayolları ve varsayılan shell yönetilir

## Kimlik bilgileri

miniterm **hiçbir sır saklamaz**. Oturum açma işini AI CLI'larının kendisi yapar.
Ayarlarda yalnızca çalıştırılacak komut metni tutulur.

## Geliştirme

```bash
npm install
npm run tauri dev
```

Testler:

```bash
npm run test                       # TypeScript birim testleri (vitest)
cd src-tauri && cargo test         # Rust birim + entegrasyon testleri
npm run check                      # svelte-check + tsc
```

Sürüm derlemesi:

```bash
npm run tauri build
```

### Windows GNU toolchain

Bu depo `x86_64-pc-windows-gnu` ile derlenir. `src-tauri/.cargo/config.toml`
makineye özgü mutlak yollar içerdiği için sürüm kontrolüne dahil edilmez;
kurulum adımları `docs/superpowers/plans/2026-09-07-miniterm.md` Task 1'dedir.

## Tasarım ve plan

- Tasarım: `docs/superpowers/specs/2026-09-07-miniterm-design.md`
- Uygulama planı: `docs/superpowers/plans/2026-09-07-miniterm.md`
- Kabul doğrulaması: `docs/verification/`
