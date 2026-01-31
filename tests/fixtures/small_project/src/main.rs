use small_project::Config;

fn main() {
    let config = Config::default();
    run_app(config);
}

fn run_app(config: Config) {
    println!("Running with config: {:?}", config);
}
