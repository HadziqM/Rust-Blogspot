use setup::Setup;

pub mod model;
pub mod routes;
pub mod setup;

#[tokio::main]
async fn main() {
    Setup::new(routes::reg()).initialize().await
}
