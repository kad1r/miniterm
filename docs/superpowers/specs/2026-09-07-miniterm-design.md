# miniterm — Tasarım Dokümanı

**Tarih:** 2026-09-07
**Durum:** Onaylandı, uygulamaya hazır

---

## 1. Amaç

miniterm, aynı anda birden fazla AI komut satırı aracını (Claude Code, Gemini CLI vb.)
çalıştırmak için tasarlanmış çok-workspace'li bir terminal uygulamasıdır.

Bugün kullanıcı bunu elle yapıyor: birkaç ayrı terminal penceresi açıp her birinde
ilgili dizine geçip AI aracını başlatıyor. miniterm bu kurulumu adlandırılabilir,
yeniden açılabilir workspace'lere dönüştürür.

**Hedef olmayan:** genel amaçlı bir terminal emülatörü olmak. miniterm, "bir dizinde
n paralel AI oturumu" iş akışına hizmet eder.

---

## 2. Karar özeti

| Konu | Karar |
|---|---|
| Stack | Tauri 2 (Rust) + Svelte 5 + xterm.js |
| Ağaç düğümü tipleri | Folder (gruplama) ve Workspace (terminalli) |
| Kimlik doğrulama | Uygulama sır saklamaz; AI CLI'ları kendi auth'unu yönetir |
| Gizli workspace'ler | Process'ler yaşar; DOM yok edilir; çıktı Rust ring buffer'ında |
| Kalıcılık | Ayarlar + workspace ağacı diske yazılır; terminal içeriği yazılmaz |
| Grid | rows × cols, sürüklenebilir ayırıcılar, ortak sütun çizgileri |
| AI komutu | Workspace'teki her terminalde çalışır |
| Shell | Global varsayılan + workspace başına override; tespit edilen listeden seçim |
| Dizinler | Elle verilen alias'lar + otomatik son kullanılanlar |
| Durum sahipliği | PTY/buffer/config Rust'ta, ağaç ve UI durumu TypeScript'te |

---

## 3. Teknoloji

Sürümler 2026-09-07'de bu makinede doğrulandı.

**Rust tarafı**
- `tauri` 2.11.5
- `portable-pty` 0.9.0 — Windows'ta ConPTY, Unix'te `openpty`
- `serde` / `serde_json` — config serileştirme
- Toolchain: `stable-x86_64-pc-windows-gnu`, rustc 1.98.0

**Frontend**
- `svelte` 5.57.0 — derleyici tabanlı, runtime'ı ihmal edilebilir
- `@xterm/xterm` 6.0.0
- `@xterm/addon-fit` 0.11.0
- `@xterm/addon-webgl` 0.19.0
- `@tauri-apps/api` 2.11.1, `@tauri-apps/cli` 2.11.4
- Vite (Tauri şablonunun getirdiği sürüm)
- Node 25.2.1

**Neden Svelte:** kapsam iç içe sürükle-bırak'lı ağaç, çok adımlı sihirbaz ve üç
sekmeli ayarlar ekranı içeriyor. Vanilla TS'te bunlar elle DOM senkronizasyonu
demek. Svelte sanal DOM taşımaz ve derleme sonrası runtime'ı ~15–20 KB'dir; "minimum
kaynak" şartına dokunmaz. xterm.js framework tarafından yönetilmez — kendi DOM'una
kendisi tutunur, Svelte sadece kapsayıcı elemanı sağlar.

**Neden Electron değil:** ~180–300 MB idle RAM. Tauri sistem WebView'ini kullanır.

### Bu makineye özgü derleme kısıtları

Bunlar depoda görünmez, ama olmadan `cargo build` başarısız olur:

1. **GNU toolchain, MSVC değil.** Sonuç olarak `src-tauri/Cargo.toml` içinde
   `crate-type = ["rlib"]` kullanılır — Tauri şablonunun
   `["staticlib", "cdylib", "rlib"]` değeri
   `ld.exe: error: too many exported symbols` ile patlar.
