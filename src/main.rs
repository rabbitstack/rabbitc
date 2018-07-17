// Copyright 2018 by Nedim Sabic (RabbitStack)
// http://rabbitstack.github.io
//
// Licensed under the Apache License, Version 2.0 (the "License"); you may
// not use this file except in compliance with the License. You may obtain
// a copy of the License at
//
// http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS, WITHOUT
// WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied. See the
// License for the specific language governing permissions and limitations
// under the License.

extern crate clap;
#[macro_use]
extern crate log;
extern crate nix;
extern crate pnet_macros_support;
extern crate pnetlink;
extern crate pretty_env_logger;

extern crate ipaddress;
extern crate num;
extern crate rand;

use clap::{App, Arg};

use nix::sched::*;
use nix::{Error, Result};
use nix::sched::clone;
use nix::unistd::{chdir, execve, mkdir, pivot_root, sethostname};
use nix::mount::*;
use std::ffi::CString;
use std::path::Path;
use std::time;
use std::thread;

use nix::sys::stat;
use nix::sys::wait::waitpid;

mod net;

use net::bridge;

fn main() {
    pretty_env_logger::init();
    let matches = App::new("rabbitc")
        .version("0.1.0")
        .about("Micro container runtime")
        .author("Nedim Šabić")
        .arg(
            Arg::with_name("rootfs")
                .required(true)
                .short("r")
                .long("rootfs")
                .multiple(false)
                .takes_value(true)
                .help("Root file system path for the container"),
        )
        .arg(
            Arg::with_name("hostname")
                .short("h")
                .long("hostname")
                .multiple(false)
                .default_value("rabbitc")
                .help("Container host name"),
        )
        .arg(
            Arg::with_name("cmd")
                .short("c")
                .long("cmd")
                .multiple(false)
                .default_value("/bin/sh")
                .help("Command that is run inside container"),
        )
        .arg(
            Arg::with_name("network-name")
                .short("n")
                .long("network-name")
                .multiple(false)
                .default_value("rabbitc0")
                .help("The name of the bridge device where containers are connected"),
        )
        .arg(
            Arg::with_name("network-ip")
                .short("i")
                .long("network-ip")
                .multiple(false)
                .default_value("172.19.0.1/16")
                .help("The default IP address for the bridge device in CIDR notation"),
        )
        .arg(
            Arg::with_name("container-ip")
                .short("t")
                .long("container-ip")
                .multiple(false)
                .default_value("172.19.0.2/16")
                .help("The default IP address for container in CIDR notation"),
        )
        .get_matches();

    let net_name = matches.value_of("network-name").unwrap();
    if let Err(e) = bridge::init(net_name, matches.value_of("network-ip").unwrap()) {
        warn!("unable to initialize bridge network {}", e);
    }

    let rootfs = matches.value_of("rootfs").unwrap();
    if !Path::new(&rootfs).exists() {
        error!("rootfs {} doesn't exist", rootfs);
        std::process::exit(1)
    }
    let hostname = matches.value_of("hostname").unwrap();
    let container_ip = matches.value_of("container-ip").unwrap();
    let cmd = matches.value_of("cmd").unwrap();

    if let Err(e) = runc(rootfs, hostname, container_ip, net_name, cmd) {
        error!("unable to run container: {}", e);
        std::process::exit(1)
    }
}

