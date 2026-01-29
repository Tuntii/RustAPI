//! WebSocket message throughput benchmarks
//!
//! Benchmarks the performance of WebSocket message handling in RustAPI.

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use std::collections::HashMap;

/// Simulate WebSocket message parsing (text)
fn parse_text_message(data: &str) -> String {
    data.to_string()
}

/// Simulate WebSocket message parsing (binary)
fn parse_binary_message(data: &[u8]) -> Vec<u8> {
    data.to_vec()
}

/// Simulate JSON message parsing
fn parse_json_message(data: &str) -> serde_json::Value {
    serde_json::from_str(data).unwrap_or(serde_json::Value::Null)
}

/// Simulate message frame encoding
fn encode_frame(opcode: u8, payload: &[u8], mask: bool) -> Vec<u8> {
    let mut frame = Vec::with_capacity(14 + payload.len());

    // FIN bit + opcode
    frame.push(0x80 | opcode);

    // Payload length
    let len = payload.len();
    if len < 126 {
        frame.push((if mask { 0x80 } else { 0 }) | len as u8);
    } else if len < 65536 {
        frame.push((if mask { 0x80 } else { 0 }) | 126);
        frame.push((len >> 8) as u8);
        frame.push(len as u8);
    } else {
        frame.push((if mask { 0x80 } else { 0 }) | 127);
        for i in (0..8).rev() {
            frame.push((len >> (i * 8)) as u8);
        }
    }

    // Masking key (if masked)
    if mask {
        let mask_key: [u8; 4] = [0x12, 0x34, 0x56, 0x78];
        frame.extend_from_slice(&mask_key);

        // Masked payload
        for (i, byte) in payload.iter().enumerate() {
            frame.push(byte ^ mask_key[i % 4]);
        }
    } else {
        frame.extend_from_slice(payload);
    }

    frame
}

/// Benchmark text message parsing
fn bench_text_message(c: &mut Criterion) {
    let mut group = c.benchmark_group("websocket_text");

    let messages = [
        ("tiny", "Hi"),
        ("small", "Hello, WebSocket!"),
        ("medium", &"x".repeat(1024)),
        ("large", &"x".repeat(64 * 1024)),
    ];

    for (name, msg) in messages.iter() {
        group.throughput(Throughput::Bytes(msg.len() as u64));
        group.bench_with_input(BenchmarkId::new("parse", name), msg, |b, msg| {
            b.iter(|| parse_text_message(black_box(msg)))
        });
    }

    group.finish();
}

/// Benchmark binary message parsing
fn bench_binary_message(c: &mut Criterion) {
    let mut group = c.benchmark_group("websocket_binary");

    let messages: Vec<(&str, Vec<u8>)> = vec![
        ("tiny", vec![1, 2, 3, 4]),
        ("small", vec![0u8; 64]),
        ("medium", vec![0u8; 4096]),
        ("large", vec![0u8; 64 * 1024]),
    ];

    for (name, msg) in messages.iter() {
        group.throughput(Throughput::Bytes(msg.len() as u64));
        group.bench_with_input(BenchmarkId::new("parse", name), msg, |b, msg| {
            b.iter(|| parse_binary_message(black_box(msg)))
        });
    }

    group.finish();
}

/// Benchmark JSON message parsing (common WebSocket pattern)
fn bench_json_message(c: &mut Criterion) {
    let mut group = c.benchmark_group("websocket_json");

    // Simple JSON message
    let simple_json = r#"{"type":"ping"}"#;

    // Typical chat message
    let chat_json =
        r#"{"type":"message","user":"alice","content":"Hello everyone!","timestamp":1704067200}"#;

    // Complex nested JSON
    let complex_json = r#"{"type":"state","data":{"users":[{"id":1,"name":"Alice"},{"id":2,"name":"Bob"}],"room":"general","active":true}}"#;

    group.bench_function("simple", |b| {
        b.iter(|| parse_json_message(black_box(simple_json)))
    });

    group.bench_function("chat", |b| {
        b.iter(|| parse_json_message(black_box(chat_json)))
    });

    group.bench_function("complex", |b| {
        b.iter(|| parse_json_message(black_box(complex_json)))
    });

    group.finish();
}

/// Benchmark frame encoding
fn bench_frame_encoding(c: &mut Criterion) {
    let mut group = c.benchmark_group("websocket_frame");

    let payloads: Vec<(&str, Vec<u8>)> = vec![
        ("tiny", vec![1, 2, 3, 4]),
        ("small", vec![0u8; 100]),
        ("medium_125", vec![0u8; 125]), // Max single-byte length
        ("medium_126", vec![0u8; 126]), // Requires 2-byte length
        ("large", vec![0u8; 1024]),
    ];

    for (name, payload) in payloads.iter() {
        // Server-side (no mask)
        group.bench_with_input(
            BenchmarkId::new("encode_unmasked", name),
            payload,
            |b, payload| b.iter(|| encode_frame(0x01, black_box(payload), false)),
        );

        // Client-side (with mask)
        group.bench_with_input(
            BenchmarkId::new("encode_masked", name),
            payload,
            |b, payload| b.iter(|| encode_frame(0x01, black_box(payload), true)),
        );
    }

    group.finish();
}

/// Benchmark broadcast scenario (sending to multiple clients)
fn bench_broadcast(c: &mut Criterion) {
    let mut group = c.benchmark_group("websocket_broadcast");

    let message = "Broadcast message to all connected clients";

    for client_count in [10, 100, 1000].iter() {
        group.bench_with_input(
            BenchmarkId::new("prepare_messages", client_count),
            client_count,
            |b, &count| {
                b.iter(|| {
                    // Simulate preparing messages for N clients
                    let mut messages = Vec::with_capacity(count);
                    for _ in 0..count {
                        messages.push(black_box(message).to_string());
                    }
                    messages
                })
            },
        );
    }

    group.finish();
}

/// Benchmark connection management (HashMap-based room pattern)
fn bench_connection_management(c: &mut Criterion) {
    let mut group = c.benchmark_group("websocket_rooms");

    // Simulate room-based connection management
    group.bench_function("join_room", |b| {
        let mut rooms: HashMap<String, Vec<u64>> = HashMap::new();
        let mut client_id = 0u64;

        b.iter(|| {
            client_id += 1;
            let room = black_box("general".to_string());
            rooms.entry(room).or_default().push(client_id);
        })
    });

    group.bench_function("leave_room", |b| {
        let mut rooms: HashMap<String, Vec<u64>> = HashMap::new();
        rooms.insert("general".to_string(), (0..1000).collect());

        b.iter(|| {
            let room = rooms.get_mut(black_box("general")).unwrap();
            let client_id = black_box(500u64);
            if let Some(pos) = room.iter().position(|&id| id == client_id) {
                room.swap_remove(pos);
            }
        })
    });

    group.bench_function("list_room_members", |b| {
        let mut rooms: HashMap<String, Vec<u64>> = HashMap::new();
        rooms.insert("general".to_string(), (0..100).collect());

        b.iter(|| rooms.get(black_box("general")).map(|members| members.len()))
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_text_message,
    bench_binary_message,
    bench_json_message,
    bench_frame_encoding,
    bench_broadcast,
    bench_connection_management,
);

criterion_main!(benches);