2. **`.cargo/config.toml` git-ignore'lu**, `target-dir` olarak boşluk içermeyen bir
   yol verir.
3. **Windows junction** `C:\miniterm-dev -> D:\Development\Cursor Apps\miniterm`.
   GNU toolchain'in `windres` aracı boşluklu yollarda başarısız oluyor.

Bu üçü kasıtlıdır; "düzeltilmemelidir".

---

## 4. Mimari

### 4.1 Durum sahipliği

```
┌─────────────────────── WebView (Svelte) ───────────────────────┐
│  store    : workspace ağacı + ayarlar (bellekte, tek kaynak)   │
│  tree     : sidebar, sürükle-bırak, rename                      │
│  grid     : layout hesabı, ayırıcı sürükleme                    │
│  terminal : xterm.js sarmalayıcısı (instance başına)            │
│  wizard   : workspace oluşturma akışı                           │
│  settings : AI Tools / Dizinler / Terminal sekmeleri            │
│  ipc      : Rust komutlarının tek tip TS karşılıkları           │
└────────────────────────────┬───────────────────────────────────┘
                             │  Tauri komutları  +  Channel (ham bayt)
┌────────────────────────────┴───────────────────────────────────┐
│  commands : ince sarmalayıcılar, iş mantığı yok                 │
│  config   : atomik oku/yaz, şema, bozuk dosya kurtarma          │
│  shell    : platform başına shell tespiti                       │
│  pty      : oturum yaşam döngüsü, okuma thread'leri, ring buffer│
└─────────────────────── Rust (src-tauri) ───────────────────────┘
```

**Neden bölünmüş sahiplik:** ağaç mutasyonları (sürükle-bırak, rename) IPC'ye hiç
dokunmaz, dolayısıyla anında tepki verir. Ring buffer'lar Rust'ta kalır, böylece
gizli workspace'lerin DOM'unu yok etmenin anlamı korunur — buffer JS heap'ine geri
taşınsaydı kaçınılan maliyet geri gelirdi. Kalıcılık tek bir debounce'lu yazmaya iner.

### 4.2 Modül sözleşmeleri

| Modül | Sorumluluk | Bağımlılığı |
|---|---|---|
| `config` | `load() -> Config`, `save(Config)`. Atomik yazma. | serde, dosya sistemi |
| `shell` | `detect() -> Vec<ShellInfo>` | işletim sistemi |
| `pty` | `spawn`, `write`, `resize`, `kill`, `buffer`, `attach`, `detach` | portable-pty |
| `commands` | Tauri yüzeyi | yukarıdaki üçü |
| `store` | Ağaç ve ayar mutasyonları (saf fonksiyonlar) | `ipc` |
| `tree` | Sidebar görünümü ve etkileşimi | `store` |
| `grid` | Hücre yerleşimi, ayırıcılar | `store`, `terminal` |
| `terminal` | Tek bir xterm.js instance'ı | `ipc` |

Sınırların testi: `pty` workspace'in ne olduğunu bilmez, yalnız oturum kimlikleri
bilir. `store` xterm.js'i bilmez. `tree` ağaç mutasyonlarını kendi yapmaz, `store`a
sorar. `ipc` dışında hiçbir modül `invoke` çağırmaz.

---

## 5. Veri modeli

### 5.1 Config dosyası

Konum, Tauri'nin uygulama config dizini:

- Windows `%APPDATA%\miniterm\config.json`
- macOS `~/Library/Application Support/miniterm/config.json`
- Linux `~/.config/miniterm/config.json`

