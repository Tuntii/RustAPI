# Replay workflow: time-travel debugging

Record HTTP request/response pairs in a controlled environment, inspect a captured request, replay it against another target, and diff the result before promoting a fix.

> **Security notice**
> Replay is intended for **development, staging, canary, and incident-response environments**. Do not expose the admin endpoints publicly on the open internet.

## Ne zaman kullanılır?

Replay en çok şu durumlarda işe yarar:

- staging ile local arasında davranış farkı varsa
- bir regresyonu gerçek trafik örneğiyle yeniden üretmek istiyorsanız
- yeni bir sürümü canary ortamına almadan önce kritik istekleri tekrar koşturmak istiyorsanız
- “bu istek neden dün çalışıyordu da bugün bozuldu?” sorusuna zaman makinesi tadında cevap arıyorsanız

## Ön koşullar

Uygulamada canonical replay feature'ını açın:

```toml
[dependencies]
rustapi-rs = { version = "0.1.335", features = ["extras-replay"] }
```

CLI tarafında `cargo-rustapi` yeterlidir; replay komutları varsayılan kurulumun parçasıdır:

```bash
cargo install cargo-rustapi
```

## 1) Replay kaydını etkinleştir

En küçük pratik kurulum için in-memory store ile başlayın:

```rust,ignore
use rustapi_rs::extras::replay::{InMemoryReplayStore, ReplayConfig, ReplayLayer};
use rustapi_rs::prelude::*;

#[rustapi_rs::get("/api/users")]
async fn list_users() -> Json<Vec<&'static str>> {
    Json(vec!["Alice", "Bob"])
}

#[rustapi_rs::main]
async fn main() -> std::result::Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let replay = ReplayLayer::new(
        ReplayConfig::new()
            .enabled(true)
            .admin_token("local-replay-token")
            .ttl_secs(900)
            .skip_path("/health")
            .skip_path("/ready")
            .skip_path("/live"),
    )
    .with_store(InMemoryReplayStore::new(200));

    RustApi::auto()
        .layer(replay)
        .run("127.0.0.1:8080")
        .await
}
```

Bu kurulum şunları yapar:

- replay kaydını açık hale getirir
- admin endpoint'lerini bearer token ile korur
- probe endpoint'lerini kayıttan çıkarır
- girdileri 15 dakika saklar
- bellekte en fazla 200 kayıt tutar

## 2) Hedef trafiği üret

Artık uygulamaya normal şekilde istek atın. Replay middleware uygulama kodunuzu değiştirmeden request/response çiftlerini yakalar.

Kayıt akışı şöyledir:

1. istek geçer
2. request metadata ve uygun body alanları saklanır
3. response durumu, header'ları ve yakalanabilir body içeriği saklanır
4. kayıt admin API ve CLI üzerinden erişilebilir hale gelir

## 3) Kayıtları listele ve doğru girdiyi bul

İlk bakış için CLI en rahat yol:

```bash
# Son replay girdilerini listele
cargo rustapi replay list -s http://localhost:8080 -t local-replay-token

# Sadece belirli bir endpoint'i filtrele
cargo rustapi replay list -s http://localhost:8080 -t local-replay-token --method GET --path /api/users --limit 20
```

Liste çıktısı size şu alanları gösterir:

- replay kimliği
- HTTP method
- path
- orijinal response status kodu
- toplam süre

## 4) Tek bir girdiyi incele

Şüpheli isteği bulduktan sonra tam kaydı açın:

```bash
cargo rustapi replay show <id> -s http://localhost:8080 -t local-replay-token
```

Bu komut tipik olarak şunları gösterir:

- orijinal request method ve URI
- saklanan header'lar
- yakalanan request body
- orijinal response status/body
- duration, client IP ve request ID gibi meta alanlar

## 5) Aynı isteği başka bir ortama tekrar koştur

