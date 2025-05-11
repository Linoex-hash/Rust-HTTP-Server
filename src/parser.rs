use clap::Parser;

#[derive(Parser, Debug)]
pub struct HTTPArgs {
    /// IP Address. Enter in the format of "1.2.3.4". Default is "127.0.0.1" (Localhost)
    #[arg(long, short)]
    pub ip_addr: Option<String>,

    /// Port Number. Enter in the format of "1234". Default is port 8080.
    #[arg(long, short)]
    pub port: Option<u16>,
}
