use gloo_worker::PublicWorker;
use worker2::WordCloudAgent;

fn main() {
    console_error_panic_hook::set_once();
    wasm_logger::init(wasm_logger::Config::new(log::Level::Info).module_prefix("worker"));
    WordCloudAgent::register();
}
