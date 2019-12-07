use nctrl::{
    camera::{camera, set_camera, Camera, SharedCamera},
    fuseable_fs::FuseableFS,
    serde_util::FILE_OPENER,
};
use std::{ffi::OsStr, io::Read, path::PathBuf};
use structopt::StructOpt;
use log::{info};
use env_logger::{Builder, Env};

/// Basic daemon for controlling the various components of a camera
#[derive(StructOpt, Debug)]
#[structopt(name = "ctrl")]
struct Opt {
    /// Config file describing the camera components and their functionality
    #[structopt(name = "FILE")]
    file: String,
    /// Set all communication channels to mock mode to prevent them from
    /// actually doing anything
    #[structopt(short = "m", long = "mock")]
    mock: bool,
    /// Mountpoint of the fuse config filesystem
    #[structopt(short = "d", long = "mountpoint", default_value = ".propfs")]
    mountpoint: String,
}

fn main() {
    Builder::from_env(Env::default().default_filter_or("info"))
        .format_indent(Some(4))
        .format_timestamp(None)
        .init();

    let opt = Opt::from_args();

    let mut f = FILE_OPENER.open(&opt.file).unwrap();
    FILE_OPENER.set_path(PathBuf::from(opt.file));

    let mut contents = String::new();
    f.read_to_string(&mut contents).expect("something went wrong reading the file");

    info!("parsing yml file");
    let mut sensor: Camera = serde_yaml::from_str(&contents).unwrap();

    info!("setting mocked mode to {}", opt.mock);
    sensor.mocked(opt.mock);

    set_camera(sensor);

    let options = ["-o", "allow_other", "-o", "rw", "-o", "fsname=propfs", "-o", "auto_unmount"]
        .iter()
        .map(|o| o.as_ref())
        .collect::<Vec<&OsStr>>();

    info!("successfully initialized");

    fuse::mount(FuseableFS::new(&mut SharedCamera { camera: camera() }), opt.mountpoint, &options)
        .unwrap();
}
