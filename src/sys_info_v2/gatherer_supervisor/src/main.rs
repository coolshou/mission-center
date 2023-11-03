mod gatherer;

fn main() {
    let now = std::time::Instant::now();

    let gatherer = gatherer::Gatherer::new();
    dbg!(gatherer.cpu_static_info());
    dbg!(gatherer.cpu_dynamic_info());
    dbg!(gatherer.enumerate_gpus());
    dbg!(gatherer.gpu_static_info("0000:01:00.0"));
    dbg!(gatherer.gpu_dynamic_info("0000:01:00.0"));
    dbg!(gatherer.processes());

    dbg!(now.elapsed());
}