```jsonc
Config {
  version: 1,                 // şema göçü için
  aiTools: AiTool[],
  directories: DirAlias[],    // elle verilmiş alias'lar
  recentDirs: string[],       // otomatik, en fazla 20, en yeni başta
  defaultShellId: string,
  tree: Node[]
}

AiTool   { id: string, name: string, command: string }
DirAlias { id: string, alias: string, path: string }

Node = Folder | Workspace

Folder {
  id: string,
  kind: "folder",
  name: string,
  expanded: boolean,
  children: Node[]
}

Workspace {
  id: string,
  kind: "workspace",
  name: string,
  path: string,
  aiToolId: string | null,    // null = "Sadece shell"
  shellId: string | null,     // null = defaultShellId kullan
  rows: number,               // 1..6
  cols: number,               // 1..6, rows * cols <= 6
  rowSizes: number[],         // uzunluk = rows, toplam = 1
  colSizes: number[]          // uzunluk = cols, toplam = 1
}
```

`id` değerleri UUID v4.

**Terminal sayısı ayrı bir alan değildir** — `rows * cols` çarpımıdır.

**Klasör derinliği en fazla 5 seviye.** Sonsuz derinlik sidebar girintisini ve
sürükle-bırak döngü kontrolünü gereksiz karmaşıklaştırır.

### 5.2 Kalıcı olmayan durum

Yalnız bellekte, uygulamayla birlikte ölür: açık PTY oturumları,
`workspaceId -> SessionId[]` eşlemesi, ring buffer içerikleri, seçili workspace,
pencere boyutu, canlı xterm instance'ları.

### 5.3 Yazma stratejisi

Ağaçta ya da ayarlarda her değişiklikten sonra **300 ms debounce**, ardından config'in
tamamı **atomik** yazılır: aynı dizine geçici dosya, `fsync`, sonra hedefin üzerine
`rename`. Ard arda üç rename tek yazma olur; yazma sırasındaki çökme config'i yarım
bırakmaz.

### 5.4 Bozuk config

JSON parse edilemezse ya da `version` bilinmiyorsa: dosya `config.json.bak` olarak
kenara alınır, uygulama boş ağaçla açılır ve durumu bir bildirimle söyler. Sessizce
silmez, sessizce çökmez.

### 5.5 İlk açılış

Config dosyası yoksa şu varsayılanlarla oluşturulur:

- `aiTools`: iki hazır kayıt — `{ name: "Claude Code", command: "claude" }` ve
  `{ name: "Gemini", command: "gemini" }`. Kullanıcı bunları Ayarlar'dan düzenler ya
  da siler; uygulama araçların kurulu olup olmadığını denetlemez.
- `defaultShellId`: `shell::detect()` sonucunun ilk elemanı (Windows'ta pratikte
  `pwsh` varsa o, yoksa `powershell`).
- `directories`, `recentDirs`, `tree`: boş.

Ağaç boşken ana alanda kısa bir boş durum metni ve "Workspace oluştur" butonu görünür.

---

## 6. Terminal katmanı

### 6.1 Oturum başlatma

Her hücre için bir PTY oturumu:

1. Shell çözümlenir: `workspace.shellId ?? config.defaultShellId`.
2. **Etkileşimli** shell spawn edilir, çalışma dizini `workspace.path`.
3. Workspace'in bir AI tool'u varsa, aracın komutu shell'in stdin'ine yazılır ve
   satır sonu gönderilir.

**Kritik:** AI komutu shell'e argüman olarak (`pwsh -Command "claude"`) verilmez.
Verilseydi, AI aracından çıkıldığı anda shell de kapanır ve hücre ölürdü. stdin'e
yazıldığında, `claude`'dan çıkınca altında çalışan bir shell bulunur.

### 6.2 Shell tespiti

`shell::detect()` kurulu olanları döner, aşağıdaki öncelik sırasıyla:

- **Windows:** `pwsh.exe` (PowerShell 7) → `powershell.exe` (Windows PowerShell 5) →
  `cmd.exe` → Git Bash (`bash.exe`, Git kurulumundan) → kayıtlı WSL dağıtımları.
