//! Micro-benchmark harness (no extra dependencies): validation latency and heap allocations per parse.
//!
//! Run with: `cargo run --release --example bench`

use rust_zod::{z, Value};
use std::alloc::{GlobalAlloc, Layout, System};
use std::collections::BTreeMap;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Instant;

struct Counting;
static ALLOCS: AtomicUsize = AtomicUsize::new(0);

unsafe impl GlobalAlloc for Counting {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        ALLOCS.fetch_add(1, Ordering::Relaxed);
        System.alloc(layout)
    }
    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        System.dealloc(ptr, layout)
    }
}

#[global_allocator]
static GLOBAL: Counting = Counting;

fn obj(pairs: Vec<(&str, Value)>) -> Value {
    let mut m = BTreeMap::new();
    for (k, v) in pairs {
        m.insert(k.to_string(), v);
    }
    Value::from(m)
}

fn bench(name: &str, iterations: usize, mut f: impl FnMut()) {
    f(); // warm-up
    let before = ALLOCS.load(Ordering::Relaxed);
    let start = Instant::now();
    for _ in 0..iterations {
        f();
    }
    let elapsed = start.elapsed();
    let allocs = ALLOCS.load(Ordering::Relaxed) - before;
    println!(
        "{:<34} {:>9.1} ns/op   {:>6.2} allocs/op",
        name,
        elapsed.as_nanos() as f64 / iterations as f64,
        allocs as f64 / iterations as f64
    );
}

fn main() {
    const N: usize = 200_000;

    let string_schema = z::string().min(2).max(20);
    let string_input = Value::from("alice");
    bench("string.min.max (identity)", N, || {
        std::hint::black_box(string_schema.parse_value(&string_input).unwrap());
    });

    let int_schema = z::int32().gte(0).lte(150);
    let int_input = Value::from(42);
    bench("int32.gte.lte -> i32 (fast path)", N, || {
        std::hint::black_box(int_schema.parse(&int_input).unwrap());
    });

    let email = z::email();
    let email_input = Value::from("alice@example.com");
    bench("email", N, || {
        std::hint::black_box(email.parse_value(&email_input).unwrap());
    });

    let user = z::object(|o| {
        o.field("id", z::uuid())
            .field("name", z::string().min(2))
            .field("age", z::int32().gte(18))
            .field("address", z::object(|a| a.field("street", z::string()).field("zip", z::string())))
    });
    let user_input = obj(vec![
        ("id", Value::from("123e4567-e89b-12d3-a456-426614174000")),
        ("name", Value::from("Alice")),
        ("age", Value::from(30)),
        (
            "address",
            obj(vec![("street", Value::from("Main St")), ("zip", Value::from("12345"))]),
        ),
    ]);
    bench("object (nested, 4 + 2 fields)", N, || {
        std::hint::black_box(user.parse_value(&user_input).unwrap());
    });

    let invalid = obj(vec![("id", Value::from("x")), ("name", Value::from("A")), ("age", Value::from(1))]);
    bench("object (invalid, issue path)", N / 4, || {
        std::hint::black_box(user.parse_value(&invalid).unwrap_err());
    });

    let list = z::array(user.clone());
    let list_input = Value::from(vec![user_input.clone(); 10]);
    bench("array of 10 objects", N / 10, || {
        std::hint::black_box(list.parse_value(&list_input).unwrap());
    });
}
