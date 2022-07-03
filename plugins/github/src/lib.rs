#[no_mangle]
pub fn run(args: Option<Vec<&str>>) {
    println!("Running GitHub");
    if let Some(args) = args {
        println!("{} args: {:?}", args.len(), args);
    }
}
