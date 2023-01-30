use frontend::AppRoot;

fn main() {
    console_error_panic_hook::set_once();
    wasm_logger::init(wasm_logger::Config::new(log::Level::Info));
    yew::start_app::<AppRoot>();
}