- **macOS / Linux:** `$SHELL`, ardından `/etc/shells` içindekiler.

`wt.exe` (Windows Terminal) **listeye girmez.** O bir shell değil, shell barındıran
bir pencere uygulamasıdır; alt process olarak spawn edilirse kendi penceresini açar
ve miniterm'in grid'inde görünmez. Kullanıcının "Windows Terminal deneyimi" pratikte
`pwsh.exe`'ye karşılık gelir.

v1'de listeye elle özel bir shell yolu eklenemez.

### 6.3 Ring buffer

Oturum başına **256 KB**'lık bayt ring buffer'ı. Dolduğunda baştan **tam satırlar**
atılır, böylece rehidrasyon yarım bir ANSI dizisiyle başlamaz.

Rolü: görünen bir terminalin scrollback'i xterm.js'tedir (5000 satır). Workspace
gizlenip DOM'u yok edildiğinde o scrollback kaybolur; geri dönüldüğünde ekran ring
buffer'dan tazelenir.

**Bilinçli takas ve bilinen kısıt:** gizlenmiş bir workspace'e dönüldüğünde son
~256 KB'lık çıktı görülür, öncesi kaybolmuştur. 30 terminal için toplam ≈ 8 MB. Tam
scrollback saklamak bunun onlarca katı olurdu.

### 6.4 Görünürlük ve LRU

- Aktif workspace: xterm canlı, oturumlar `attach` durumunda, baytlar IPC'den akar.
- Gizli workspace: process'ler çalışmaya devam eder, oturumlar `detach` edilir,
  baytlar yalnız ring buffer'a yazılır, IPC'ye hiç çıkmaz.
- **En son kullanılan 3 workspace'in xterm instance'ı canlı tutulur** (LRU). İki
  workspace arasında gidip gelmek hiçbir şey kaybettirmez; DOM yok etme yalnız 3'ün
  ötesine düşenlerde olur.

---

## 7. IPC

### 7.1 Komut yüzeyi

| Komut | İmza |
|---|---|
| `load_config` | `() -> Config` |
| `save_config` | `(Config) -> ()` |
| `detect_shells` | `() -> ShellInfo[]` |
| `pick_directory` | `() -> string \| null` |
| `spawn_session` | `(cwd, shellId, initialCommand?) -> SessionId` |
| `write_session` | `(SessionId, bytes) -> ()` |
| `resize_session` | `(SessionId, cols, rows) -> ()` |
| `kill_session` | `(SessionId) -> ()` |
| `attach_session` | `(SessionId, Channel) -> ()` |
| `detach_session` | `(SessionId) -> ()` |
| `get_buffer` | `(SessionId) -> bytes` |

### 7.2 Çıktı akışı

PTY çıktısı Tauri 2 `Channel` üzerinden **ham bayt** olarak gider — JSON
serileştirme yoktur. Tauri'nin varsayılan olay yolu her mesajı JSON'a çevirir; yoğun
terminal çıktısında (`npm install`, `cargo build`) bu tek başına CPU'yu yer.

Okuma thread'i baytları biriktirir ve **8 ms dolduğunda ya da 32 KB'a ulaştığında**
gönderir — hangisi önce olursa. Bir çıktı seli saniyede binlerce mesaj değil, ~125
toplu mesaj üretir.

---

## 8. Arayüz

### 8.1 Sidebar

Genişlik ~240 px, daraltılabilir. Başlıkta "Workspaces" ve `+` butonu.

Satırlar derinliğe göre girintili. Klasör: açılır ok + isim. Workspace: durum noktası
+ isim. Nokta **yeşil** = terminalleri çalışıyor, **gri** = henüz açılmamış,
**kırmızı** = spawn hatası.

- Sol tık workspace'i aktif eder; klasöre tık daraltır/genişletir.
- Sağ tık menüsü: Yeni workspace, Yeni klasör, Yeniden adlandır (F2), Sil.
- Silme onay ister ve kaç terminalin kapanacağını söyler.