/// Creates a new process with its own view of system resources segregated through namespace facility.
/// Also ensures to prevent mount namespace propagation which is mandatory for a successful outcome
/// of the `pivot_root` syscall. Setups a network adapter for container and connects it to in kernel
/// bridge device. Then waits for the container process to finish.
fn runc(rootfs: &str, hostname: &str, container_ip: &str, net_name: &str, cmd: &str) -> Result<()> {
    let clone_flags = CloneFlags::CLONE_NEWUTS | CloneFlags::CLONE_NEWPID | CloneFlags::CLONE_NEWNS
        | CloneFlags::CLONE_NEWIPC | CloneFlags::CLONE_NEWNET;

    // set root mount namespace propagation to private.
    // This prevents leaking mounts in the container to
    // the host
    mount(
        None::<&str>,
        "/",
        None::<&str>,
        MsFlags::MS_REC | MsFlags::MS_PRIVATE,
        None::<&str>,
    )?;

    let peer_iface = net::generate_ifname(7);
    bridge::create_veth(&peer_iface, net_name).map_err(|_| nix::Error::UnsupportedOperation)?;

    let stack = &mut [0; 1024 * 1024];
    let cb = Box::new(|| {
        if let Err(e) = init_container(rootfs, hostname, cmd, &peer_iface, container_ip) {
            error!("unable to initialize container: {}", e);
            -1
        } else {
            0
        }
    });

    let child = clone(cb, stack, clone_flags, None)?;

    // move peer interface to container network namespace
    bridge::join(&peer_iface, child.to_string().parse::<u32>().unwrap())
        .map_err(|_| nix::Error::UnsupportedOperation)?;

    // give child process a chance to boot
    thread::sleep(time::Duration::from_millis(300));

    // wait for child process
    waitpid(child, None)?;

    Ok(())
}

/// Does initial container's initialization tasks such as provisioining a new
/// rootfs or setting up network interfaces.
fn init_container(
    rootfs: &str,
    hostname: &str,
    cmd: &str,
    peer: &str,
    container_ip: &str,
) -> Result<()> {
    init_rootfs(rootfs)?;
    sethostname(hostname)?;
    bridge::setup_peer(peer, container_ip).map_err(|_| nix::Error::UnsupportedOperation)?;

    do_exec(cmd)?;

    Ok(())
}

/// Initializes container root file system. It performs bind mount of the root fs
/// prior to giving container a new view of the mount tables. After process's root
/// file system is swapped successfully, this function mounts additional file systems
/// in the container's mount namespace.
fn init_rootfs(rootfs: &str) -> Result<()> {
    mount(
        Some(rootfs),
        rootfs,
        None::<&str>,
        MsFlags::MS_BIND | MsFlags::MS_REC,
        None::<&str>,
    )?;

    let prev_rootfs = Path::new(rootfs).join(".oldrootfs");
    std::fs::remove_dir_all(&prev_rootfs).map_err(|_| Error::InvalidPath)?;
    mkdir(
        &prev_rootfs,
        stat::Mode::S_IRWXU | stat::Mode::S_IRWXG | stat::Mode::S_IRWXO,
    )?;

    pivot_root(rootfs, &prev_rootfs)?;
    chdir("/")?;

    umount2("/.oldrootfs", MntFlags::MNT_DETACH)?;

    // remount procfs system in the container mount namespace
    // so it starts with a brand new proc file system
    mount(
        Some("proc"),
        "/proc",
        Some("proc"),
        MsFlags::MS_NOEXEC | MsFlags::MS_NOSUID | MsFlags::MS_NODEV | MsFlags::MS_RELATIME,
        None::<&str>,
    )?;

    // mount tmpfs on /dev and create some devices
    mount(
        Some("tmpfs"),
        "/dev",
        Some("tmpfs"),
        MsFlags::MS_NOSUID | MsFlags::MS_RELATIME,
        None::<&str>,
    )?;
    Ok(())
}

/// Attempts to load the provided binary image into the address space of the
/// container process.
fn do_exec(cmd: &str) -> Result<()> {
    let args = &[Path::new(cmd).file_stem().unwrap().to_str().unwrap()];
    let envs = &["PATH=/bin:/sbin:/usr/bin:/usr/sbin"];
    let p = CString::new(cmd).unwrap();

    let a: Vec<CString> = args.iter()
        .map(|s| CString::new(s.to_string()).unwrap_or_default())
        .collect();
    let e: Vec<CString> = envs.iter()
        .map(|s| CString::new(s.to_string()).unwrap_or_default())
        .collect();

    execve(&p, &a, &e)?;
    Ok(())
}
