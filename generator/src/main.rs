use my_site_generator::build;

fn main() -> anyhow::Result<()> {
    env_logger::builder()
        .filter_level(log::LevelFilter::Info)
        .init();

    let res = std::fs::remove_dir_all("/tmp/test");
    println!("res: {:?}", res);

    build("/home/uima/src/my-site-content/", "/tmp/test")?;

    Ok(())
}