Şimdi aynı isteği local düzeltmeniz, staging ya da canary ortamınız üzerinde çalıştırabilirsiniz:

```bash
cargo rustapi replay run <id> -s http://localhost:8080 -t local-replay-token -T http://localhost:3000
```

Pratik kullanım örnekleri:

- local düzeltmenin gerçekten incident'ı çözüp çözmediğini görmek
- staging ortamının eski üretim davranışıyla uyumunu kontrol etmek
- kritik endpoint'leri deploy öncesi smoke test gibi replay etmek

## 6) Farkları otomatik çıkar

Asıl sihir burada: replay edilen response ile orijinal response'u karşılaştırın.

```bash
cargo rustapi replay diff <id> -s http://localhost:8080 -t local-replay-token -T http://staging:8080
```

`diff` çıktısı şu alanlarda fark arar:

- status code
- response header'ları
- JSON body alanları

Bu sayede “200 döndü ama payload değişti” gibi daha sinsi regresyonları da yakalarsınız.

## Önerilen resmi workflow

Bir incident ya da regresyon sırasında önerilen akış şu sıradadır:

1. **Kayıt aç**: replay'i staging/canary ortamında kısa TTL ile etkinleştir.
2. **Örneği yakala**: problemi üreten gerçek isteği yeniden geçir.
3. **Listele**: `cargo rustapi replay list` ile doğru girdiyi bul.
4. **İncele**: `cargo rustapi replay show` ile request/response çiftini doğrula.
5. **Düzeltmeyi dene**: girdiyi local veya aday sürüme `run` ile tekrar oynat.
6. **Diff al**: `diff` ile davranışın beklenen şekilde değiştiğini doğrula.
7. **Kapat**: incident sonrası replay kaydını kapat veya TTL'i kısa tut.

Kısacası: **capture → inspect → replay → diff → promote**.

## Admin API referansı

Tüm admin endpoint'leri şu header'ı ister:

```text
Authorization: Bearer <admin_token>
```

| Method | Path | Açıklama |
|--------|------|----------|
| GET | `/__rustapi/replays` | Kayıtları listele |
| GET | `/__rustapi/replays/{id}` | Tek bir girdiyi göster |
| POST | `/__rustapi/replays/{id}/run?target=URL` | İsteği başka hedefe replay et |
| POST | `/__rustapi/replays/{id}/diff?target=URL` | Replay et ve fark üret |
| DELETE | `/__rustapi/replays/{id}` | Bir girdiyi sil |

### cURL örnekleri

```bash
curl -H "Authorization: Bearer local-replay-token" \
     "http://localhost:8080/__rustapi/replays?limit=10"

curl -H "Authorization: Bearer local-replay-token" \
     "http://localhost:8080/__rustapi/replays/<id>"

curl -X POST -H "Authorization: Bearer local-replay-token" \
     "http://localhost:8080/__rustapi/replays/<id>/run?target=http://staging:8080"

curl -X POST -H "Authorization: Bearer local-replay-token" \
     "http://localhost:8080/__rustapi/replays/<id>/diff?target=http://staging:8080"
```

## Konfigürasyon notları

`ReplayConfig` ile en sık ayarlanan seçenekler:

```rust,ignore
use rustapi_rs::extras::replay::ReplayConfig;

let config = ReplayConfig::new()
    .enabled(true)
    .admin_token("local-replay-token")
    .store_capacity(1_000)
    .ttl_secs(7_200)
    .sample_rate(0.5)
    .max_request_body(131_072)
    .max_response_body(524_288)
    .record_path("/api/orders")
    .record_path("/api/users")
    .skip_path("/health")
    .skip_path("/metrics")
    .redact_header("x-custom-secret")
    .redact_body_field("password")
    .redact_body_field("credit_card")
    .admin_route_prefix("/__admin/replays");
```

Varsayılan olarak şu header'lar `[REDACTED]` olarak saklanır:

