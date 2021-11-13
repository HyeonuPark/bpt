use criterion::{black_box, criterion_group, criterion_main, Criterion};

#[rustfmt::skip]
mod input;

#[global_allocator]
static ALLOC: jemallocator::Jemalloc = jemallocator::Jemalloc;

fn benchmark_input(criterion: &mut Criterion, input_name: &str, input: &[u32]) {
    macro_rules! each_maps {
        ($($mapvar:ident, $name:expr, $init:expr),*) => {
            {
                let mut bench = criterion.benchmark_group(&format!("insert-{}", input_name));
                $(
                    bench.bench_function($name, |bench| {
                        bench.iter(|| {
                            let mut map = $init;
                            for &n in input {
                                let n = black_box(n);
                                black_box(map.insert(n, n));
                            }
                        })
                    });
                )*
            }
            {
                let mut bench = criterion.benchmark_group(&format!("insert-delete-{}", input_name));
                $(
                    bench.bench_function($name, |bench| {
                        bench.iter(|| {
                            let mut map = $init;
                            for &n in input {
                                let n = black_box(n);
                                black_box(map.insert(n, n));
                            }
                        })
                    });
                )*
            }
            {
                $(
                    let mut $mapvar = $init;
                )*
                for &n in input {
                    $(
                        $mapvar.insert(n, n);
                    )*
                }
                let mut bench = criterion.benchmark_group(&format!("get-{}", input_name));
                $(
                    bench.bench_function($name, |bench| {
                        bench.iter(|| {
                            for &n in input {
                                black_box($mapvar.get(&black_box(n)));
                                black_box($mapvar.get(&black_box(n).wrapping_add(1)));
                            }
                        })
                    });
                )*
            }
        }
    }
    macro_rules! each_caps {
        ($($CAP:literal)*) => {
            paste::paste! {
                each_maps!(
                    stdmap, "std", std::collections::BTreeMap::new()
                    $(
                        , [<bpt_ $CAP>], &format!("bpt_{}", $CAP), bpt::BTreeMap::<u32, u32, $CAP>::new()
                    )*
                );
            }
        }
    }

    each_caps!(15 17 19 21 23 25 27 29 31);
}

fn criterion_benchmark(criterion: &mut Criterion) {
    benchmark_input(criterion, "small", &input::SHORT);
    benchmark_input(criterion, "large", &input::LONG);
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