**Sürükle-bırak:** bir satırın *üstüne* bırakmak o klasörün içine taşır; satırlar
*arasına* bırakmak sıralar. Bırakma noktası bir çizgiyle gösterilir. Reddedilen
hedefler (imleç "yasak" olur): bir düğümün kendi alt ağacına bırakılması, ve 5 seviye
sınırını aşacak bırakmalar.

### 8.2 Workspace sihirbazı

Dört adım, geri gidilebilir.

1. **Dizin.** Üstte kayıtlı alias'lar, altında son kullanılanlar, en altta "Gözat…"
   (yerel dosya seçici). Seçimden sonra isteğe bağlı "alias olarak kaydet" alanı.
2. **AI tool.** Ayarlardaki tool listesi + **"Sadece shell"** seçeneği (workspace'i
   düz terminal grubu olarak açmak için: build, log, git).
3. **Terminal sayısı ve düzen.** 1–6 arası butonlar; seçilen sayının çarpanları
   küçük grid önizlemeleriyle sunulur:

   | Terminal | Düzenler |
   |---|---|
   | 1 | 1x1 |
   | 2 | 1x2, 2x1 |
   | 3 | 1x3, 3x1 |
   | 4 | 1x4, 2x2, 4x1 |
   | 5 | 1x5, 5x1 |
   | 6 | 1x6, 2x3, 3x2, 6x1 |

4. **İsim.** Varsayılan olarak dizinin son klasör adı, düzenlenebilir.

Bitince workspace ağaca eklenir, aktif olur, terminaller spawn edilir ve varsa AI
komutu her birinde çalışır.

### 8.3 Grid

CSS grid; `rowSizes` / `colSizes` kesirleri `fr` birimine çevrilir.

**Sütun ayırıcıları tüm satırlarda ortaktır** — gerçek bir tablo gibi. Bir sütun
çizgisi çekildiğinde her satırda birlikte hareket eder. Alternatifi (satır başına
bağımsız sütunlar) veri modelini iki katına çıkarır ve sürükleme davranışını
öngörülemez kılar.

Ayırıcıların tutma alanı 4 px. Sürükleme sırasında kesirler her karede güncellenir ve
xterm `fit()` çalışır (görsel akıcılık için), ama PTY'ye `resize` çağrısı **sürükleme
bitince** gider — her karede resize göndermek AI CLI'larını sürekli yeniden
çizdirirdi. Hücreler 120 px'in altına inemez.

Pencere boyutu değişince 100 ms debounce ile görünen tüm hücreler yeniden fit edilir.

Aktif hücrenin ince bir kenarlığı olur; tıklayınca odak ona geçer.

### 8.4 Ayarlar

Modal değil, ayrı bir tam ekran görünüm. Üç sekme.

- **AI Tools** — liste; ekle / düzenle / sil. Alanlar: görünen ad, komut. Komut
  alanının altında canlı önizleme, ne olacağını gösterir:
  `pwsh.exe` (çalışma dizini: workspace yolu), ilk komut: `claude --model sonnet-5`.
- **Dizinler** — alias listesi (ekle / düzenle / sil) ve son kullanılanlar listesi
  ("alias yap", "temizle").
- **Terminal** — tespit edilen shell'lerden varsayılan seçimi, "yeniden tara" butonu.

Silinen bir AI tool'unu ya da alias'ı kullanan workspace varsa, silme uyarır ve kaç
workspace'i etkilediğini söyler.

### 8.5 Görsel dil

Tek koyu tema. Sistem monospace font yığını. Dar tipografi, düşük kontrastlı
ayırıcılar. Renk yalnız durum için kullanılır (yeşil / gri / kırmızı nokta, aktif
hücre kenarlığı).

---

## 9. Hata yönetimi

