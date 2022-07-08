use std::env;

use gelato_sdk::*;

#[tokio::main]
async fn main() -> Result<(), reqwest::Error> {
    let gelato = GelatoClient::default();

    let id = &env::args().collect::<Vec<_>>()[1];
    let task_status = gelato.get_task_status(id.parse().unwrap()).await.unwrap();
    println!("Task status: {:?}", task_status);

    Ok(())
}
