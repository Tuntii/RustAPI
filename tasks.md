# RustAPI Tasks

Bu dosya, audit sonrası uygulanacak işleri tek yerde toplar. Tamamlanan maddeler işaretlidir.

## Tamamlananlar

### Production baseline - phase 1
- [x] `RustApi` builder'a standart health probe desteği ekle
- [x] `/health`, `/ready`, `/live` endpoint'lerini built-in olarak sun
- [x] `HealthEndpointConfig` ile probe path'lerini özelleştirilebilir yap
- [x] `with_health_check(...)` ile custom dependency health check bağlanabilsin
- [x] unhealthy readiness durumunda `503 Service Unavailable` döndür
- [x] health endpoint'leri için entegrasyon testleri ekle
- [x] `README.md` ve `docs/GETTING_STARTED.md` içinde health probe kullanımını dokümante et

### Production baseline - phase 2
- [x] tek çağrıda production başlangıç ayarlarını açan preset ekle
- [x] `production_defaults("service-name")` API'sini ekle
- [x] `ProductionDefaultsConfig` ile preset davranışını konfigüre edilebilir yap
- [x] preset içinde `RequestIdLayer` etkinleştir
- [x] preset içinde `TracingLayer` etkinleştir
- [x] tracing span'lerine `service` ve `environment` alanlarını ekle
- [x] opsiyonel `version` bilgisini preset üzerinden health/tracing tarafına bağla
- [x] yeni public tipleri `rustapi-core` ve `rustapi-rs` facade üzerinden export et
- [x] production preset için entegrasyon testleri ekle
- [x] production preset kullanımını README ve Getting Started içinde dokümante et

### Doğrulama
- [x] `cargo test -p rustapi-core --test health_endpoints --test status_page`
- [x] `cargo test -p rustapi-core --test production_defaults --test health_endpoints --test status_page`

## Kritik sıradaki işler

### Kimlik doğrulama ve session hikâyesi
- [x] built-in session store tasarla
- [x] memory-backed session store ekle
- [x] Redis-backed session store ekle
- [x] cookie + session extractor/middleware akışını resmileştir
- [x] login/logout/session refresh örnekleri ekle
- [x] OIDC / OAuth2 üretim rehberi yaz

### Production güveni ve operasyonel netlik
- [x] resmi production checklist dokümanı yaz
- [x] recommended production baseline rehberi yaz
- [x] graceful shutdown + draining davranışını tek rehberde topla
- [x] deployment health/readiness/liveness önerilerini cookbook'a ekle
- [x] observability için golden config örneği yayınla

### Performans güvenilirliği
- [x] benchmark iddialarını tek authoritative kaynağa taşı
- [x] README / docs / release notları arasındaki performans sayılarını senkronize et
- [x] p50/p95/p99 latency benchmark çıktıları ekle
- [x] feature-cost benchmark matrisi çıkar
- [x] execution path (ultra fast / fast / full) benchmark karşılaştırması ekle

## Yüksek etkili DX işleri

### Resmi örnekler
- [x] `crates/rustapi-rs/examples/full_crud_api.rs` ekle
- [x] `crates/rustapi-rs/examples/auth_api.rs` ekle
- [x] `crates/rustapi-rs/examples/streaming_api.rs` ekle
- [x] `crates/rustapi-rs/examples/jobs_api.rs` ekle
- [x] examples için index/README ekle

### Dokümantasyon ve discoverability
- [x] macro attribute reference yaz (`#[tag]`, `#[summary]`, `#[param]`, `#[errors]`)
- [x] custom extractor cookbook rehberi yaz
- [x] error handling cookbook rehberi yaz
- [x] observability cookbook rehberi yaz
- [x] middleware debugging rehberi yaz
- [x] Axum -> RustAPI migration guide yaz
- [x] Actix -> RustAPI migration guide yaz

### Data / DB guidance
- [x] SQLx / Diesel / SeaORM tercih rehberi yaz
- [x] migration strategy rehberi yaz
- [x] connection pooling önerilerini dokümante et

## Nice-to-have / ekosistem büyütme

### Runtime ve protocol geliştirmeleri
- [ ] streaming multipart upload desteği ekle
- [ ] büyük dosya yükleme için memory-safe akış tasarla
- [ ] GraphQL integration araştır ve adapter tasarla
- [ ] gelişmiş rate limiting stratejileri ekle (sliding window / token bucket)

### CLI ve tooling
- [ ] `cargo rustapi doctor` komutunu production checklist ile hizala
- [ ] deploy/config doctor çıktısını geliştir
- [ ] feature preset scaffold'ları ekle (`prod-api`, `ai-api`, `realtime-api`)
- [ ] replay / benchmark / observability akışlarını CLI'dan erişilebilir yap

### Farklılaştırıcı ürünleşme
- [ ] adaptive execution model için görünür profiling/debug UX tasarla
- [ ] TOON/AI-first API deneyimini preset + örnek + tooling ile ürünleştir
- [ ] replay/time-travel debugging için resmi workflow rehberi yaz

## Notlar
- Tamamlanan maddeler bu branch'te uygulanmış ve test edilmiş işleri temsil eder.
- Bir sonraki en yüksek kaldıraçlı uygulama dilimi: **session store + auth/session story** veya **production checklist + observability guide**.
