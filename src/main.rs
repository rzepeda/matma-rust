fn main() {
    if let Err(error) = matma::run() {
        eprintln!("MATMA error: {error:#}");
        std::process::exit(1);
    }
}
