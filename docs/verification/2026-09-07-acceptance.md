# miniterm kabul doğrulaması

Tarih: 2026-09-07
Sürüm: 96329da
Platform: Windows 11 26200
Toolchain: x86_64-pc-windows-gnu

---

## Doğrulama durumu — okuyun önce

Aşağıdaki tablolarda iki tür satır vardır:

- **OTOMATİK — DOĞRULANDI**: derleme, birim testleri veya tip denetimi tarafından
  makinede doğrulandı. Sonuç güvenilirdir.
- **MANUEL — DOĞRULANMADI**: çalışan uygulamayı, Görev Yöneticisi'ni veya GUI
  etkileşimini gerektirir; bu oturumda yapılamadı.

`MANUEL` satırlarının her birinin yanında, sıfır ek bilgiye gerek kalmadan
doğrulamayı tamamlayacak komut veya tıklama yolu verilmiştir.

---

## Derleme ve test sonuçları (otomatik)

| Kontrol | Komut | Sonuç |
|---|---|---|
| TypeScript / Svelte tip denetimi | `npm run check` | **0 hata, 0 uyarı** |
| TypeScript birim testleri | `npm run test` | **99 geçti, 0 başarısız** |
| Rust birim testleri | `cd src-tauri && cargo test` | **26 geçti, 0 başarısız** |
| Sürüm derlemesi | `npm run tauri build` | **Başarılı** (bkz. Derleme çıktısı) |

### Derleme çıktısı

- `src-tauri/target/release/miniterm.exe` — 21.96 MB
- `src-tauri/target/release/bundle/msi/miniterm_0.1.0_x64_en-US.msi` — 6.07 MB
- `src-tauri/target/release/bundle/nsis/miniterm_0.1.0_x64-setup.exe` — 3.94 MB

WiX (MSI) ve NSIS her ikisi de başarıyla üretildi; GNU toolchain ile NSIS veya
WiX'in başarısız olacağına dair önceki beklenti doğrulanmadı.

### Bilinen derleme uyarısı

```
ld.exe: .rsrc merge failure: multiple non-default manifests
```

Bu uyarı GNU toolchain + Windows manifest birleştirmesinde beklenen bir davranıştır
ve zararsızdır. Bağlayıcı çıktıyı doğru şekilde üretmektedir. Gelecekteki okuyucuları
alarma geçirmemek için buraya not düşülmüştür.

---

## Fonksiyonel doğrulama

| Ölçüt (§11) | Sonuç | Doğrulama yolu |
|---|---|---|
| Workspace oluşturma sihirbazı 4 adımda tamamlanıyor | MANUEL — DOĞRULANMADI | Uygulamayı başlat → "+" düğmesine tıkla → 4 adımı tamamla → workspace listede görünmeli |
| AI komutu her terminalde çalışıyor | MANUEL — DOĞRULANMADI | Bir workspace aç → terminallerin her birinin `settings.aiTools[n].command` değerini çalıştırdığını doğrula |
| AI aracından çıkınca panel yaşıyor | MANUEL — DOĞRULANMADI | Bir terminalde `exit` yaz → pane kapanmamalı, shell prompt geri dönmeli |
| wt.exe shell listesinde yok | MANUEL — DOĞRULANMADI | Ayarlar → Terminal → Shell listesinde `wt.exe` veya "Windows Terminal" görünmemeli |
| Gizli workspace süreçleri yaşıyor | MANUEL — DOĞRULANMADI | Bir workspace'i gizle → `tasklist /fi "imagename eq pwsh.exe"` → süreç listede kalmalı |
| Ayraç sürükleme PTY boyutunu güncelliyor | MANUEL — DOĞRULANMADI | Bir panelde `$Host.UI.RawUI.WindowSize` çalıştır → ayracı sürükle → komutu tekrar çalıştır → Width/Height değişmeli |
| Ağaç ve ayarlar yeniden başlatmada korunuyor | MANUEL — DOĞRULANMADI | Uygulamayı kapat → yeniden aç → workspace ağacı ve ayarlar aynı kalmalı |
| Terminal içeriği kalıcı değil (tasarım gereği) | MANUEL — DOĞRULANMADI | Uygulamayı kapat → yeniden aç → terminal geçmişi temizlenmiş olmalı |
| Bozuk config kurtarılıyor ve .bak yazılıyor | MANUEL — DOĞRULANMADI | `%APPDATA%\dev.miniterm.app\config.json` dosyasına geçersiz JSON yaz → uygulamayı başlat → `.bak` dosyası oluşmalı, uygulama varsayılanlarla açılmalı |
| Boşluk içeren dizinde workspace açılıyor | MANUEL — DOĞRULANMADI | Dizin olarak `D:\Development\Cursor Apps\` seç → shell o dizinde başlamalı |

---

## Performans ölçümleri

| Ölçüm | Hedef | Gerçekleşen | Doğrulama yolu |
|---|---|---|---|
| Boşta RAM (3 workspace / 6 terminal) | < 250 MB | MANUEL — DOĞRULANMADI | 3 workspace, toplam 6 terminal aç → 60 s bekle → Görev Yöneticisi → Ayrıntılar → `miniterm.exe` + tüm WebView2 alt süreçlerinin "Bellek (etkin özel çalışma kümesi)" sütununu topla |
| Boşta CPU | < %1 | MANUEL — DOĞRULANMADI | Aynı durum → 60 s boyunca Görev Yöneticisi CPU sütununu izle |
| Yoğun çıktı altında miniterm CPU | < %15 | MANUEL — DOĞRULANMADI | Bir panelde `while ($true) { "y" }` çalıştır → 30 s boyunca `miniterm.exe` + WebView2 süreçlerinin CPU toplamını izle → ölçümü bitirmek için Ctrl+C. Eşik aşılırsa `src-tauri/src/pty/batch.rs` içindeki `FLUSH_INTERVAL_MS` ve `src/term/Terminal.svelte` içindeki xterm `scrollback` değerini düşürmeyi dene |

---

## macOS ve Linux doğrulaması

macOS ve Linux doğrulaması yapılmadı — donanım yok. Kod yolları platform-koşullu
(src-tauri/src/shell.rs) ve birim testleri her iki dalı da kapsıyor, ancak uçtan uca
doğrulanmadı.

---

## Bilinen sapmalar

- NSIS ve WiX her ikisi de GNU toolchain ile başarıyla üretildi (önceki beklentinin
  aksine).
- `ld.exe: .rsrc merge failure: multiple non-default manifests` uyarısı GNU toolchain
  ile beklenen, zararsız bir linker uyarısıdır.
- `bundle.identifier` değeri (`dev.miniterm.app`) `.app` ile bitmektedir; Tauri bu
  konuda uyarı vermektedir (`ends with .app, conflicts with macOS bundle extension`).
  Windows'ta işlevsel etkisi yoktur; macOS paketlemesi yapılacaksa identifier
  değiştirilmelidir.
- `icon.icns` Windows'ta başarıyla üretildi (`npx @tauri-apps/cli icon` kendi
  cross-platform üreticisini kullanır).
