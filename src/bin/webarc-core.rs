#[tokio::main]
async fn main() {
    let sum = webarc::add(4, 2);
    println!("core: {sum}");
}
