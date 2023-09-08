use worker2::WordCloudAgent;
use yew_agent::Threaded;

fn main() {
    console_error_panic_hook::set_once();
    wasm_logger::init(wasm_logger::Config::new(log::Level::Info).module_prefix("worker"));
    WordCloudAgent::register();
}
