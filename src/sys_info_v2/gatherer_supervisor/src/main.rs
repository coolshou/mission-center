mod gatherer;

fn main() {
    let gatherer = gatherer::Gatherer::new();
    dbg!(gatherer.enumerate_gpus());
    dbg!(gatherer.gpu_static_info("0000:01:00.0"));
    dbg!(gatherer.gpu_dynamic_info("0000:01:00.0"));
}
