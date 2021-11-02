use criterion::{black_box, criterion_group, criterion_main, Criterion};

#[global_allocator]
static ALLOC: jemallocator::Jemalloc = jemallocator::Jemalloc;

fn criterion_benchmark(criterion: &mut Criterion) {
    let inputs: Vec<i32> = std::iter::repeat_with(rand::random).take(100).collect();

    let mut bench = criterion.benchmark_group("insert-small");
    bench.bench_function("std", |bench| {
        bench.iter(|| {
            let mut map = std::collections::BTreeMap::new();
            for &n in &inputs {
                let n = black_box(n);
                black_box(map.insert(n, n));
            }
        })
    });
    bench.bench_function("custom", |bench| {
        bench.iter(|| {
            let mut map = bpt::BTreeMap::new();
            for &n in &inputs {
                let n = black_box(n);
                black_box(map.insert(n, n));
            }
        })
    });
    drop(bench);

    let mut bench = criterion.benchmark_group("insert-delete-small");
    bench.bench_function("std", |bench| {
        bench.iter(|| {
            let mut map = std::collections::BTreeMap::new();
            for &n in &inputs {
                let n = black_box(n);
                black_box(map.insert(n, n));
            }
            for &n in &inputs {
                black_box(map.remove(&n));
            }
            assert!(map.is_empty());
        })
    });
    bench.bench_function("custom", |bench| {
        bench.iter(|| {
            let mut map = bpt::BTreeMap::new();
            for &n in &inputs {
                let n = black_box(n);
                black_box(map.insert(n, n));
            }
            for &n in &inputs {
                black_box(map.remove(&n));
            }
            assert!(map.is_empty());
        })
    });
    drop(bench);

    let mut stdmap = std::collections::BTreeMap::new();
    let mut custommap = bpt::BTreeMap::new();

    for &n in &inputs {
        stdmap.insert(n, n);
        custommap.insert(n, n);
    }

    let mut bench = criterion.benchmark_group("get-small");
    bench.bench_function("std", |bench| {
        bench.iter(|| {
            for &n in &inputs {
                let n = black_box(n);
                black_box(stdmap.get(&n));
                black_box(stdmap.get(&n.wrapping_add(1)));
            }
        })
    });
    bench.bench_function("custom", |bench| {
        bench.iter(|| {
            for &n in &inputs {
                let n = black_box(n);
                black_box(custommap.get(&n));
                black_box(custommap.get(&n.wrapping_add(1)));
            }
        })
    });
    drop(bench);

    let inputs: Vec<i32> = std::iter::repeat_with(rand::random)
        .take(1024 * 100)
        .collect();

    let mut bench = criterion.benchmark_group("insert");
    bench.bench_function("std", |bench| {
        bench.iter(|| {
            let mut map = std::collections::BTreeMap::new();
            for &n in &inputs {
                let n = black_box(n);
                black_box(map.insert(n, n));
            }
        })
    });
    bench.bench_function("custom", |bench| {
        bench.iter(|| {
            let mut map = bpt::BTreeMap::new();
            for &n in &inputs {
                let n = black_box(n);
                black_box(map.insert(n, n));
            }
        })
    });
    drop(bench);

    let mut bench = criterion.benchmark_group("insert-delete");
    bench.bench_function("std", |bench| {
        bench.iter(|| {
            let mut map = std::collections::BTreeMap::new();
            for &n in &inputs {
                let n = black_box(n);
                black_box(map.insert(n, n));
            }
            for &n in &inputs {
                black_box(map.remove(&n));
            }
            assert!(map.is_empty());
        })
    });
    bench.bench_function("custom", |bench| {
        bench.iter(|| {
            let mut map = bpt::BTreeMap::new();
            for &n in &inputs {
                let n = black_box(n);
                black_box(map.insert(n, n));
            }
            for &n in &inputs {
                black_box(map.remove(&n));
            }
            assert!(map.is_empty());
        })
    });
    drop(bench);

    let mut stdmap = std::collections::BTreeMap::new();
    let mut custommap = bpt::BTreeMap::new();

    for &n in &inputs {
        stdmap.insert(n, n);
        custommap.insert(n, n);
    }

    let mut bench = criterion.benchmark_group("get");
    bench.bench_function("std", |bench| {
        bench.iter(|| {
            for &n in &inputs {
                let n = black_box(n);
                black_box(stdmap.get(&n));
                black_box(stdmap.get(&n.wrapping_add(1)));
            }
        })
    });
    bench.bench_function("custom", |bench| {
        bench.iter(|| {
            for &n in &inputs {
                let n = black_box(n);
                black_box(custommap.get(&n));
                black_box(custommap.get(&n.wrapping_add(1)));
            }
        })
    });
    drop(bench);
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