- `authorization`
- `cookie`
- `x-api-key`
- `x-auth-token`

JSON body redaction recursive çalışır; örneğin `password` alanı iç içe nesnelerde de maskelenir.

## Kalıcı saklama için filesystem store

Geliştirici makinesi yeniden başlasa bile kayıtların kalmasını istiyorsanız filesystem store kullanın:

```rust,ignore
use rustapi_rs::extras::replay::{
    FsReplayStore, FsReplayStoreConfig, ReplayConfig, ReplayLayer,
};

let config = ReplayConfig::new()
    .enabled(true)
    .admin_token("local-replay-token");

let fs_store = FsReplayStore::new(FsReplayStoreConfig {
    directory: "./replay-data".into(),
    max_file_size: Some(10 * 1024 * 1024),
    create_if_missing: true,
});

let replay = ReplayLayer::new(config).with_store(fs_store);
```

## Özel backend yazmak isterseniz

Redis, object storage ya da kurumsal bir audit backend'i kullanmak istiyorsanız `ReplayStore` trait'ini uygulayın:

```rust,ignore
use async_trait::async_trait;
use rustapi_rs::extras::replay::{
    ReplayEntry, ReplayQuery, ReplayStore, ReplayStoreResult,
};

#[derive(Clone)]
struct MyCustomStore;

#[async_trait]
impl ReplayStore for MyCustomStore {
    async fn store(&self, entry: ReplayEntry) -> ReplayStoreResult<()> {
        let _ = entry;
        Ok(())
    }

    async fn get(&self, id: &str) -> ReplayStoreResult<Option<ReplayEntry>> {
        let _ = id;
        Ok(None)
    }

    async fn list(&self, query: &ReplayQuery) -> ReplayStoreResult<Vec<ReplayEntry>> {
        let _ = query;
        Ok(vec![])
    }

    async fn delete(&self, id: &str) -> ReplayStoreResult<bool> {
        let _ = id;
        Ok(false)
    }

    async fn count(&self) -> ReplayStoreResult<usize> {
        Ok(0)
    }

    async fn clear(&self) -> ReplayStoreResult<()> {
        Ok(())
    }

    async fn delete_before(&self, timestamp_ms: u64) -> ReplayStoreResult<usize> {
        let _ = timestamp_ms;
        Ok(0)
    }

    fn clone_store(&self) -> Box<dyn ReplayStore> {
        Box::new(self.clone())
    }
}
```

## Doğrulama kontrol listesi

Replay kurulumundan sonra şu kısa kontrolü yapın:

1. uygulamaya bir istek gönderin
2. `cargo rustapi replay list -t <token>` ile girdiyi görün
3. `cargo rustapi replay show <id> -t <token>` ile body/header kaydını doğrulayın
4. `cargo rustapi replay diff <id> -t <token> -T <target>` ile karşılaştırma alın

Bu dört adım başarılıysa workflow hazırdır.

## Güvenlik özeti

Replay sistemi birden fazla koruma ile gelir:

1. **Varsayılan kapalıdır**: `enabled(false)` ile başlar.
2. **Admin token zorunludur**: admin endpoint'leri bearer token ister.
3. **Header redaction vardır**: hassas header'lar maskelenir.
4. **Body field redaction vardır**: JSON alanları seçmeli maskelenebilir.
5. **TTL uygulanır**: eski kayıtlar otomatik temizlenir.
6. **Body boyutu sınırlandırılır**: request/response capture sınırlıdır.
7. **Bounded storage kullanılır**: in-memory store FIFO eviction ile sınırlıdır.

Öneriler:

- replay'i herkese açık production ingress arkasında açmayın
- kısa TTL kullanın
- uygulamaya özel gizli alanları redaction listesine ekleyin
- büyük kapasite ile in-memory store kullanıyorsanız bellek tüketimini izleyin
- incident sonrasında replay kaydını kapatmayı düşünün
