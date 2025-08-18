mod blog_entry;
mod css;
mod generator;
mod git_repo;
mod markdown;
mod pages;

use std::path::PathBuf;

use anyhow::Context as _;
use chrono::TimeZone as _;

const FETCH_DUR: std::time::Duration = std::time::Duration::from_secs(5 * 60);

fn main() {
    let config = Config {
        source_dir: "/source".into(),
        output_dir: "/output".into(),
        static_dir: "/static".into(),
        remote_url: "https://github.com/uimataso/my-site-content".to_string(),
        content: ContentConfig {
            site_url: "https://uimataso.com".to_string(),
            site_name: "uima's site".to_string(),
            author: "uima".to_string(),
            author_email: "me@uimataso.com".to_string(),
            commit_base_url: "https://github.com/uimataso/my-site-content/commit".to_string(),
            footer_links: vec![
                (
                    "https://github.com/uimataso/my-site".to_string(),
                    "code of this site".to_string(),
                ),
                (
                    "https://github.com/uimataso/my-site-content".to_string(),
                    "content of this site".to_string(),
                ),
                ("/blog/rss.xml".to_string(), "rss".to_string()),
                (
                    "mailto:me@uimataso.com".to_string(),
                    "me@uimataso.com".to_string(),
                ),
            ],
        },
    };

    // watch_local(config);
    fetch_and_build(config);
}

#[derive(Clone)]
struct Config {
    source_dir: PathBuf,
    output_dir: PathBuf,
    static_dir: PathBuf,
    remote_url: String,
    content: ContentConfig,
}

#[derive(Clone)]
struct ContentConfig {
    site_url: String,
    site_name: String,
    author: String,
    author_email: String,
    commit_base_url: String,
    footer_links: Vec<(String, String)>,
}

#[allow(clippy::needless_pass_by_value)]
fn fetch_and_build(config: Config) {
    let git_repo = git_repo::GitRepo::new(&config.source_dir, &config.remote_url)
        .expect("cannot open the git repo");

    build_once(config.clone()).expect("failed to build the site at the start");

    loop {
        std::thread::sleep(FETCH_DUR);

        let cur_commit = git_repo
            .current_commit()
            .expect("failed to get current commit");
        let fet_commit = git_repo.fetch().expect("failed to fetch the commit");

        if cur_commit.id() == fet_commit.id() {
            println!("nothing new");
            continue;
        }

        println!("new commit found, rebuild the site");

        let mut cur_ref = git_repo.current_ref().expect("failed to get current ref");
        git_repo
            .fast_forward(&mut cur_ref, &fet_commit)
            .expect("failed to fast forward");

        let res = build_once(config.clone());
        if let Err(error) = res {
            println!("failed to build the site: {:?}", error);
        }
    }
}

fn watch_local(config: Config) {
    build_once(config.clone()).expect("failed to build the site at the start");

    println!("watching for changes: {}", config.source_dir.display());

    let mut watching = hotwatch::blocking::Hotwatch::new().expect("hotwatch failed to initialize!");

    watching
        .watch(config.static_dir.clone(), move |_| {
            println!("rebuilding site");
            build_once(config.clone()).expect("failed to build the site at the start");
            hotwatch::blocking::Flow::Continue
        })
        .expect("failed to watch content folder!");

    watching.run();
}

fn build_once(config: Config) -> anyhow::Result<()> {
    generator::Generator::new(config)
        .context("cannot create the generator")?
        .build()
        .context("failed to build the site")?;
    Ok(())
}

trait PathExt {
    fn extension_is(&self, ext: Option<&str>) -> bool;
}

impl PathExt for std::path::Path {
    fn extension_is(&self, ext: Option<&str>) -> bool {
        self.extension().and_then(|x| x.to_str()) == ext
    }
}

fn to_datetime(time: git2::Time) -> chrono::DateTime<chrono::FixedOffset> {
    let offset_seconds = time.offset_minutes() * 60;

    let offset = chrono::FixedOffset::east_opt(offset_seconds)
        .unwrap_or(chrono::FixedOffset::east_opt(0).unwrap());

    offset
        .timestamp_opt(time.seconds(), 0)
        .single()
        .expect("invalid timestamp")
}
