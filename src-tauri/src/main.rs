// Windows のリリースビルドで余分なコンソールウィンドウを出さない。
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
  expert_base_lib::run();
}