| Durum | Davranış |
|---|---|
| Shell bulunamadı / spawn başarısız | Hücre hata metnini gösterir; sidebar noktası kırmızı |
| Process çıktı | Hücrede `[process exited: <kod>] — yeniden başlatmak için Enter`; Enter oturumu yeniden spawn eder |
| Workspace dizini artık yok | Aktive ederken uyarır; "yeni dizin seç" ya da "ev dizininde aç" |
| Config bozuk | `.bak`'a alınır, boş ağaçla açılır, bildirim (bkz. 5.4) |
| Channel gönderimi başarısız | Mesaj düşürülür; ring buffer'a yazma sürer |
| Uygulama kapanışı | Tüm oturumlara nazik kapanma; 2 sn sonra yaşayanlar `kill` |

---

## 10. Test

**Rust birim**
- Ring buffer satır sınırında atıyor ve 256 KB'ı aşmıyor.
- Config atomik yazılıyor; bozuk dosyadan kurtarma `.bak` üretiyor ve boş config
  dönüyor.
- Shell tespiti sahte bir dosya sistemiyle doğru sırayı veriyor; `wt.exe` listeye
  girmiyor.

**Rust entegrasyon** (üç platformda da koşar)
- Gerçek bir PTY'de `echo hello` çalıştır, buffer'da bul.
- `resize_session` sonrası shell'den ölçülen boyut (`stty size` /
  `$Host.UI.RawUI.WindowSize`) beklenen değeri veriyor.

**TypeScript birim**
- Ağaç mutasyonları: taşıma, döngü reddi, derinlik sınırı, sıralama.
- Grid kesir matematiği ve minimum hücre kısıtı.
- Debounce'lu kaydetme ard arda değişiklikleri tek yazmada birleştiriyor.
- Terminal sayısına göre sunulan düzen listesi (çarpan tablosu).

**Manuel**
- Sürükle-bırak akıcılığı ve görsel doğrulama. Otomatize edilmez.

---

## 11. Kabul kriterleri

**İşlevsel**
- Sidebar'da klasör ve workspace oluşturulabiliyor, yeniden adlandırılabiliyor,
  sürükle-bırak ile taşınabiliyor; 5 seviyeye kadar iç içe.
- Sihirbaz dizin / AI tool / sayı+düzen / isim adımlarını yürütüyor; workspace açılıp
  terminaller AI komutuyla başlıyor.
- Grid ayırıcıları sürüklenebiliyor; oranlar kaydediliyor ve yeniden açılışta geri
  geliyor.
- Ayarlar üç sekmede ekleme/düzenleme/silme yapabiliyor.
- Uygulama kapanıp açıldığında ağaç ve ayarlar aynen geliyor; terminaller boş.
- Windows, macOS ve Linux'ta derleniyor ve çalışıyor.

**Performans** (1 workspace × 4 boş terminal, boşta)
- Idle RAM < 250 MB (uygulama + shell process'leri toplamı).
- Idle CPU < %1.
- Bir terminalde `yes` çalışırken miniterm'in kendi CPU'su < %15.

---

## 12. v1 kapsamı dışında

Referans ekran görüntüsünde bulunan ama alınmayanlar: gömülü web önizleme paneli
(`localhost:3000` sekmesi), Agent/Code/Chat modları, kredi göstergesi, bildirim çanı.

Kasıtlı olarak ertelenenler: tema seçimi (tek koyu tema var), font ayarı ve Ctrl+/−
zoom, terminal içi arama, klavye kısayolları (workspace geçişi, terminal odağı), tek
terminali ayrı kapatma/yeniden başlatma UI'ı, workspace kopyalama ve dışa/içe aktarma,
otomatik güncelleme, özel shell yolu girme, terminal çıktısının diske kaydedilmesi,
satır başına bağımsız sütun genişlikleri.

Mobil hedefler (iOS/Android) kapsam dışıdır ve bu makinenin GNU toolchain kurulumuyla
zaten derlenmez.
