pub mod bridge;
mod netlink;

use rand::{self, Rng};
use rand::distributions::Alphanumeric;

pub fn generate_ifname(len: usize) -> String {
    rand::thread_rng()
        .sample_iter(&Alphanumeric)
        .take(len)
        .collect::<String>()
}
