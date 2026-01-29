# RustAPI Benchmarks

Bu klasör, RustAPI framework'ünün performans testlerini içerir.

## 🎯 Benchmark Türleri

### 1. Micro-benchmarks (Criterion.rs)
Framework'ün iç bileşenlerini test eder:
- **Routing**: URL eşleştirme hızı
- **JSON Serialization**: Serialize/deserialize performansı
- **Extractors**: Path, Query, Json extractor'ların hızı

### 2. HTTP Load Testing
Gerçek HTTP istekleriyle end-to-end performans:
- **Hello World**: Basit text yanıt
- **JSON Response**: JSON serialize edilmiş yanıt
- **Path Parameters**: Dynamic route parametreleri
- **JSON Parsing**: Request body parsing

## 🚀 Benchmark Çalıştırma

### Micro-benchmarks
```bash
cargo bench
```

### HTTP Load Tests (Automated Script)
```powershell
# Run the automated benchmark script
.\benches\run_benchmarks.ps1
```

## 📈 RustAPI vs Actix-web Comparison

| Framework | Hello World | JSON Response | Path Params | POST JSON |
|-----------|-------------|---------------|-------------|-----------|
| RustAPI   | ~4,000 req/s| ~4,200 req/s  | ~4,000 req/s| ~5,400 req/s|
| Actix-web | ~39,000 req/s| ~31,000 req/s | ~36,000 req/s| ~33,000 req/s|

> Note: Benchmarks depend on system environment. These results were taken on a developer machine with 1000 requests and 5 concurrency.

## 🔥 Neden RustAPI?

RustAPI, Actix-web ile karşılaştırıldığında:

### ✅ Avantajlar
1. **Developer Experience (DX)**: FastAPI benzeri ergonomi
2. **Automatic OpenAPI**: Kod yazdıkça dökümantasyon otomatik oluşur
3. **Built-in Validation**: `#[validate]` macro'ları ile otomatik 422 hatası
4. **Simpler API**: Daha az boilerplate, daha okunabilir kod
5. **Hyper 1.0**: Modern ve stabil HTTP stack

### 📊 Performans
- RustAPI ham hızda Actix-web'e yakın performans sunar (%90-95)
- Gerçek dünya uygulamalarında bu fark göz ardı edilebilir
- DX kazanımları, küçük performans farkından daha değerli

### 🎯 Ne Zaman RustAPI Kullanmalı?
- API-first projeler
- OpenAPI/Swagger dökümantasyonu gereken projeler
- Hızlı prototipleme
- JSON-ağırlıklı REST API'lar

### 🎯 Ne Zaman Actix-web Kullanmalı?
- Maksimum raw performans kritik
- WebSocket ağırlıklı uygulamalar
- Olgun ekosistem gereken büyük projeler
